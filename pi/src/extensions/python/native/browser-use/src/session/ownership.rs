//! Report browser process groups to the Python worker's parent supervisor.

use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

struct Hook {
    callback: Py<PyAny>,
    thread: ThreadId,
}

fn hook() -> &'static Mutex<Option<Hook>> {
    static HOOK: OnceLock<Mutex<Option<Hook>>> = OnceLock::new();
    HOOK.get_or_init(Mutex::default)
}

#[pyfunction]
#[pyo3(signature = (callback=None))]
fn _set_lifecycle_hook(callback: Option<Bound<'_, PyAny>>) -> PyResult<()> {
    if callback
        .as_ref()
        .is_some_and(|callback| !callback.is_callable())
    {
        return Err(PyTypeError::new_err(
            "lifecycle hook must be callable or None",
        ));
    }
    *hook().lock().unwrap() = callback.map(|callback| Hook {
        callback: callback.unbind(),
        thread: std::thread::current().id(),
    });
    Ok(())
}

pub(super) fn check_thread() -> PyResult<()> {
    if hook()
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|hook| hook.thread != std::thread::current().id())
    {
        return Err(PyRuntimeError::new_err(
            "browser lifecycle operations must run on the worker thread",
        ));
    }
    Ok(())
}

pub(super) fn notify(py: Python<'_>, action: &str, id: u64, pid: u32) -> PyResult<()> {
    check_thread()?;
    let callback = hook()
        .lock()
        .unwrap()
        .as_ref()
        .map(|hook| hook.callback.clone_ref(py));
    if let Some(callback) = callback {
        let event = serde_json::json!({"action": action, "id": id.to_string(), "pid": pid});
        callback.call1(py, (pythonize::pythonize(py, &event)?,))?;
    }
    Ok(())
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(_set_lifecycle_hook, module)?)
}
