//! Native Linux Hyprland capture, input and structured compositor access for Python.

#[cfg(not(target_os = "linux"))]
compile_error!("computer-use-sdk requires Linux; skip this package on macOS");

mod capture;
mod desktop;
mod hyprland;
mod input;
mod logging;
mod wait;
mod wayland;

use pyo3::prelude::*;

/// Register the native SDK without connecting to Wayland or Hyprland.
#[pymodule]
fn computer_use(module: &Bound<'_, PyModule>) -> PyResult<()> {
    capture::register(module)?;
    desktop::register(module)?;
    module
        .py()
        .import("atexit")?
        .call_method1("register", (module.getattr("_close_all")?,))?;
    Ok(())
}
