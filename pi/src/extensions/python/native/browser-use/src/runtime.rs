//! Execute CDP futures off the Python thread while keeping cancellation responsive.

use std::future::Future;
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use pyo3::exceptions::{PyRuntimeError, PyTimeoutError, PyValueError};
use pyo3::prelude::*;
use tokio::runtime::Runtime;

use crate::diagnostics;

pub(crate) fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("browser-use")
            .build()
            .expect("create browser runtime")
    })
}

pub(crate) fn seconds(value: f64) -> PyResult<Duration> {
    let duration = Duration::try_from_secs_f64(value)
        .map_err(|_| PyValueError::new_err("timeout must be finite and positive"))?;
    if duration.is_zero() {
        return Err(PyValueError::new_err("timeout must be finite and positive"));
    }
    Ok(duration)
}

pub(crate) fn error(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

/// Abort the pending future on timeout or Python interruption; sent browser actions cannot be undone.
pub(crate) fn run<T: Send + 'static>(
    py: Python<'_>,
    name: &'static str,
    timeout: Duration,
    future: impl Future<Output = anyhow::Result<T>> + Send + 'static,
) -> PyResult<T> {
    let started = Instant::now();
    let (sender, receiver) = mpsc::sync_channel(1);
    let receiver = Mutex::new(receiver);
    diagnostics::event(name, "started");
    let task = runtime().spawn(async move {
        let result = tokio::time::timeout(timeout, future).await;
        let _ = sender.send(result);
    });
    struct AbortOnDrop(tokio::task::JoinHandle<()>);
    impl Drop for AbortOnDrop {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _task = AbortOnDrop(task);
    let result = (|| loop {
        py.check_signals()?;
        match py.detach(|| {
            receiver
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_millis(25))
        }) {
            Ok(result) => {
                py.check_signals()?;
                break match result {
                    Ok(value) => value.map_err(error),
                    Err(_) => Err(PyTimeoutError::new_err(format!(
                        "{name} timed out after {:.3} seconds",
                        timeout.as_secs_f64()
                    ))),
                };
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break Err(error("browser operation stopped unexpectedly"));
            }
        }
    })();
    let outcome = match &result {
        Ok(_) => "completed".to_owned(),
        Err(error) => format!("failed type={}", error.get_type(py).name()?),
    };
    diagnostics::event(
        name,
        &format!("{outcome} duration_ms={}", started.elapsed().as_millis()),
    );
    result
}
