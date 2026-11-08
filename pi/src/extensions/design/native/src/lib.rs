mod actor;
mod documents;
mod history;
mod preview;
mod registry;
mod sdk;
mod service;

use pyo3::prelude::*;
use std::path::PathBuf;

#[pyfunction]
fn serve(py: Python<'_>, agent_dir: PathBuf, web_dir: PathBuf, session_id: String) -> PyResult<()> {
    py.detach(move || service::serve(agent_dir, web_dir, session_id))
        .map_err(sdk::python_error)
}

#[pymodule]
fn pi_design(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(serve, module)?)?;
    sdk::register(module)
}
