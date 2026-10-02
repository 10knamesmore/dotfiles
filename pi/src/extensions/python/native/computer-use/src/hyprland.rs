//! Query one fixed Hyprland instance through interruptible Unix socket requests.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use pyo3::prelude::*;
use serde::Deserialize;
use serde_json::Value;

use crate::desktop::{Control, Target};
use crate::wait;

fn request(control: &Control, target: &Target, command: &str) -> PyResult<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let result = (|| {
        let mut socket = socket_connect(control, &target.hypr_socket(), deadline)?;
        let mut bytes = command.as_bytes();
        while !bytes.is_empty() {
            control.check()?;
            match socket.write(bytes) {
                Ok(0) => return Err(wait::runtime("Hyprland closed the request socket")),
                Ok(count) => bytes = &bytes[count..],
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    wait::poll(control, socket.as_raw_fd(), libc::POLLOUT, deadline, true)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(wait::runtime(error)),
            }
        }
        let mut response = Vec::new();
        loop {
            control.check()?;
            let mut chunk = [0; 16384];
            match socket.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => response.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    wait::poll(control, socket.as_raw_fd(), libc::POLLIN, deadline, true)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(wait::runtime(error)),
            }
            if Instant::now() >= deadline {
                return Err(wait::runtime("Hyprland request timed out"));
            }
        }
        String::from_utf8(response).map_err(wait::runtime)
    })();
    if result.is_err() {
        control.disconnect();
    }
    result
}

pub(crate) fn socket_connect(
    control: &Control,
    path: &std::path::Path,
    deadline: Instant,
) -> PyResult<UnixStream> {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();
    // The address is zero-filled and the path leaves space for its NUL terminator.
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= address.sun_path.len() {
        return Err(wait::runtime("desktop socket path is too long"));
    }
    address.sun_family = libc::AF_UNIX as _;
    for (target, source) in address.sun_path.iter_mut().zip(bytes) {
        *target = *source as _;
    }
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
    // Ownership starts before any fallible connection attempt.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    loop {
        control.check()?;
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
                wait::poll(control, raw, libc::POLLOUT, deadline, true)?;
            }
            _ => return Err(wait::runtime(error)),
        }
    }
}

pub(crate) fn query_value(control: &Control, target: &Target, name: &str) -> PyResult<Value> {
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(wait::invalid(
            "query name must be one Hyprland JSON query name",
        ));
    }
    let response = request(control, target, &format!("j/{name}"))?;
    let value: Value = serde_json::from_str(&response)
        .map_err(|_| wait::runtime("Hyprland did not return JSON for this query"))?;
    if !value.is_object() && !value.is_array() {
        return Err(wait::runtime(
            "Hyprland query must return an object or array",
        ));
    }
    Ok(value)
}

pub(crate) fn dispatch(control: &Control, target: &Target, expression: &str) -> PyResult<()> {
    if expression.trim().is_empty() || expression.contains('\0') {
        return Err(wait::invalid("dispatch requires a nonempty Lua expression"));
    }
    let response = request(control, target, &format!("/dispatch {expression}"))?;
    if response.trim() != "ok" {
        return Err(wait::runtime(format!(
            "Hyprland dispatch failed: {}",
            response.trim()
        )));
    }
    Ok(())
}

/// Output geometry in desktop logical coordinates; capture pixels retain the native mode.
#[derive(Clone, Deserialize)]
pub(crate) struct Monitor {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub focused: bool,
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

pub(crate) fn monitors(control: &Control, target: &Target) -> PyResult<Vec<Monitor>> {
    let monitors: Vec<Monitor> =
        serde_json::from_value(query_value(control, target, "monitors")?).map_err(wait::runtime)?;
    let monitors: Vec<_> = monitors.into_iter().filter(|m| !m.disabled).collect();
    if monitors.iter().any(|m| {
        m.width == 0 || m.height == 0 || !m.scale.is_finite() || m.scale <= 0.0 || m.transform > 7
    }) {
        return Err(wait::runtime("Hyprland returned invalid output geometry"));
    }
    Ok(monitors)
}
