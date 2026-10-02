//! Native Chromium discovery, CDP sessions and page interaction for Python.

mod diagnostics;
mod discovery;
mod page;
mod runtime;
mod session;

use pyo3::prelude::*;

/// Register the SDK without opening sockets or starting a browser.
#[pymodule]
fn browser_use(module: &Bound<'_, PyModule>) -> PyResult<()> {
    discovery::register(module)?;
    session::register(module)?;
    page::register(module)?;
    module
        .py()
        .import("atexit")?
        .call_method1("register", (module.getattr("close_all")?,))?;
    Ok(())
}
