//! Collect Chrome download events and expose completed local files without guessing filenames.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, bail};
use chromiumoxide::Browser;
use chromiumoxide::cdp::browser_protocol::browser::{
    CancelDownloadParams, DownloadProgressState, EventDownloadProgress, EventDownloadWillBegin,
    SetDownloadBehaviorBehavior, SetDownloadBehaviorParams,
};
use futures::StreamExt;
use pyo3::prelude::*;
use tokio::sync::watch;

use super::SessionState;
use crate::{diagnostics, runtime};

struct Record {
    guid: String,
    url: String,
    suggested_filename: String,
    path: PathBuf,
    state: DownloadProgressState,
    received_bytes: f64,
    total_bytes: f64,
}

struct Store {
    records: Vec<Arc<Mutex<Record>>>,
    pending: VecDeque<Arc<Mutex<Record>>>,
    stopped: bool,
}

/// Own one event subscription for the session's default browser context.
pub(super) struct Downloads {
    pub directory: PathBuf,
    store: Arc<Mutex<Store>>,
    changed: watch::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl Downloads {
    pub async fn enable(browser: &Browser, directory: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&directory)?;
        let directory = std::fs::canonicalize(directory)?;
        let mut begins = browser.event_listener::<EventDownloadWillBegin>().await?;
        let mut progress = browser.event_listener::<EventDownloadProgress>().await?;
        let store = Arc::new(Mutex::new(Store {
            records: Vec::new(),
            pending: VecDeque::new(),
            stopped: false,
        }));
        let (changed, _) = watch::channel(());
        let state = store.clone();
        let updates = changed.clone();
        let folder = directory.clone();
        let task = tokio::spawn(async move {
            // Separate typed streams can be polled in either order; retain early progress until its begin event.
            let mut early_progress = HashMap::new();
            loop {
                tokio::select! {
                    event = begins.next() => {
                        let Some(event) = event else { break };
                        let mut record = Record { guid: event.guid.clone(), url: event.url.clone(), suggested_filename: event.suggested_filename.clone(), path: folder.join(&event.guid), state: DownloadProgressState::InProgress, received_bytes: 0.0, total_bytes: 0.0 };
                        if let Some(progress) = early_progress.remove(&event.guid) { apply(&mut record, &progress); }
                        let record = Arc::new(Mutex::new(record));
                        let mut store = state.lock().unwrap();
                        store.records.push(record.clone());
                        store.pending.push_back(record);
                        updates.send_replace(());
                        diagnostics::event("download", "started");
                    }
                    event = progress.next() => {
                        let Some(event) = event else { break };
                        let store = state.lock().unwrap();
                        let record = store.records.iter().find(|record| record.lock().unwrap().guid == event.guid);
                        if let Some(record) = record { apply(&mut record.lock().unwrap(), &event); }
                        else { early_progress.insert(event.guid.clone(), (*event).clone()); }
                        updates.send_replace(());
                    }
                }
            }
            state.lock().unwrap().stopped = true;
            updates.send_replace(());
        });
        let downloads = Self {
            directory,
            store,
            changed,
            task,
        };
        Ok(downloads)
    }

    pub async fn configure(&self, browser: &Browser) -> anyhow::Result<()> {
        browser
            .execute(
                SetDownloadBehaviorParams::builder()
                    .behavior(SetDownloadBehaviorBehavior::AllowAndName)
                    .download_path(self.directory.to_string_lossy())
                    .events_enabled(true)
                    .build()
                    .map_err(anyhow::Error::msg)?,
            )
            .await?;
        Ok(())
    }

    pub async fn reset(&self, browser: &Browser) -> anyhow::Result<()> {
        browser
            .execute(
                SetDownloadBehaviorParams::builder()
                    .behavior(SetDownloadBehaviorBehavior::Default)
                    .events_enabled(false)
                    .build()
                    .map_err(anyhow::Error::msg)?,
            )
            .await?;
        self.stop();
        Ok(())
    }

    fn stop(&self) {
        self.task.abort();
        self.store.lock().unwrap().stopped = true;
        self.changed.send_replace(());
    }

    pub fn list(&self, session: Arc<SessionState>) -> Vec<Download> {
        self.store
            .lock()
            .unwrap()
            .records
            .iter()
            .map(|record| Download {
                record: record.clone(),
                store: self.store.clone(),
                changed: self.changed.clone(),
                session: session.clone(),
            })
            .collect()
    }

    pub async fn next(&self, session: Arc<SessionState>) -> anyhow::Result<Download> {
        let mut changes = self.changed.subscribe();
        loop {
            {
                let mut store = self.store.lock().unwrap();
                if let Some(record) = store.pending.front().cloned() {
                    let state = record.lock().unwrap().state.clone();
                    if state != DownloadProgressState::InProgress {
                        store.pending.pop_front();
                        if state == DownloadProgressState::Canceled {
                            bail!("Download was canceled");
                        }
                        return Ok(Download {
                            record,
                            store: self.store.clone(),
                            changed: self.changed.clone(),
                            session,
                        });
                    }
                }
                if store.stopped {
                    bail!("Download collection has closed");
                }
            }
            changes.changed().await?;
        }
    }
}

impl Drop for Downloads {
    fn drop(&mut self) {
        self.stop();
    }
}

fn apply(record: &mut Record, progress: &EventDownloadProgress) {
    record.state = progress.state.clone();
    record.received_bytes = progress.received_bytes;
    record.total_bytes = progress.total_bytes;
    if record.state != DownloadProgressState::InProgress {
        diagnostics::event("download", record.state.as_ref());
    }
}

/// Track one download; completed files remain usable after the browser connection closes.
#[pyclass(frozen, skip_from_py_object, module = "browser_use")]
#[derive(Clone)]
pub(crate) struct Download {
    record: Arc<Mutex<Record>>,
    store: Arc<Mutex<Store>>,
    changed: watch::Sender<()>,
    session: Arc<SessionState>,
}

impl Download {
    async fn completed_path(&self) -> anyhow::Result<PathBuf> {
        let mut changes = self.changed.subscribe();
        loop {
            {
                let record = self.record.lock().unwrap();
                match record.state {
                    DownloadProgressState::Completed => return Ok(record.path.clone()),
                    DownloadProgressState::Canceled => bail!("Download was canceled"),
                    DownloadProgressState::InProgress => {}
                }
            }
            if self.store.lock().unwrap().stopped {
                bail!("Download collection has closed");
            }
            changes.changed().await?;
        }
    }
}

#[pymethods]
impl Download {
    #[getter]
    fn guid(&self) -> String {
        self.record.lock().unwrap().guid.clone()
    }
    #[getter]
    fn url(&self) -> String {
        self.record.lock().unwrap().url.clone()
    }
    #[getter]
    fn suggested_filename(&self) -> String {
        self.record.lock().unwrap().suggested_filename.clone()
    }
    #[getter]
    fn state(&self) -> &'static str {
        match self.record.lock().unwrap().state {
            DownloadProgressState::InProgress => "in_progress",
            DownloadProgressState::Completed => "completed",
            DownloadProgressState::Canceled => "canceled",
        }
    }
    #[getter]
    fn received_bytes(&self) -> f64 {
        self.record.lock().unwrap().received_bytes
    }
    #[getter]
    fn total_bytes(&self) -> f64 {
        self.record.lock().unwrap().total_bytes
    }
    #[getter]
    fn path(&self) -> Option<PathBuf> {
        let record = self.record.lock().unwrap();
        (record.state == DownloadProgressState::Completed).then(|| record.path.clone())
    }

    /// Wait until Chrome reports completion and return the local GUID-named file path.
    #[pyo3(signature = (*, timeout=30.0))]
    fn wait(&self, py: Python<'_>, timeout: f64) -> PyResult<PathBuf> {
        let download = self.clone();
        runtime::run(
            py,
            "download_wait",
            runtime::seconds(timeout)?,
            async move { download.completed_path().await },
        )
    }

    /// Wait for completion and copy the file to an explicit local destination.
    #[pyo3(signature = (path, *, timeout=30.0))]
    fn save_as(&self, py: Python<'_>, path: PathBuf, timeout: f64) -> PyResult<PathBuf> {
        let path = std::path::absolute(path).map_err(runtime::error)?;
        let download = self.clone();
        runtime::run(
            py,
            "download_save",
            runtime::seconds(timeout)?,
            async move {
                let source = download.completed_path().await?;
                tokio::task::spawn_blocking(move || {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(source, &path)?;
                    Ok(path)
                })
                .await?
            },
        )
    }

    /// Cancel this download if it is still in progress.
    #[pyo3(signature = (*, timeout=10.0))]
    fn cancel(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        let timeout = runtime::seconds(timeout)?;
        let record = self.record.lock().unwrap();
        if record.state != DownloadProgressState::InProgress {
            return Ok(());
        }
        let guid = record.guid.clone();
        drop(record);
        let session = self.session.clone();
        runtime::run(py, "download_cancel", timeout, async move {
            session.ensure_open()?;
            let connection = session.connection.lock().await;
            connection
                .as_ref()
                .context("Browser session is closed")?
                .browser
                .execute(CancelDownloadParams::new(guid))
                .await?;
            Ok(())
        })
    }
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Download>()
}
