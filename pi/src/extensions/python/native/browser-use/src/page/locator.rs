//! Apply strict locators to current DOM objects and send input only after the requested state is ready.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use chromiumoxide::cdp::browser_protocol::{dom::SetFileInputFilesParams, input::InsertTextParams};
use chromiumoxide::layout::Point;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use serde::Deserialize;

use super::{
    Page, keyboard,
    query::{Element, Query},
};

/// Store a query rather than an element reference, so rerenders do not stale the locator.
#[pyclass(frozen, module = "browser_use")]
pub(crate) struct Locator {
    page: Page,
    query: Query,
}

impl Locator {
    pub(super) fn new(page: Page, query: Query) -> Self {
        Self { page, query }
    }
}

#[derive(Deserialize)]
struct ElementState {
    connected: bool,
    visible: Option<bool>,
    ready: Option<bool>,
    x: Option<f64>,
    y: Option<f64>,
    text: Option<String>,
    multiple: Option<bool>,
}

async fn inspect(element: &Element, action: &str) -> anyhow::Result<ElementState> {
    let function = format!(
        "function() {{ return ({}).call(this, {}); }}",
        include_str!("locator.js"),
        serde_json::to_string(action)?
    );
    Ok(serde_json::from_value(element.call(function).await?)?)
}

async fn ready(
    page: &chromiumoxide::Page,
    query: &Query,
    action: &str,
) -> anyhow::Result<(Element, ElementState)> {
    let mut previous = None;
    loop {
        let matches = query.resolve(page).await?;
        if matches.count > 1 {
            bail!(
                "Locator matched {} elements; refine the query before acting",
                matches.count
            );
        }
        if let Some(element) = matches.element {
            let state = inspect(&element, action).await?;
            if state.connected && (action == "text" || state.ready == Some(true)) {
                if action != "click" || previous == Some((state.x, state.y)) {
                    return Ok((element, state));
                }
                previous = Some((state.x, state.y));
            } else {
                previous = None;
            }
        } else {
            previous = None;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[derive(FromPyObject)]
enum InputFiles {
    Single(PathBuf),
    Multiple(Vec<PathBuf>),
}

#[pymethods]
impl Locator {
    /// Return the current match count without waiting for an element to appear.
    #[pyo3(signature = (*, timeout=10.0))]
    fn count(&self, py: Python<'_>, timeout: f64) -> PyResult<usize> {
        let query = self.query.clone();
        self.page
            .run(py, "locator_count", timeout, |page| async move {
                Ok(query.resolve(&page).await?.count)
            })
    }

    /// Wait for one visible, enabled, unobscured element and send a mouse click.
    #[pyo3(signature = (*, timeout=10.0))]
    fn click(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        let query = self.query.clone();
        self.page.run(py, "click", timeout, |page| async move {
            let (_element, state) = ready(&page, &query, "click").await?;
            page.click(Point {
                x: state.x.unwrap(),
                y: state.y.unwrap(),
            })
            .await?;
            Ok(())
        })
    }

    /// Replace editable text through Chrome input events; an empty string clears the content.
    #[pyo3(signature = (text, *, timeout=10.0))]
    fn fill(&self, py: Python<'_>, text: String, timeout: f64) -> PyResult<()> {
        let query = self.query.clone();
        let lock = self.page.resources.keyboard.clone();
        let backspace = keyboard::chord(vec!["Backspace".into()])?;
        self.page.run(py, "fill", timeout, |page| async move {
            let guard = lock.lock_owned().await;
            let (_element, _) = ready(&page, &query, "fill").await?;
            if text.is_empty() {
                keyboard::send_chord(page, backspace, guard).await?;
            } else {
                page.execute(InsertTextParams::new(text)).await?;
            }
            Ok(())
        })
    }

    /// Focus the matched element and press a chord, e.g. press("ctrl", "a").
    #[pyo3(signature = (*keys, timeout=10.0))]
    fn press(&self, py: Python<'_>, keys: Vec<String>, timeout: f64) -> PyResult<()> {
        let keys = keyboard::chord(keys)?;
        let query = self.query.clone();
        let lock = self.page.resources.keyboard.clone();
        self.page
            .run(py, "locator_press", timeout, |page| async move {
                let guard = lock.lock_owned().await;
                let (_element, _) = ready(&page, &query, "focus").await?;
                keyboard::send_chord(page, keys, guard).await
            })
    }

    /// Set a file input from local paths, including hidden inputs; an empty list clears it.
    #[pyo3(signature = (files, *, timeout=10.0))]
    fn set_input_files(&self, py: Python<'_>, files: InputFiles, timeout: f64) -> PyResult<()> {
        let paths = match files {
            InputFiles::Single(path) => vec![path],
            InputFiles::Multiple(paths) => paths,
        };
        let files = paths
            .into_iter()
            .map(|path| {
                let path = std::fs::canonicalize(path)
                    .map_err(|error| PyOSError::new_err(error.to_string()))?;
                if !path.is_file() {
                    return Err(PyValueError::new_err("Upload paths must be regular files"));
                }
                Ok(path.to_string_lossy().into_owned())
            })
            .collect::<PyResult<Vec<_>>>()?;
        let query = self.query.clone();
        self.page
            .run(py, "set_input_files", timeout, |page| async move {
                let (element, state) = ready(&page, &query, "files").await?;
                if files.len() > 1 && state.multiple != Some(true) {
                    bail!("File input does not allow multiple files");
                }
                if files.is_empty() {
                    // Chrome's file-path command leaves an existing selection unchanged for an empty list.
                    element.call("function() { this.files = new DataTransfer().files; this.dispatchEvent(new Event('input', {bubbles:true, composed:true})); this.dispatchEvent(new Event('change', {bubbles:true})); }".into()).await?;
                } else {
                    page.execute(SetFileInputFilesParams::builder().files(files).object_id(element.id.clone()).build().map_err(anyhow::Error::msg)?).await?;
                }
                Ok(())
            })
    }

    /// Wait for one attached element and read its text without requiring visibility.
    #[pyo3(signature = (*, timeout=10.0))]
    fn inner_text(&self, py: Python<'_>, timeout: f64) -> PyResult<String> {
        let query = self.query.clone();
        self.page.run(py, "inner_text", timeout, |page| async move {
            Ok(ready(&page, &query, "text")
                .await?
                .1
                .text
                .unwrap_or_default())
        })
    }

    /// Wait for attached, detached, visible or hidden state; timeouts use seconds.
    #[pyo3(signature = (*, state="visible", timeout=10.0))]
    fn wait_for(&self, py: Python<'_>, state: &str, timeout: f64) -> PyResult<()> {
        if !["attached", "detached", "visible", "hidden"].contains(&state) {
            return Err(PyValueError::new_err(
                "state must be attached, detached, visible or hidden",
            ));
        }
        let state = state.to_owned();
        let query = self.query.clone();
        self.page.run(py, "wait_for", timeout, |page| async move {
            loop {
                let matches = query.resolve(&page).await?;
                if matches.count > 1 {
                    bail!(
                        "Locator matched {} elements; refine the query",
                        matches.count
                    );
                }
                let element = match matches.element {
                    Some(element) => Some(inspect(&element, "visible").await?),
                    None => None,
                };
                let attached = element.as_ref().is_some_and(|state| state.connected);
                let visible = element
                    .as_ref()
                    .is_some_and(|state| state.visible == Some(true));
                let ready = match state.as_str() {
                    "attached" => attached,
                    "detached" => !attached,
                    "visible" => visible,
                    "hidden" => !attached || !visible,
                    _ => unreachable!(),
                };
                if ready {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
    }

    fn __repr__(&self) -> String {
        format!("Locator({:?})", self.query)
    }
}
