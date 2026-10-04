//! Explicit desktop objects bind capture, input, applications and IPC to one target.

mod background;
mod control;
mod session;

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::capture::{self, Capture, Rect};
use crate::{hyprland, input::Input, wait};
use session::{Handle, Open, active_handles};

pub(crate) use control::{Control, Target};

/// Control one fixed desktop until close, user revocation or connection loss.
/// Saved captures survive closure, but this object and its Hyprland view never reconnect.
#[pyclass(module = "computer_use", frozen)]
struct Desktop {
    /// Shared with Hyprland and hold scopes; input stays on the native execution thread.
    handle: Arc<Handle>,
}

impl Desktop {
    fn point(&self, point: (f64, f64), relative_to: Option<&Capture>) -> PyResult<(f64, f64)> {
        self.handle.shared.check()?;
        match relative_to {
            Some(capture) if capture.desktop_id != self.handle.id => Err(wait::invalid(
                "relative_to capture belongs to another desktop",
            )),
            Some(capture) => capture.desktop_point(point.0, point.1),
            None if point.0.is_finite() && point.1.is_finite() => Ok(point),
            None => Err(wait::invalid("coordinates must be finite")),
        }
    }
}

#[pymethods]
impl Desktop {
    #[getter]
    fn status(&self) -> &'static str {
        self.handle.shared.status()
    }

    #[getter]
    fn hyprland(&self) -> PyResult<Hyprland> {
        self.handle.shared.check()?;
        Ok(Hyprland {
            handle: self.handle.clone(),
        })
    }

    /// Capture the focused or named output; rect uses native pixels and max_size only scales the image.
    #[pyo3(signature = (*, monitor=None, rect=None, max_size=None))]
    fn capture(
        &self,
        py: Python<'_>,
        monitor: Option<String>,
        rect: Option<Rect>,
        max_size: Option<u32>,
    ) -> PyResult<Capture> {
        let id = self.handle.id;
        self.handle.call(py, "capture", move |session| {
            capture::capture(
                &session.control,
                &session.target,
                id,
                monitor,
                rect,
                max_size,
            )
        })
    }

    #[pyo3(signature = (x, y, *, relative_to=None))]
    fn move_pointer(
        &self,
        py: Python<'_>,
        x: f64,
        y: f64,
        relative_to: Option<&Capture>,
    ) -> PyResult<()> {
        let point = self.point((x, y), relative_to)?;
        self.handle.call(py, "move_pointer", move |session| {
            session.input(|input, control, target| input.move_pointer(control, target, point))
        })
    }

    #[pyo3(signature = (x, y, *, button="left", count=1, relative_to=None))]
    fn click(
        &self,
        py: Python<'_>,
        x: f64,
        y: f64,
        button: &str,
        count: u32,
        relative_to: Option<&Capture>,
    ) -> PyResult<()> {
        let point = self.point((x, y), relative_to)?;
        let button = button.to_owned();
        self.handle.call(py, "click", move |session| {
            session
                .input(|input, control, target| input.click(control, target, point, &button, count))
        })
    }

    #[pyo3(signature = (start, end, *, button="left", duration=0.3, relative_to=None))]
    fn drag(
        &self,
        py: Python<'_>,
        start: (f64, f64),
        end: (f64, f64),
        button: &str,
        duration: f64,
        relative_to: Option<&Capture>,
    ) -> PyResult<()> {
        let start = self.point(start, relative_to)?;
        let end = self.point(end, relative_to)?;
        let button = button.to_owned();
        self.handle.call(py, "drag", move |session| {
            session.input(|input, control, target| {
                input.drag(control, target, start, end, &button, duration)
            })
        })
    }

    #[pyo3(signature = (amount, *, horizontal=false))]
    fn scroll(&self, py: Python<'_>, amount: i32, horizontal: bool) -> PyResult<()> {
        self.handle.call(py, "scroll", move |session| {
            session.input(|input, control, _| input.scroll(control, amount, horizontal))
        })
    }

    #[pyo3(signature = (*keys, count=1, interval=0.05))]
    fn press(&self, py: Python<'_>, keys: Vec<String>, count: u32, interval: f64) -> PyResult<()> {
        self.handle.call(py, "press", move |session| {
            session.input(|input, control, _| input.press(control, &keys, count, interval))
        })
    }

    #[pyo3(signature = (*keys))]
    fn key_down(&self, py: Python<'_>, keys: Vec<String>) -> PyResult<()> {
        self.handle.call(py, "key_down", move |session| {
            session.input(|input, control, _| input.key_down(control, &keys).map(|_| ()))
        })
    }

    #[pyo3(signature = (*keys))]
    fn key_up(&self, py: Python<'_>, keys: Vec<String>) -> PyResult<()> {
        self.handle.call(py, "key_up", move |session| {
            session.input(|input, control, _| input.key_up(control, &keys))
        })
    }

    fn held_keys<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let keys = self.handle.call(py, "held_keys", |session| {
            session.input(|input, _, _| Ok(input.held_keys()))
        })?;
        PyTuple::new(py, keys)
    }

    fn release_keys(&self, py: Python<'_>) -> PyResult<()> {
        self.handle.call(py, "release_keys", |session| {
            session.input(|input, control, _| input.run(control, |input| input.release_keys()))
        })
    }

    #[pyo3(signature = (*keys))]
    fn hold(&self, py: Python<'_>, keys: Vec<String>) -> PyResult<Hold> {
        let names = keys.clone();
        self.handle.call(py, "hold", move |session| {
            Input::validate_keys(&session.control, &names)
        })?;
        Ok(Hold {
            handle: self.handle.clone(),
            keys,
            acquisitions: None,
        })
    }

    #[pyo3(signature = (text, /, *, interval=0.0))]
    fn type_text(&self, py: Python<'_>, text: String, interval: f64) -> PyResult<()> {
        self.handle.call(py, "type_text", move |session| {
            session.input(|input, control, _| input.type_text(control, &text, interval))
        })
    }

    /// Launch argv directly on this desktop. Background apps close with it; host apps stay open.
    #[pyo3(signature = (argv, *, cwd=None))]
    fn launch(&self, py: Python<'_>, argv: Vec<String>, cwd: Option<String>) -> PyResult<u32> {
        self.handle.call(py, "desktop.launch", move |session| {
            session.launch(argv, cwd)
        })
    }

    /// Release input and wait for cleanup; idempotent and preserves a revocation/disconnection reason.
    fn close(&self, py: Python<'_>) {
        self.handle.close(py);
    }

    fn __enter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<PyRef<'py, Self>> {
        slf.handle.call(py, "desktop.enter", |_| Ok(()))?;
        Ok(slf)
    }

    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: &Bound<'_, PyAny>,
        _exc_value: &Bound<'_, PyAny>,
        _traceback: &Bound<'_, PyAny>,
    ) -> bool {
        self.close(py);
        false
    }

    fn __repr__(&self) -> String {
        format!("Desktop(status={:?})", self.status())
    }
}

/// Query or dispatch to the same target and lifetime as the originating Desktop.
#[pyclass(module = "computer_use", frozen)]
struct Hyprland {
    /// Prevents a saved view from bypassing desktop invalidation.
    handle: Arc<Handle>,
}

#[pymethods]
impl Hyprland {
    fn query<'py>(&self, py: Python<'py>, name: String) -> PyResult<Bound<'py, PyAny>> {
        let value = self.handle.call(py, "hyprland.query", move |session| {
            hyprland::query_value(&session.control, &session.target, &name)
        })?;
        pythonize::pythonize(py, &value).map_err(Into::into)
    }

    fn dispatch(&self, py: Python<'_>, expression: String) -> PyResult<()> {
        self.handle.call(py, "hyprland.dispatch", move |session| {
            hyprland::dispatch(&session.control, &session.target, &expression)
        })
    }
}

/// Release only this entry's newly acquired keys, preserving outer scopes and later re-presses.
#[pyclass(module = "computer_use")]
struct Hold {
    /// Keeps the scope bound to its original desktop, including after close.
    handle: Arc<Handle>,

    /// Validated key names; construction does not press them.
    keys: Vec<String>,

    /// None outside a with scope; tokens identify acquisitions rather than key names.
    acquisitions: Option<Vec<u64>>,
}

#[pymethods]
impl Hold {
    fn __enter__(&mut self, py: Python<'_>) -> PyResult<()> {
        if self.acquisitions.is_some() {
            return Err(wait::runtime("this hold scope is already entered"));
        }
        let keys = self.keys.clone();
        self.acquisitions = Some(self.handle.call(py, "hold.enter", move |session| {
            session.input(|input, control, _| input.key_down(control, &keys))
        })?);
        Ok(())
    }

    fn __exit__(
        &mut self,
        py: Python<'_>,
        exc_type: &Bound<'_, PyAny>,
        _exc_value: &Bound<'_, PyAny>,
        _traceback: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        if let Some(acquisitions) = self.acquisitions.take() {
            let result = self.handle.call(py, "hold.exit", move |session| {
                session.input(|input, control, _| {
                    input.run(control, |input| {
                        input.release_acquisitions(&acquisitions);
                        Ok(())
                    })
                })
            });
            if exc_type.is_none() {
                result?;
            }
        }
        Ok(false)
    }
}

/// Exclusively connect to the current Hyprland after its Quickshell takeover overlay is visible.
#[pyfunction]
fn connect_host(py: Python<'_>) -> PyResult<Desktop> {
    Ok(Desktop {
        handle: Handle::open(py, Open::Host(Target::host()?))?,
    })
}

/// Create a non-visible Hyprland desktop backed by KWin virtual outputs; size is pixels at scale 1.
#[pyfunction]
#[pyo3(signature = (*, size=(1920, 1080)))]
fn create_background(py: Python<'_>, size: (u32, u32)) -> PyResult<Desktop> {
    if size.0 == 0 || size.1 == 0 {
        return Err(wait::invalid("size must contain positive width and height"));
    }
    let python = py.import("sys")?.getattr("executable")?.extract()?;
    Ok(Desktop {
        handle: Handle::open(py, Open::Background { python, size })?,
    })
}

#[pyfunction]
fn _release_inputs(py: Python<'_>) -> PyResult<()> {
    let mut result = Ok(());
    for handle in active_handles() {
        if handle.shared.check().is_ok() {
            let released = handle.call(py, "input.cleanup", |session| session.release_inputs());
            if released.is_err() {
                handle.close(py);
                result = released;
            }
        }
    }
    result
}

/// Close all live desktop objects on worker failure or exit, invalidating them before joining cleanup.
#[pyfunction]
fn _close_all(py: Python<'_>) {
    let handles = active_handles();
    for handle in &handles {
        handle.shared.invalidate(control::CLOSED);
    }
    for handle in handles {
        handle.close(py);
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Desktop>()?;
    module.add_class::<Hyprland>()?;
    module.add_class::<Hold>()?;
    module.add_function(wrap_pyfunction!(connect_host, module)?)?;
    module.add_function(wrap_pyfunction!(create_background, module)?)?;
    module.add_function(wrap_pyfunction!(_release_inputs, module)?)?;
    module.add_function(wrap_pyfunction!(_close_all, module)?)?;
    Ok(())
}
