//! Speak current Hyprland JSON queries and Lua dispatch over its Unix socket.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use pyo3::prelude::*;
use serde::Deserialize;
use serde_json::Value;

use crate::{logging, wait};

fn request(py: Python<'_>, command: &str) -> PyResult<String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| wait::runtime("XDG_RUNTIME_DIR is not set"))?;
    let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")
        .ok_or_else(|| wait::runtime("HYPRLAND_INSTANCE_SIGNATURE is not set"))?;
    let path = PathBuf::from(runtime)
        .join("hypr")
        .join(signature)
        .join(".socket.sock");
    let deadline = Instant::now() + Duration::from_secs(5);
    // Unix connect is bounded by the local listening socket; nonblocking mode also
    // covers a full compositor accept queue, before any command is transmitted.
    let socket = socket_connect(py, &path, deadline)?;
    let mut socket = socket;
    let mut bytes = command.as_bytes();
    while !bytes.is_empty() {
        py.check_signals()?;
        match socket.write(bytes) {
            Ok(0) => return Err(wait::runtime("Hyprland closed the request socket")),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                wait::poll(py, socket.as_raw_fd(), libc::POLLOUT, deadline, true)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(wait::runtime(error)),
        }
    }
    let mut response = Vec::new();
    loop {
        py.check_signals()?;
        let mut chunk = [0; 16384];
        match socket.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => response.extend_from_slice(&chunk[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                wait::poll(py, socket.as_raw_fd(), libc::POLLIN, deadline, true)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(wait::runtime(error)),
        }
        if Instant::now() >= deadline {
            return Err(pyo3::exceptions::PyTimeoutError::new_err(
                "Hyprland request timed out",
            ));
        }
    }
    String::from_utf8(response).map_err(wait::runtime)
}

pub(crate) fn socket_connect(
    py: Python<'_>,
    path: &std::path::Path,
    deadline: Instant,
) -> PyResult<UnixStream> {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();
    // sockaddr_un is zero-initialized and the copied path is explicitly bounded.
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= address.sun_path.len() {
        return Err(wait::runtime("Hyprland socket path is too long"));
    }
    address.sun_family = libc::AF_UNIX as _;
    for (target, source) in address.sun_path.iter_mut().zip(bytes) {
        *target = *source as _;
    }
    // The returned descriptor is immediately owned and closed on every error path.
    let raw = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if raw < 0 {
        return Err(wait::runtime(std::io::Error::last_os_error()));
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    loop {
        py.check_signals()?;
        let result = unsafe {
            libc::connect(
                raw,
                (&address as *const libc::sockaddr_un).cast(),
                std::mem::size_of_val(&address) as _,
            )
        };
        if result == 0 {
            return Ok(UnixStream::from(fd));
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EISCONN) => return Ok(UnixStream::from(fd)),
            Some(libc::EAGAIN | libc::EINPROGRESS | libc::EALREADY | libc::EINTR) => {
                wait::poll(py, raw, libc::POLLOUT, deadline, true)?;
            }
            _ => return Err(wait::runtime(error)),
        }
    }
}

pub(crate) fn query_value(py: Python<'_>, name: &str) -> PyResult<Value> {
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(wait::invalid(
            "query name must be one Hyprland JSON query name",
        ));
    }
    let response = request(py, &format!("j/{name}"))?;
    let value: Value = serde_json::from_str(&response)
        .map_err(|_| wait::runtime("Hyprland did not return JSON for this query"))?;
    if !value.is_object() && !value.is_array() {
        return Err(wait::runtime(
            "Hyprland query must return an object or array",
        ));
    }
    Ok(value)
}

/// Read one JSON query, such as clients, monitors, workspaces or cursorpos.
#[pyfunction]
#[pyo3(signature = (name, /))]
fn query(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
    logging::result(
        "hyprland.query",
        query_value(py, name).and_then(|value| {
            pythonize::pythonize(py, &value)
                .map(Bound::unbind)
                .map_err(Into::into)
        }),
    )
}

/// Execute a Lua dispatcher expression, for example hl.dsp.focus({ workspace = 3 }).
#[pyfunction]
#[pyo3(signature = (expression, /))]
fn dispatch(py: Python<'_>, expression: &str) -> PyResult<()> {
    let result = (|| {
        if expression.trim().is_empty() || expression.contains('\0') {
            return Err(wait::invalid("dispatch requires a nonempty Lua expression"));
        }
        let response = request(py, &format!("/dispatch {expression}"))?;
        if response.trim() != "ok" {
            return Err(wait::runtime(format!(
                "Hyprland dispatch failed: {}",
                response.trim()
            )));
        }
        Ok(())
    })();
    logging::result("hyprland.dispatch", result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let child = PyModule::new(module.py(), "computer_use.hyprland")?;
    child.add_function(wrap_pyfunction!(query, &child)?)?;
    child.add_function(wrap_pyfunction!(dispatch, &child)?)?;
    module.add("hyprland", &child)?;
    module
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("computer_use.hyprland", child)?;
    Ok(())
}

/// Retain compositor geometry in logical desktop units, including negative origins.
#[derive(Clone, Deserialize)]
pub(crate) struct Monitor {
    /// Hyprland connector name, matched against wl_output.name.
    pub name: String,

    /// Untransformed output mode width in pixels.
    pub width: u32,

    /// Untransformed output mode height in pixels.
    pub height: u32,

    /// Desktop logical left edge, which can be negative.
    pub x: i32,

    /// Desktop logical top edge, which can be negative.
    pub y: i32,

    /// Fractional compositor scale applied after orientation correction.
    pub scale: f64,

    /// Wayland output rotation/reflection value in 0..=7.
    pub transform: u32,

    /// Selects the default screenshot output at query time.
    pub focused: bool,

    /// Disabled outputs cannot be captured or used as pointer destinations.
    pub disabled: bool,
}

impl Monitor {
    pub(crate) fn bounds(&self) -> (f64, f64, f64, f64) {
        let (w, h) = if self.transform % 2 == 1 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        (
            self.x as f64,
            self.y as f64,
            (w as f64 / self.scale).round(),
            (h as f64 / self.scale).round(),
        )
    }
}

pub(crate) fn monitors(py: Python<'_>) -> PyResult<Vec<Monitor>> {
    let monitors: Vec<Monitor> =
        serde_json::from_value(query_value(py, "monitors")?).map_err(wait::runtime)?;
    let monitors: Vec<_> = monitors.into_iter().filter(|m| !m.disabled).collect();
    if monitors.iter().any(|m| {
        m.width == 0 || m.height == 0 || !m.scale.is_finite() || m.scale <= 0.0 || m.transform > 7
    }) {
        return Err(wait::runtime("Hyprland returned invalid output geometry"));
    }
    Ok(monitors)
}
