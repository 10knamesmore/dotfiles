//! Expose stable page handles and return browser data directly to Python.

use std::future::Future;
use std::sync::Arc;

use chromiumoxide::cdp::browser_protocol::{
    accessibility::GetFullAxTreeParams, browser::GetWindowForTargetParams,
    page::CaptureScreenshotFormat,
};
use chromiumoxide::page::ScreenshotParams;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde_json::Value;

use crate::{runtime, session::SessionState};

mod keyboard;
mod locator;
mod query;
mod routing;
use keyboard::Keyboard;
pub(crate) use locator::Locator;

/// Share input sequencing and background work across handles for the same target.
pub(crate) struct PageResources {
    keyboard: Arc<tokio::sync::Mutex<()>>,
    routing: tokio::sync::Mutex<Option<routing::Routing>>,
}

impl PageResources {
    pub(crate) fn new() -> Self {
        Self {
            keyboard: Arc::new(tokio::sync::Mutex::new(())),
            routing: tokio::sync::Mutex::new(None),
        }
    }

    pub(crate) async fn close(&self) -> anyhow::Result<()> {
        if let Some(routing) = self.routing.lock().await.take() {
            routing.close().await;
        }
        let _guard = self.keyboard.lock().await;
        Ok(())
    }
}

/// Hold a live CDP page target; navigation preserves this handle, session closure invalidates it.
#[pyclass(frozen, skip_from_py_object, module = "browser_use")]
#[derive(Clone)]
pub(crate) struct Page {
    state: Arc<SessionState>,
    inner: chromiumoxide::Page,
    resources: Arc<PageResources>,
}

impl Page {
    pub(crate) fn new(state: Arc<SessionState>, inner: chromiumoxide::Page) -> Self {
        let resources = state.page_resources(inner.target_id().as_ref());
        Self {
            state,
            inner,
            resources,
        }
    }

    fn run<T: Send + 'static, F: Future<Output = anyhow::Result<T>> + Send + 'static>(
        &self,
        py: Python<'_>,
        name: &'static str,
        timeout: f64,
        operation: impl FnOnce(chromiumoxide::Page) -> F + Send + 'static,
    ) -> PyResult<T> {
        let page = self.inner.clone();
        let state = self.state.clone();
        runtime::run(py, name, runtime::seconds(timeout)?, async move {
            state.ensure_open()?;
            operation(page).await
        })
    }
}

#[pymethods]
impl Page {
    #[getter]
    fn target_id(&self) -> String {
        self.inner.target_id().as_ref().to_owned()
    }

    /// Read the title, URL and Chrome window ID; window_id is not an operating-system window ID.
    #[pyo3(signature = (*, timeout=10.0))]
    fn info(&self, py: Python<'_>, timeout: f64) -> PyResult<Py<PyAny>> {
        let data = self.run(py, "page_info", timeout, |page| async move {
            let window = page.execute(GetWindowForTargetParams::builder().target_id(page.target_id().clone()).build()).await?.result;
            Ok(serde_json::json!({ "target_id": page.target_id().as_ref(), "window_id": window.window_id, "title": page.get_title().await?, "url": page.url().await? }))
        })?;
        Ok(pythonize::pythonize(py, &data)?.unbind())
    }

    /// Navigate and wait for the document's load lifecycle; timeout is in seconds.
    #[pyo3(signature = (url, *, timeout=30.0))]
    fn goto(&self, py: Python<'_>, url: String, timeout: f64) -> PyResult<()> {
        self.run(py, "navigate", timeout, |page| async move {
            page.goto(url).await?;
            Ok(())
        })
    }

    /// Evaluate JavaScript in the main frame, await promises and return a JSON-compatible value.
    #[pyo3(signature = (expression, *, timeout=10.0))]
    fn evaluate(&self, py: Python<'_>, expression: String, timeout: f64) -> PyResult<Py<PyAny>> {
        let value: Value = self.run(py, "evaluate", timeout, |page| async move {
            Ok(page.evaluate_expression(expression).await?.into_value()?)
        })?;
        Ok(pythonize::pythonize(py, &value)?.unbind())
    }

    /// Return non-ignored Chrome accessibility nodes for inspecting roles, names and state.
    #[pyo3(signature = (*, timeout=10.0))]
    fn snapshot(&self, py: Python<'_>, timeout: f64) -> PyResult<Py<PyAny>> {
        let nodes = self.run(py, "snapshot", timeout, |page| async move {
            let nodes = page
                .execute(GetFullAxTreeParams::default())
                .await?
                .result
                .nodes;
            Ok(nodes
                .into_iter()
                .filter(|node| !node.ignored)
                .collect::<Vec<_>>())
        })?;
        Ok(pythonize::pythonize(py, &nodes)?.unbind())
    }

    /// Capture PNG bytes suitable for display_image().
    #[pyo3(signature = (*, full_page=false, timeout=10.0))]
    fn screenshot<'py>(
        &self,
        py: Python<'py>,
        full_page: bool,
        timeout: f64,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.run(py, "screenshot", timeout, move |page| async move {
            Ok(page
                .screenshot(
                    ScreenshotParams::builder()
                        .format(CaptureScreenshotFormat::Png)
                        .full_page(full_page)
                        .build(),
                )
                .await?)
        })?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Build a main-document CSS locator, resolved again for every operation.
    fn locator(&self, selector: String) -> Locator {
        Locator::new(self.clone(), query::Query::Css(selector))
    }

    /// Locate elements by Chrome's computed accessibility role and accessible name.
    #[pyo3(signature = (role, *, name=None, exact=false))]
    fn get_by_role(&self, role: String, name: Option<String>, exact: bool) -> Locator {
        Locator::new(self.clone(), query::Query::Role { role, name, exact })
    }

    /// Match normalized text, selecting the deepest matching main-document elements.
    #[pyo3(signature = (text, *, exact=false))]
    fn get_by_text(&self, text: String, exact: bool) -> Locator {
        Locator::new(self.clone(), query::Query::Text { text, exact })
    }

    #[getter]
    fn keyboard(&self) -> Keyboard {
        Keyboard { page: self.clone() }
    }

    /// Register a background request rule; the most recently added matching rule wins.
    #[pyo3(signature = (pattern, *, action, response=None, request=None, timeout=10.0))]
    fn route(
        &self,
        py: Python<'_>,
        pattern: String,
        action: &str,
        response: Option<&Bound<'_, PyAny>>,
        request: Option<&Bound<'_, PyAny>>,
        timeout: f64,
    ) -> PyResult<String> {
        let rule = routing::rule(pattern, action, response, request)?;
        let resources = self.resources.clone();
        self.run(py, "route_add", timeout, |page| async move {
            let mut routing = resources.routing.lock().await;
            if routing.is_none() {
                *routing = Some(routing::Routing::start(page).await?);
            }
            routing.as_ref().unwrap().add(rule).await
        })
    }

    /// Remove one rule by its returned ID, or all rules when omitted.
    #[pyo3(signature = (route_id=None, *, timeout=10.0))]
    fn unroute(&self, py: Python<'_>, route_id: Option<String>, timeout: f64) -> PyResult<()> {
        let resources = self.resources.clone();
        self.run(py, "route_remove", timeout, |_page| async move {
            if let Some(routing) = resources.routing.lock().await.as_ref() {
                routing.remove(route_id.as_deref()).await?;
            }
            Ok(())
        })
    }

    /// Bring this tab to the front of its Chrome window.
    #[pyo3(signature = (*, timeout=10.0))]
    fn bring_to_front(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        self.run(py, "bring_to_front", timeout, |page| async move {
            page.bring_to_front().await?;
            Ok(())
        })
    }

    /// Close this tab, including when it belongs to an attached user's browser.
    #[pyo3(signature = (*, timeout=10.0))]
    fn close(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        let resources = self.resources.clone();
        self.run(py, "close_page", timeout, |page| async move {
            resources.close().await?;
            page.close().await?;
            Ok(())
        })?;
        self.state.forget_page(self.inner.target_id().as_ref());
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("Page(target_id={:?})", self.target_id())
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Keyboard>()?;
    module.add_class::<Page>()?;
    module.add_class::<Locator>()?;
    Ok(())
}
