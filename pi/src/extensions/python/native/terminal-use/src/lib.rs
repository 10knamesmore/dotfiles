#![cfg(unix)]

//! Unix PTY and terminal-screen support exposed directly to Python.

mod diagnostics;
mod error;
mod input;
mod manager;
mod model;
mod palette;
mod session;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyModule, PySequence, PyString};
use pythonize::pythonize;

use crate::error::TerminalError;
use crate::input::{
    PyRect, encode_events, encode_key_input, encode_paste, encode_text, parse_rect, parse_wait,
};
use crate::manager::global_manager;
use crate::model::InputSpec;

const DEFAULT_WORKER_CONTROL_FDS: [i32; 2] = [3, 4];

static LIFECYCLE_HOOK: OnceLock<Mutex<Option<Py<PyAny>>>> = OnceLock::new();
static LIFECYCLE_THREAD: OnceLock<Mutex<Option<ThreadId>>> = OnceLock::new();

/// Register the public `terminal_use` module.
#[pymodule]
fn terminal_use(module: &Bound<'_, PyModule>) -> PyResult<()> {
    mark_worker_control_fds_cloexec(&DEFAULT_WORKER_CONTROL_FDS);
    module.add_class::<PyRect>()?;
    module.add_function(wrap_pyfunction!(start, module)?)?;
    module.add_function(wrap_pyfunction!(list, module)?)?;
    module.add_function(wrap_pyfunction!(inspect, module)?)?;
    module.add_function(wrap_pyfunction!(read, module)?)?;
    module.add_function(wrap_pyfunction!(read_raw, module)?)?;
    module.add_function(wrap_pyfunction!(wait, module)?)?;
    module.add_function(wrap_pyfunction!(send_events, module)?)?;
    module.add_function(wrap_pyfunction!(send_text, module)?)?;
    module.add_function(wrap_pyfunction!(send_key, module)?)?;
    module.add_function(wrap_pyfunction!(paste, module)?)?;
    module.add_function(wrap_pyfunction!(write, module)?)?;
    module.add_function(wrap_pyfunction!(resize, module)?)?;
    module.add_function(wrap_pyfunction!(signal, module)?)?;
    module.add_function(wrap_pyfunction!(close, module)?)?;
    module.add_function(wrap_pyfunction!(close_all, module)?)?;
    module.add_function(wrap_pyfunction!(_set_lifecycle_hook, module)?)?;
    module.add_function(wrap_pyfunction!(_set_worker_control_fds, module)?)?;

    let atexit = module.py().import("atexit")?;
    atexit.call_method1("register", (module.getattr("close_all")?,))?;
    Ok(())
}

/// Install or clear the worker callback receiving PTY ownership events.
#[pyfunction]
#[pyo3(signature = (callback = None))]
fn _set_lifecycle_hook(callback: Option<Bound<'_, PyAny>>) -> PyResult<()> {
    if let Some(callback) = callback.as_ref()
        && !callback.is_callable()
    {
        return Err(type_error("lifecycle hook must be callable or None"));
    }
    let thread = callback.as_ref().map(|_| std::thread::current().id());
    *lifecycle_hook()
        .lock()
        .map_err(|_| runtime_error("lifecycle hook mutex poisoned"))? = callback.map(Bound::unbind);
    *lifecycle_thread()
        .lock()
        .map_err(|_| runtime_error("lifecycle thread mutex poisoned"))? = thread;
    Ok(())
}

/// Override the worker control descriptors protected from PTY child inheritance.
#[pyfunction]
#[pyo3(signature = (fds = None))]
fn _set_worker_control_fds(fds: Option<Bound<'_, PyAny>>) -> PyResult<()> {
    let fds = match fds {
        None => DEFAULT_WORKER_CONTROL_FDS.to_vec(),
        Some(value) => parse_worker_control_fds(&value)?,
    };
    mark_worker_control_fds_cloexec(&fds);
    Ok(())
}

/// Start a program attached to a new Unix PTY.
#[pyfunction]
#[pyo3(signature = (argv, *, cwd = None, env = None, cols = 80, rows = 24))]
fn start(
    py: Python<'_>,
    argv: Vec<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    cols: u16,
    rows: u16,
) -> PyResult<Py<PyAny>> {
    ensure_lifecycle_thread()?;
    let info = global_manager()
        .start(argv, cwd, env, rows, cols)
        .map_err(terminal_error)?;
    if let Err(error) = emit_ownership(py, "opened", &info.id, info.pid) {
        let _ = global_manager().close(&info.id, 500);
        return Err(error);
    }
    to_python(py, &info)
}

/// List live and recently exited PTY sessions owned by this worker.
#[pyfunction]
fn list(py: Python<'_>) -> PyResult<Py<PyAny>> {
    to_python(py, &global_manager().list())
}

/// Inspect one PTY session without consuming output.
#[pyfunction]
fn inspect(py: Python<'_>, session_id: String) -> PyResult<Py<PyAny>> {
    let info = global_manager()
        .inspect(&session_id)
        .map_err(terminal_error)?;
    to_python(py, &info)
}

/// Read a screen snapshot, optionally waiting for a screen or raw-output pattern.
#[pyfunction]
#[pyo3(signature = (session_id, *, rect = None, wait_for = None, timeout = 0.0, trim_trailing_spaces = true, cells = false))]
fn read(
    py: Python<'_>,
    session_id: String,
    rect: Option<Bound<'_, PyAny>>,
    wait_for: Option<Bound<'_, PyAny>>,
    timeout: f64,
    trim_trailing_spaces: bool,
    cells: bool,
) -> PyResult<Py<PyAny>> {
    let rect = parse_rect(rect.as_ref())?;
    let wait_for = parse_wait(wait_for.as_ref())?;
    if !timeout.is_finite() || timeout < 0.0 {
        return Err(value_error("timeout must be a finite non-negative number"));
    }
    if wait_for.is_empty() && timeout != 0.0 {
        return Err(value_error("timeout is only used with wait_for"));
    }
    if !wait_for.is_empty() && timeout == 0.0 {
        return Err(value_error(
            "wait_for requires a finite timeout greater than 0",
        ));
    }
    let deadline = Instant::now()
        .checked_add(Duration::from_secs_f64(timeout))
        .ok_or_else(|| value_error("timeout is too large"))?;
    let has_wait = !wait_for.is_empty();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        let slice = if has_wait {
            remaining.min(Duration::from_millis(50))
        } else {
            Duration::ZERO
        };
        let snapshot = py.detach(|| {
            global_manager().read(
                &session_id,
                rect,
                wait_for.clone(),
                slice.as_secs_f64(),
                trim_trailing_spaces,
                cells,
            )
        });
        let snapshot = snapshot.map_err(terminal_error)?;
        if !has_wait || snapshot.wait.matched || remaining.is_zero() {
            return to_python(py, &snapshot);
        }
        py.check_signals()?;
    }
}

/// Read retained bytes from the bounded raw-output ring without changing the screen snapshot.
#[pyfunction]
#[pyo3(signature = (session_id, *, max_bytes = 65536, since = None))]
fn read_raw(
    py: Python<'_>,
    session_id: String,
    max_bytes: usize,
    since: Option<u64>,
) -> PyResult<Py<PyAny>> {
    if max_bytes == 0 {
        return Err(value_error("max_bytes must be greater than 0"));
    }
    let output = py
        .detach(|| global_manager().read_raw(&session_id, max_bytes, since))
        .map_err(terminal_error)?;
    to_python(py, &output)
}

/// Wait for the child process and its PTY reader to finish.
#[pyfunction]
#[pyo3(signature = (session_id, *, timeout = 5.0))]
fn wait(py: Python<'_>, session_id: String, timeout: f64) -> PyResult<Py<PyAny>> {
    if !timeout.is_finite() || timeout < 0.0 {
        return Err(value_error("timeout must be a finite non-negative number"));
    }
    let deadline = Instant::now()
        .checked_add(Duration::from_secs_f64(timeout))
        .ok_or_else(|| value_error("timeout is too large"))?;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        let status = py.detach(|| {
            global_manager().wait(
                &session_id,
                remaining.min(Duration::from_millis(50)).as_secs_f64(),
            )
        });
        let status = status.map_err(terminal_error)?;
        if (status.exited && status.drained) || remaining.is_zero() {
            return to_python(py, &status);
        }
        py.check_signals()?;
    }
}

/// Validate and write a sequence of encoded input events.
#[pyfunction(name = "input")]
#[pyo3(signature = (session_id, events, *, delay = 0.0))]
fn send_events(
    py: Python<'_>,
    session_id: String,
    events: Bound<'_, PyAny>,
    delay: f64,
) -> PyResult<()> {
    let encoded_events = encode_events(&events)?;
    let delay = input_delay(delay)?;

    if delay.is_zero() {
        return write_inputs(&session_id, &encoded_events);
    }

    for (index, input) in encoded_events.iter().enumerate() {
        write_inputs(&session_id, std::slice::from_ref(input))?;
        if index + 1 < encoded_events.len() {
            py.detach(|| std::thread::sleep(delay));
            py.check_signals()?;
        }
    }
    Ok(())
}

/// Write literal UTF-8 text without appending a newline.
#[pyfunction]
fn send_text(session_id: String, text: String) -> PyResult<()> {
    write_input(&session_id, encode_text(&text))
}

/// Write one encoded terminal key press.
#[pyfunction]
fn send_key(session_id: String, key: String) -> PyResult<()> {
    write_input(&session_id, encode_key_input(&key)?)
}

/// Write text wrapped in bracketed-paste markers.
#[pyfunction]
fn paste(session_id: String, text: String) -> PyResult<()> {
    write_input(&session_id, encode_paste(&text))
}

/// Write bytes to the PTY master; child termios and the kernel line discipline may translate, echo, buffer, or interpret them as signals.
#[pyfunction]
fn write(session_id: String, data: Vec<u8>) -> PyResult<()> {
    write_input(&session_id, InputSpec::Bytes(data))
}

/// Change the PTY and screen dimensions.
#[pyfunction]
#[pyo3(signature = (session_id, *, cols, rows))]
fn resize(py: Python<'_>, session_id: String, cols: u16, rows: u16) -> PyResult<Py<PyAny>> {
    let info = global_manager()
        .resize(&session_id, rows, cols)
        .map_err(terminal_error)?;
    to_python(py, &info)
}

/// Send a Unix signal to the PTY's process group.
#[pyfunction]
fn signal(session_id: String, signal_name: String) -> PyResult<()> {
    global_manager()
        .signal(&session_id, &signal_name)
        .map_err(terminal_error)
}

/// Close one PTY session and reap its child process.
#[pyfunction]
#[pyo3(signature = (session_id, *, grace_ms = 500))]
fn close(py: Python<'_>, session_id: String, grace_ms: u64) -> PyResult<Py<PyAny>> {
    ensure_lifecycle_thread()?;
    let info = global_manager()
        .close(&session_id, grace_ms)
        .map_err(terminal_error)?;
    emit_ownership(py, "closed", &info.id, info.pid)?;
    to_python(py, &info)
}

/// Close every PTY session owned by this worker.
#[pyfunction]
#[pyo3(signature = (*, grace_ms = 500))]
fn close_all(py: Python<'_>, grace_ms: u64) -> PyResult<()> {
    ensure_lifecycle_thread()?;
    let sessions = global_manager().list();
    let mut first_error = None;
    for session in sessions {
        let result = global_manager()
            .close(&session.id, grace_ms)
            .map_err(terminal_error)
            .and_then(|info| {
                emit_ownership(py, "closed", &info.id, info.pid)?;
                Ok(info)
            });
        if let Err(error) = result
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn mark_worker_control_fds_cloexec(fds: &[i32]) {
    for &fd in fds {
        // The Python worker owns these protocol descriptors. PTY children must not inherit them,
        // otherwise a child application's own descendants can corrupt the worker event stream.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags >= 0 {
            unsafe {
                libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            }
        }
    }
}

fn input_delay(delay_ms: f64) -> PyResult<Duration> {
    if !delay_ms.is_finite() || delay_ms < 0.0 {
        return Err(value_error(
            "delay must be a finite non-negative number of milliseconds",
        ));
    }
    Duration::try_from_secs_f64(delay_ms / 1000.0).map_err(|_| value_error("delay is too large"))
}

fn write_input(session_id: &str, input: InputSpec) -> PyResult<()> {
    global_manager()
        .write(session_id, &input)
        .map_err(terminal_error)
}

fn write_inputs(session_id: &str, inputs: &[InputSpec]) -> PyResult<()> {
    global_manager()
        .write_many(session_id, inputs)
        .map_err(terminal_error)
}

fn emit_ownership(py: Python<'_>, action: &str, session_id: &str, pid: u32) -> PyResult<()> {
    ensure_lifecycle_thread()?;
    let callback = lifecycle_hook()
        .lock()
        .map_err(|_| runtime_error("lifecycle hook mutex poisoned"))?
        .as_ref()
        .map(|callback| callback.clone_ref(py));
    let Some(callback) = callback else {
        return Ok(());
    };

    let event = PyDict::new(py);
    event.set_item("type", "terminal_ownership")?;
    event.set_item("action", action)?;
    event.set_item("id", session_id)?;
    event.set_item("pid", pid)?;
    callback.bind(py).call1((event,))?;
    Ok(())
}

fn ensure_lifecycle_thread() -> PyResult<()> {
    let expected_thread = lifecycle_thread()
        .lock()
        .map_err(|_| runtime_error("lifecycle thread mutex poisoned"))?
        .to_owned();
    if expected_thread.is_some_and(|thread| thread != std::thread::current().id()) {
        return Err(runtime_error(
            "terminal lifecycle operations must run on the thread that registered the lifecycle hook",
        ));
    }
    Ok(())
}

fn parse_worker_control_fds(value: &Bound<'_, PyAny>) -> PyResult<Vec<i32>> {
    if value.is_instance_of::<PyString>() || value.is_instance_of::<PyBool>() {
        return Err(type_error(
            "worker control fds must be a sequence of integers",
        ));
    }
    let sequence = value
        .cast::<PySequence>()
        .map_err(|_| type_error("worker control fds must be a sequence of integers"))?;
    let mut fds = Vec::with_capacity(sequence.len()?);
    for index in 0..sequence.len()? {
        let item = sequence.get_item(index)?;
        if item.is_instance_of::<PyBool>() {
            return Err(type_error(format!(
                "worker control fd {index} must be an integer"
            )));
        }
        let fd = item
            .extract::<i64>()
            .map_err(|_| type_error(format!("worker control fd {index} must be an integer")))?;
        if !(0..=i64::from(i32::MAX)).contains(&fd) {
            return Err(value_error(format!(
                "worker control fd {index} must be between 0 and {}",
                i32::MAX
            )));
        }
        fds.push(fd as i32);
    }
    Ok(fds)
}

fn lifecycle_hook() -> &'static Mutex<Option<Py<PyAny>>> {
    LIFECYCLE_HOOK.get_or_init(|| Mutex::new(None))
}

fn lifecycle_thread() -> &'static Mutex<Option<ThreadId>> {
    LIFECYCLE_THREAD.get_or_init(|| Mutex::new(None))
}

fn to_python<T: serde::Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    pythonize(py, value).map(Bound::unbind).map_err(|error| {
        PyRuntimeError::new_err(format!("convert terminal result failed: {error}"))
    })
}

fn terminal_error(error: TerminalError) -> PyErr {
    match error {
        TerminalError::InvalidArgument(message) => PyValueError::new_err(message),
        TerminalError::Runtime(message) => PyRuntimeError::new_err(message),
    }
}

fn runtime_error(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

fn type_error(error: impl std::fmt::Display) -> PyErr {
    PyTypeError::new_err(error.to_string())
}

fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}
