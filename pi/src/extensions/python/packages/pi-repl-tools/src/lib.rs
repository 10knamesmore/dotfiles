//! Provide native image display and namespace summaries for Pi's Python worker.

mod diagnostics;
mod display_image;
mod summarize;

use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Register helpers whose session bindings and output transport belong to the worker.
#[pymodule]
fn pi_repl_tools(module: &Bound<'_, PyModule>) -> PyResult<()> {
    display_image::register(module)?;
    summarize::register(module)?;
    Ok(())
}
