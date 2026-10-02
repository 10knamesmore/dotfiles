//! Native Linux Hyprland capture, input and structured compositor access for Python.

#[cfg(not(target_os = "linux"))]
compile_error!("computer-use-sdk requires Linux; skip this package on macOS");

mod capture;
mod hyprland;
mod input;
mod logging;
mod wait;
mod wayland;

use pyo3::prelude::*;

/// Register the native SDK without connecting to Wayland or Hyprland.
#[pymodule]
fn computer_use(module: &Bound<'_, PyModule>) -> PyResult<()> {
    input::mark_owner();
    capture::register(module)?;
    input::register(module)?;
    hyprland::register(module)?;
    module
        .py()
        .import("atexit")?
        .call_method1("register", (module.getattr("close")?,))?;
    Ok(())
}
