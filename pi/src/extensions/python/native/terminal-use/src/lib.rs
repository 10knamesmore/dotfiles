#![cfg(unix)]

//! Unix PTY and terminal-screen support exposed directly to Python.

mod diagnostics;
mod error;
mod images;
mod input;
mod manager;
mod model;
mod palette;
mod session;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use nix::sys::signal::Signal;
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyList, PyModule, PySequence, PyString};
use pythonize::pythonize;

use crate::error::TerminalError;
use crate::images::{ImageSnapshot, encode_png};
use crate::input::{
    PyRect, encode_events, encode_key_input, encode_paste, encode_text, parse_cell_size,
    parse_rect, parse_wait,
};
use crate::manager::global_manager;
use crate::model::{InputSpec, ScreenSnapshot, SessionInfo};
use crate::session::Session;

const DEFAULT_WORKER_CONTROL_FDS: [i32; 2] = [3, 4];

static LIFECYCLE_HOOK: OnceLock<Mutex<Option<Py<PyAny>>>> = OnceLock::new();
static LIFECYCLE_THREAD: OnceLock<Mutex<Option<ThreadId>>> = OnceLock::new();

/// Register the public `terminal_use` module.
#[pymodule]
fn terminal_use(module: &Bound<'_, PyModule>) -> PyResult<()> {
    mark_worker_control_fds_cloexec(&DEFAULT_WORKER_CONTROL_FDS);
    module.add_class::<PySession>()?;
    module.add_class::<PyRect>()?;
    module.add_class::<crate::images::PyTerminalImage>()?;
    module.add_function(wrap_pyfunction!(start, module)?)?;
    module.add_function(wrap_pyfunction!(list, module)?)?;
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

/// Start a program attached to a new Unix PTY and return its Session handle.
#[pyfunction]
#[pyo3(signature = (argv, *, cwd = None, env = None, cols = 80, rows = 24, cell_size = None))]
fn start(
    py: Python<'_>,
    argv: Vec<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    cols: u16,
    rows: u16,
    cell_size: Option<Bound<'_, PyAny>>,
) -> PyResult<PySession> {
    ensure_lifecycle_thread()?;
    let cell_size = parse_cell_size(cell_size.as_ref())?;
    let inner = global_manager()
        .start(argv, cwd, env, rows, cols, cell_size)
        .map_err(terminal_error)?;
    let info = inner.info();
    if let Err(error) = emit_ownership(py, "opened", &info.id, info.pid) {
        let _ = global_manager().close(&inner, 500);
        return Err(error);
    }
    Ok(PySession { inner })
}

/// Return handles to this worker's running and exited sessions that have not been closed.
#[pyfunction]
fn list() -> Vec<PySession> {
    global_manager()
        .list()
        .into_iter()
        .map(|inner| PySession { inner })
        .collect()
}

/// A handle to one worker-owned PTY, created by start() or recovered through list().
/// Handles share process state; dropping a handle does not close the worker's session.
#[pyclass(name = "Session", frozen, module = "terminal_use")]
struct PySession {
    /// The same native session is shared by all Python handles and the worker registry.
    inner: Arc<Session>,
}

impl PySession {
    fn ensure_open(&self) -> PyResult<()> {
        if self.inner.info().closed {
            return Err(runtime_error("terminal session is closed"));
        }
        Ok(())
    }

    fn write_input(&self, input: InputSpec) -> PyResult<()> {
        self.ensure_open()?;
        self.inner.write(&input).map_err(terminal_error)
    }

    fn write_inputs(&self, inputs: &[InputSpec]) -> PyResult<()> {
        self.ensure_open()?;
        self.inner.write_many(inputs).map_err(terminal_error)
    }

    fn close_session(&self, py: Python<'_>, grace_ms: u64) -> PyResult<SessionInfo> {
        ensure_lifecycle_thread()?;
        if let Some(info) = global_manager()
            .close(&self.inner, grace_ms)
            .map_err(terminal_error)?
        {
            emit_ownership(py, "closed", &info.id, info.pid)?;
            Ok(info)
        } else {
            Ok(self.inner.info())
        }
    }
}

#[pymethods]
impl PySession {
    /// Worker-local diagnostic ID, also included in snapshots and ownership events.
    #[getter]
    fn id(&self) -> String {
        self.inner.info().id
    }

    /// PID of the program attached to this PTY, retained after it exits.
    #[getter]
    fn pid(&self) -> u32 {
        self.inner.info().pid
    }

    /// Whether close() has begun releasing the PTY; process exit alone does not close it.
    #[getter]
    fn closed(&self) -> bool {
        self.inner.info().closed
    }

    /// Inspect current process and terminal metadata, including after close().
    fn inspect(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_python(py, &self.inner.info())
    }

    /// Read a screen snapshot, optionally waiting for a screen or raw-output pattern.
    #[allow(clippy::too_many_arguments)] // Keep Python keyword options explicit and discoverable.
    #[pyo3(signature = (*, rect = None, wait_for = None, timeout = 0.0, trim_trailing_spaces = true, cells = false, images = false))]
    fn read(
        &self,
        py: Python<'_>,
        rect: Option<Bound<'_, PyAny>>,
        wait_for: Option<Bound<'_, PyAny>>,
        timeout: f64,
        trim_trailing_spaces: bool,
        cells: bool,
        images: bool,
    ) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
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
        loop {
            if !wait_for.is_empty() {
                loop {
                    self.ensure_open()?;
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    let matched = py.detach(|| {
                        self.inner.wait_screen(
                            rect,
                            &wait_for,
                            remaining.min(Duration::from_millis(50)),
                            trim_trailing_spaces,
                        )
                    });
                    if matched.map_err(terminal_error)? {
                        break;
                    }
                    py.check_signals()?;
                }
            }
            self.ensure_open()?;
            let mut snapshot = py
                .detach(|| {
                    self.inner
                        .read(rect, &wait_for, trim_trailing_spaces, cells, images)
                })
                .map_err(terminal_error)?;
            if wait_for.is_empty() || snapshot.wait.matched || Instant::now() >= deadline {
                return snapshot_to_python(py, &mut snapshot, images);
            }
            // A redraw can remove the match between the wait and the snapshot request.
            py.check_signals()?;
        }
    }

    /// Read retained bytes from the bounded raw-output ring without changing the screen snapshot.
    #[pyo3(signature = (*, max_bytes = 65536, since = None))]
    fn read_raw(
        &self,
        py: Python<'_>,
        max_bytes: usize,
        since: Option<u64>,
    ) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
        if max_bytes == 0 {
            return Err(value_error("max_bytes must be greater than 0"));
        }
        let output = py.detach(|| self.inner.read_raw(max_bytes, since));
        to_python(py, &output)
    }

    /// Wait for the child process and its PTY reader to finish; timeout does not kill the child.
    #[pyo3(signature = (*, timeout = 5.0))]
    fn wait(&self, py: Python<'_>, timeout: f64) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
        if !timeout.is_finite() || timeout < 0.0 {
            return Err(value_error("timeout must be a finite non-negative number"));
        }
        let deadline = Instant::now()
            .checked_add(Duration::from_secs_f64(timeout))
            .ok_or_else(|| value_error("timeout is too large"))?;
        loop {
            self.ensure_open()?;
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_default();
            let status = py.detach(|| self.inner.wait(remaining.min(Duration::from_millis(50))));
            if (status.exited && status.drained) || remaining.is_zero() {
                return to_python(py, &status);
            }
            py.check_signals()?;
        }
    }

    /// Validate all input events before writing; delay is milliseconds between events.
    #[pyo3(name = "input", signature = (events, *, delay = 0.0))]
    fn send_events(&self, py: Python<'_>, events: Bound<'_, PyAny>, delay: f64) -> PyResult<()> {
        self.ensure_open()?;
        let encoded_events = encode_events(&events)?;
        let delay = input_delay(delay)?;

        if delay.is_zero() {
            return self.write_inputs(&encoded_events);
        }

        for (index, input) in encoded_events.iter().enumerate() {
            self.write_inputs(std::slice::from_ref(input))?;
            if index + 1 < encoded_events.len() {
                py.detach(|| std::thread::sleep(delay));
                py.check_signals()?;
            }
        }
        Ok(())
    }

    /// Write literal UTF-8 text without appending a newline.
    fn send_text(&self, text: String) -> PyResult<()> {
        self.write_input(encode_text(&text))
    }

    /// Write one chord with separate modifier names and one US base key; no keys remain held.
    #[pyo3(signature = (*keys))]
    fn send_key(&self, keys: Vec<String>) -> PyResult<()> {
        self.write_input(encode_key_input(&keys)?)
    }

    /// Write text wrapped in bracketed-paste markers.
    fn paste(&self, text: String) -> PyResult<()> {
        self.write_input(encode_paste(&text))
    }

    /// Write bytes to the PTY master; child termios and the kernel line discipline still apply.
    fn write(&self, data: Vec<u8>) -> PyResult<()> {
        self.write_input(InputSpec::Bytes(data))
    }

    /// Change the PTY and screen dimensions, returning updated metadata.
    #[pyo3(signature = (*, cols, rows))]
    fn resize(&self, py: Python<'_>, cols: u16, rows: u16) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
        let info = self.inner.resize(rows, cols).map_err(terminal_error)?;
        to_python(py, &info)
    }

    /// Send a Unix signal to the PTY's process group.
    fn signal(&self, signal_name: String) -> PyResult<()> {
        self.ensure_open()?;
        let signal = match signal_name
            .trim()
            .trim_start_matches("SIG")
            .to_ascii_uppercase()
            .as_str()
        {
            "HUP" => Signal::SIGHUP,
            "INT" => Signal::SIGINT,
            "TERM" => Signal::SIGTERM,
            "KILL" => Signal::SIGKILL,
            "QUIT" => Signal::SIGQUIT,
            other => return Err(value_error(format!("unsupported Unix signal: {other}"))),
        };
        self.inner.signal(signal).map_err(terminal_error)
    }

    /// Close and reap the PTY; repeated calls return metadata without another ownership event.
    #[pyo3(signature = (*, grace_ms = 500))]
    fn close(&self, py: Python<'_>, grace_ms: u64) -> PyResult<Py<PyAny>> {
        let info = self.close_session(py, grace_ms)?;
        to_python(py, &info)
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyResult<PyRef<'_, Self>> {
        slf.ensure_open()?;
        Ok(slf)
    }

    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: &Bound<'_, PyAny>,
        _exc_value: &Bound<'_, PyAny>,
        _traceback: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        self.close_session(py, 500)?;
        Ok(false)
    }

    fn __repr__(&self) -> String {
        let info = self.inner.info();
        format!(
            "Session(id={:?}, pid={}, closed={})",
            info.id, info.pid, info.closed
        )
    }
}

/// Close every PTY session owned by this worker, including sessions with no saved Python handle.
#[pyfunction]
#[pyo3(signature = (*, grace_ms = 500))]
fn close_all(py: Python<'_>, grace_ms: u64) -> PyResult<()> {
    ensure_lifecycle_thread()?;
    let mut first_error = None;
    for session in list() {
        if let Err(error) = session.close_session(py, grace_ms)
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

/// Convert a screen snapshot to Python, replacing the skipped image field with frozen objects.
///
/// Source pixels are captured before this function runs; PNG encoding happens with the GIL
/// released because it is the expensive part of an image read.
fn snapshot_to_python(
    py: Python<'_>,
    snapshot: &mut ScreenSnapshot,
    include_images: bool,
) -> PyResult<Py<PyAny>> {
    let images = snapshot.images.take();
    let object = to_python(py, snapshot)?;
    let dict = object.bind(py).cast::<PyDict>()?;
    if !include_images {
        dict.set_item("images", py.None())?;
        return Ok(object);
    }

    let images = images.unwrap_or_default();
    let encoded: Result<Vec<(ImageSnapshot, Vec<u8>)>, String> = py.detach(|| {
        images
            .into_iter()
            .map(|image| encode_png(&image).map(|png| (image, png)))
            .collect()
    });
    let encoded =
        encoded.map_err(|error| runtime_error(format!("encode source image failed: {error}")))?;
    let list = PyList::empty(py);
    for (image, png) in encoded {
        list.append(Py::new(py, images::PyTerminalImage::new(image, png))?)?;
    }
    dict.set_item("images", list)?;
    Ok(object)
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
