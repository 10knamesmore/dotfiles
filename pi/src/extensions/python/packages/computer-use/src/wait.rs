//! Bound socket waits and delays so Python cancellation can run between slices.

use std::os::fd::RawFd;
use std::time::{Duration, Instant};

use pyo3::exceptions::{PyRuntimeError, PyTimeoutError, PyValueError};
use pyo3::prelude::*;

pub(crate) fn runtime(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

pub(crate) fn invalid(message: impl Into<String>) -> PyErr {
    PyValueError::new_err(message.into())
}

pub(crate) fn seconds(value: f64, name: &str) -> PyResult<Duration> {
    Duration::try_from_secs_f64(value)
        .map_err(|_| invalid(format!("{name} must be finite and non-negative")))
}

pub(crate) fn pause(control: &crate::desktop::Control, duration: Duration) -> PyResult<()> {
    let deadline = Instant::now()
        .checked_add(duration)
        .ok_or_else(|| invalid("duration is too large"))?;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        control.check()?;
        std::thread::sleep(remaining.min(Duration::from_millis(20)));
    }
    control.check()
}

/// Run pixel processing off the Python thread and check cancellation while waiting.
pub(crate) fn compute<T: Send + 'static>(
    py: Python<'_>,
    task: impl FnOnce() -> PyResult<T> + Send + 'static,
) -> PyResult<T> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let receiver = std::sync::Mutex::new(receiver);
    std::thread::Builder::new()
        .name("computer-use-image".into())
        .spawn(move || {
            let _ = sender.send(task());
        })
        .map_err(runtime)?;
    loop {
        py.check_signals()?;
        match py.detach(|| {
            receiver
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_millis(25))
        }) {
            Ok(result) => {
                py.check_signals()?;
                return result;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(runtime("image processing stopped unexpectedly"));
            }
        }
    }
}

pub(crate) fn poll(
    control: &crate::desktop::Control,
    fd: RawFd,
    events: i16,
    deadline: Instant,
    signals: bool,
) -> PyResult<bool> {
    if signals {
        control.check()?;
    }
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| PyTimeoutError::new_err("desktop connection timed out"))?;
    let timeout = remaining.as_millis().clamp(1, 25) as i32;
    let result = (|| {
        let mut descriptor = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        // The descriptor stays owned by the caller throughout this bounded poll.
        let ready = unsafe { libc::poll(&mut descriptor, 1, timeout) };
        if ready < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok((ready, descriptor.revents))
        }
    })();
    if signals {
        control.check()?;
    }
    match result {
        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => Ok(false),
        Err(error) => Err(runtime(error)),
        Ok((_, flags)) if flags & libc::POLLNVAL != 0 => Err(runtime("desktop socket is closed")),
        Ok((ready, _)) => Ok(ready > 0),
    }
}
