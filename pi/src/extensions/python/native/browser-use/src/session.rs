//! Own browser connections separately from Chrome processes and Python page handles.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use anyhow::{Context, bail};
use chromiumoxide::cdp::browser_protocol::target::GetTargetsParams;
use chromiumoxide::handler::HandlerConfig;
use chromiumoxide::{Browser, Handler};
use futures::StreamExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    diagnostics, discovery,
    page::{Page, PageResources},
    runtime,
};

mod cookies;
mod downloads;
use downloads::{Download, Downloads};
mod ownership;
mod process;
use process::OwnedBrowser;

fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn handler_config() -> HandlerConfig {
    HandlerConfig {
        viewport: None,
        ignore_https_errors: false,
        ..Default::default()
    }
}

struct Connection {
    /// CDP command sender; process ownership stays with OwnedBrowser.
    browser: Browser,

    /// Continuously receives protocol messages, including between Python cells.
    handler: tokio::task::JoinHandle<()>,

    /// Present only for SDK-launched Chrome; dropped after connection shutdown.
    process: Option<Arc<OwnedBrowser>>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.handler.abort();
    }
}

/// Keep page handles invalidatable when their owning session closes.
pub(crate) struct SessionState {
    /// Taken when closing so page handles can outlive the socket safely.
    connection: AsyncMutex<Option<Connection>>,

    /// Rejects new page operations as soon as shutdown starts.
    pub closed: AtomicBool,

    /// Retained after process cleanup to report the ownership release.
    owned_pid: Option<u32>,

    /// Per-target tasks and input ordering, shared by every Python handle for that page.
    pages: Mutex<HashMap<String, Arc<PageResources>>>,

    /// Download events are enabled only after the caller chooses a destination directory.
    downloads: AsyncMutex<Option<Arc<Downloads>>>,
}

impl SessionState {
    pub(crate) fn page_resources(&self, target: &str) -> Arc<PageResources> {
        self.pages
            .lock()
            .unwrap()
            .entry(target.to_owned())
            .or_insert_with(|| Arc::new(PageResources::new()))
            .clone()
    }

    pub(crate) fn forget_page(&self, target: &str) {
        self.pages.lock().unwrap().remove(target);
    }

    pub(crate) fn ensure_open(&self) -> anyhow::Result<()> {
        if self.closed.load(Ordering::Acquire) {
            bail!("Browser session is closed");
        }
        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        self.closed.store(true, Ordering::Release);
        let pages: Vec<_> = self.pages.lock().unwrap().values().cloned().collect();
        for page in pages {
            page.close().await?;
        }
        self.pages.lock().unwrap().clear();
        let downloads = self.downloads.lock().await.take();
        if let Some(mut connection) = self.connection.lock().await.take() {
            if let Some(downloads) = downloads
                && downloads.reset(&connection.browser).await.is_err()
            {
                diagnostics::event("download_cleanup", "connection_closed_or_failed");
            }
            if let Some(process) = &connection.process {
                // Attached sessions only disconnect after restoring SDK-owned routing/download settings.
                let _ =
                    tokio::time::timeout(Duration::from_secs(2), connection.browser.close()).await;
                let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
                while !process.exited()? && tokio::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                process.terminate()?;
            }
        }
        Ok(())
    }
}

fn sessions() -> &'static Mutex<HashMap<u64, Arc<SessionState>>> {
    static SESSIONS: OnceLock<Mutex<HashMap<u64, Arc<SessionState>>>> = OnceLock::new();
    SESSIONS.get_or_init(Mutex::default)
}

/// Represent a worker-owned connection; close disconnects attached Chrome and exits launched Chrome.
#[pyclass(frozen, module = "browser_use")]
pub(crate) struct Session {
    /// Worker-local identity shared with the parent's ownership record.
    id: u64,

    /// Shared with every page and locator created from this session.
    state: Arc<SessionState>,

    /// Main browser PID when launched or resolved locally; None for endpoint connections.
    #[pyo3(get)]
    pid: Option<u32>,

    /// Describes whether close disconnects or exits the browser.
    #[pyo3(get)]
    mode: &'static str,
}

impl Session {
    fn register(id: u64, state: Arc<SessionState>, pid: Option<u32>) -> Self {
        let mode = if state.owned_pid.is_some() {
            "launched"
        } else {
            "attached"
        };
        sessions().lock().unwrap().insert(id, state.clone());
        diagnostics::event(
            "session_registered",
            &format!("id={id} mode={mode} browser_pid={pid:?}"),
        );
        Self {
            id,
            state,
            pid,
            mode,
        }
    }
}

#[pymethods]
impl Session {
    /// Enumerate live page targets; returned Page objects retain stable target IDs.
    #[pyo3(signature = (*, timeout=10.0))]
    fn tabs(&self, py: Python<'_>, timeout: f64) -> PyResult<Vec<Page>> {
        let state = self.state.clone();
        let pages = runtime::run(py, "list_tabs", runtime::seconds(timeout)?, async move {
            state.ensure_open()?;
            let mut connection = state.connection.lock().await;
            let browser = &mut connection
                .as_mut()
                .context("Browser session is closed")?
                .browser;
            let targets = browser
                .execute(GetTargetsParams::default())
                .await?
                .result
                .target_infos;
            let mut pages = Vec::new();
            for target in targets.into_iter().filter(|target| target.r#type == "page") {
                loop {
                    match browser.get_page(target.target_id.clone()).await {
                        Ok(page) => {
                            pages.push(page);
                            break;
                        }
                        Err(chromiumoxide::error::CdpError::NotFound) => {
                            tokio::time::sleep(Duration::from_millis(25)).await
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            Ok(pages)
        })?;
        Ok(pages
            .into_iter()
            .map(|page| Page::new(self.state.clone(), page))
            .collect())
    }

    /// Open a new page in this browser's default context.
    #[pyo3(signature = (url="about:blank", *, timeout=30.0))]
    fn new_page(&self, py: Python<'_>, url: &str, timeout: f64) -> PyResult<Page> {
        let state = self.state.clone();
        let url = url.to_owned();
        let page = runtime::run(py, "new_page", runtime::seconds(timeout)?, async move {
            state.ensure_open()?;
            let connection = state.connection.lock().await;
            Ok(connection
                .as_ref()
                .context("Browser session is closed")?
                .browser
                .new_page(url)
                .await?)
        })?;
        Ok(Page::new(self.state.clone(), page))
    }

    /// Enable default-context download collection; Chrome writes GUID-named files here.
    #[pyo3(signature = (path, *, timeout=10.0))]
    fn set_download_directory(&self, py: Python<'_>, path: PathBuf, timeout: f64) -> PyResult<()> {
        let directory = std::path::absolute(path).map_err(runtime::error)?;
        let state = self.state.clone();
        runtime::run(
            py,
            "downloads_enable",
            runtime::seconds(timeout)?,
            async move {
                state.ensure_open()?;
                let mut downloads = state.downloads.lock().await;
                let connection = state.connection.lock().await;
                let browser = &connection
                    .as_ref()
                    .context("Browser session is closed")?
                    .browser;
                if let Some(current) = downloads.as_ref() {
                    if std::fs::canonicalize(&directory)? != current.directory {
                        bail!("Reset download collection before changing its directory");
                    }
                } else {
                    *downloads = Some(Arc::new(Downloads::enable(browser, directory).await?));
                }
                downloads.as_ref().unwrap().configure(browser).await
            },
        )
    }

    /// Return every download observed since collection was enabled, including in-progress files.
    #[pyo3(signature = (*, timeout=10.0))]
    fn downloads(&self, py: Python<'_>, timeout: f64) -> PyResult<Vec<Download>> {
        let state = self.state.clone();
        runtime::run(
            py,
            "downloads_list",
            runtime::seconds(timeout)?,
            async move {
                state.ensure_open()?;
                let downloads = state.downloads.lock().await;
                Ok(downloads
                    .as_ref()
                    .context("Call set_download_directory before collecting downloads")?
                    .list(state.clone()))
            },
        )
    }

    /// Wait for the next completed download; a timeout leaves it available for the next call.
    #[pyo3(signature = (*, timeout=30.0))]
    fn wait_for_download(&self, py: Python<'_>, timeout: f64) -> PyResult<Download> {
        let state = self.state.clone();
        runtime::run(
            py,
            "download_next",
            runtime::seconds(timeout)?,
            async move {
                state.ensure_open()?;
                let downloads = state
                    .downloads
                    .lock()
                    .await
                    .as_ref()
                    .cloned()
                    .context("Call set_download_directory before collecting downloads")?;
                downloads.next(state).await
            },
        )
    }

    /// Stop collecting download events and restore Chrome's default download behavior.
    #[pyo3(signature = (*, timeout=10.0))]
    fn reset_downloads(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        let state = self.state.clone();
        runtime::run(
            py,
            "downloads_reset",
            runtime::seconds(timeout)?,
            async move {
                state.ensure_open()?;
                let mut downloads = state.downloads.lock().await;
                let connection = state.connection.lock().await;
                if let Some(current) = downloads.as_ref() {
                    current
                        .reset(
                            &connection
                                .as_ref()
                                .context("Browser session is closed")?
                                .browser,
                        )
                        .await?;
                }
                *downloads = None;
                Ok(())
            },
        )
    }

    /// Read all cookies in the browser's default context, including HttpOnly cookies.
    #[pyo3(signature = (*, timeout=10.0))]
    fn cookies(&self, py: Python<'_>, timeout: f64) -> PyResult<Py<PyAny>> {
        let state = self.state.clone();
        let records = runtime::run(py, "cookies_read", runtime::seconds(timeout)?, async move {
            state.ensure_open()?;
            let connection = state.connection.lock().await;
            Ok(cookies::records(
                connection
                    .as_ref()
                    .context("Browser session is closed")?
                    .browser
                    .get_cookies()
                    .await?,
            ))
        })?;
        Ok(pythonize::pythonize(py, &records)?.unbind())
    }

    /// Add or replace default-context cookies from records using Python snake_case fields.
    #[pyo3(signature = (cookies, *, timeout=10.0))]
    fn add_cookies(
        &self,
        py: Python<'_>,
        cookies: &Bound<'_, PyAny>,
        timeout: f64,
    ) -> PyResult<()> {
        let cookies = cookies::parse(cookies)?;
        let state = self.state.clone();
        runtime::run(
            py,
            "cookies_write",
            runtime::seconds(timeout)?,
            async move {
                state.ensure_open()?;
                if cookies.is_empty() {
                    return Ok(());
                }
                let connection = state.connection.lock().await;
                connection
                    .as_ref()
                    .context("Browser session is closed")?
                    .browser
                    .set_cookies(cookies)
                    .await?;
                Ok(())
            },
        )
    }

    /// Remove every cookie in the browser's default context.
    #[pyo3(signature = (*, timeout=10.0))]
    fn clear_cookies(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        let state = self.state.clone();
        runtime::run(
            py,
            "cookies_clear",
            runtime::seconds(timeout)?,
            async move {
                state.ensure_open()?;
                let connection = state.connection.lock().await;
                connection
                    .as_ref()
                    .context("Browser session is closed")?
                    .browser
                    .clear_cookies()
                    .await?;
                Ok(())
            },
        )
    }

    /// Release this connection; attached user windows and tabs remain open.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        ownership::check_thread()?;
        if !sessions().lock().unwrap().contains_key(&self.id) {
            return Ok(());
        }
        let state = self.state.clone();
        let result = runtime::run(py, "session_close", Duration::from_secs(5), async move {
            state.close().await
        });
        if result.is_ok() {
            if let Some(pid) = self.state.owned_pid {
                ownership::notify(py, "closed", self.id, pid)?;
            }
            sessions().lock().unwrap().remove(&self.id);
            diagnostics::event(
                "session_released",
                &format!("id={} mode={}", self.id, self.mode),
            );
        }
        result
    }

    #[getter]
    fn closed(&self) -> bool {
        self.state.closed.load(Ordering::Acquire)
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &self,
        py: Python<'_>,
        _kind: &Bound<'_, PyAny>,
        _value: &Bound<'_, PyAny>,
        _traceback: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.close(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Session(id={}, mode={:?}, pid={:?}, closed={})",
            self.id,
            self.mode,
            self.pid,
            self.closed()
        )
    }
}

async fn start_handler(
    browser: Browser,
    mut handler: Handler,
    process: Option<Arc<OwnedBrowser>>,
) -> anyhow::Result<Arc<SessionState>> {
    let handler = tokio::spawn(async move {
        while let Some(result) = handler.next().await {
            if result.is_err() {
                diagnostics::event("cdp_handler", "connection_failed");
                break;
            }
        }
    });
    // Keep the handler owned during initialization so cancellation cannot leave
    // a detached task controlling a browser with no Python session.
    let owned_pid = process.as_ref().map(|process| process.pid);
    let mut connection = Connection {
        browser,
        handler,
        process,
    };
    connection.browser.fetch_targets().await?;
    Ok(Arc::new(SessionState {
        connection: AsyncMutex::new(Some(connection)),
        closed: AtomicBool::new(false),
        owned_pid,
        pages: Mutex::new(HashMap::new()),
        downloads: AsyncMutex::new(None),
    }))
}

/// Connect using a desktop/window PID or an explicit CDP endpoint; timeout is in seconds.
#[pyfunction]
#[pyo3(signature = (*, pid=None, endpoint=None, timeout=30.0))]
fn connect(
    py: Python<'_>,
    pid: Option<u32>,
    endpoint: Option<String>,
    timeout: f64,
) -> PyResult<Session> {
    let timeout = runtime::seconds(timeout)?;
    if pid.is_some() == endpoint.is_some() {
        return Err(PyValueError::new_err(
            "Supply exactly one of pid or endpoint",
        ));
    }
    let (state, pid) = runtime::run(py, "connect", timeout, async move {
        let (pid, endpoint) = if let Some(pid) = pid {
            let (instance, endpoint) =
                tokio::task::spawn_blocking(move || discovery::resolve(pid)).await??;
            (Some(instance.pid), endpoint)
        } else {
            (None, endpoint.unwrap())
        };
        let (browser, handler) = Browser::connect_with_config(endpoint, handler_config()).await?;
        Ok((start_handler(browser, handler, None).await?, pid))
    })?;
    Ok(Session::register(next_id(), state, pid))
}

/// Launch installed Chrome/Chromium with a separate profile; None uses a temporary profile.
#[pyfunction]
#[pyo3(signature = (*, executable_path=None, user_data_dir=None, headless=false, timeout=30.0))]
fn launch(
    py: Python<'_>,
    executable_path: Option<PathBuf>,
    user_data_dir: Option<PathBuf>,
    headless: bool,
    timeout: f64,
) -> PyResult<Session> {
    let timeout = runtime::seconds(timeout)?;
    ownership::check_thread()?;
    let id = next_id();
    let process = Arc::new(
        py.detach(|| OwnedBrowser::spawn(executable_path, user_data_dir, headless))
            .map_err(runtime::error)?,
    );
    let pid = process.pid;
    ownership::notify(py, "opened", id, pid)?;
    let pending = process.clone();
    let result = runtime::run(py, "launch", timeout, async move {
        let endpoint = pending.endpoint().await?;
        let (browser, handler) = Browser::connect_with_config(endpoint, handler_config()).await?;
        start_handler(browser, handler, Some(pending)).await
    });
    match result {
        Ok(state) => Ok(Session::register(id, state, Some(pid))),
        Err(error) => {
            py.detach(|| process.terminate()).map_err(runtime::error)?;
            ownership::notify(py, "closed", id, pid)?;
            Err(error)
        }
    }
}

/// Close all worker-owned connections, preserving browsers that were only attached.
#[pyfunction]
fn close_all(py: Python<'_>) -> PyResult<()> {
    ownership::check_thread()?;
    let states: Vec<_> = sessions()
        .lock()
        .unwrap()
        .iter()
        .map(|(id, state)| (*id, state.clone()))
        .collect();
    let mut failure = None;
    for (id, state) in states {
        let pid = state.owned_pid;
        let result = runtime::run(py, "session_close", Duration::from_secs(5), async move {
            state.close().await
        });
        match result {
            Ok(()) => {
                if let Some(pid) = pid {
                    ownership::notify(py, "closed", id, pid)?;
                }
                sessions().lock().unwrap().remove(&id);
            }
            Err(error) => failure = Some(error),
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    ownership::register(module)?;
    downloads::register(module)?;
    module.add_class::<Session>()?;
    module.add_function(wrap_pyfunction!(connect, module)?)?;
    module.add_function(wrap_pyfunction!(launch, module)?)?;
    module.add_function(wrap_pyfunction!(close_all, module)?)?;
    Ok(())
}
