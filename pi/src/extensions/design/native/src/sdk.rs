use anyhow::{Context, Result, bail};
use pyo3::{
    exceptions::{PyKeyError, PyRuntimeError, PyValueError},
    prelude::*,
    types::{PyDict, PyList},
};
use reqwest::{Method, blocking::Client};
use serde_json::{Value, json};
use std::{env, fs, path::PathBuf, time::Duration};

pub fn python_error(error: anyhow::Error) -> PyErr {
    PyRuntimeError::new_err(format!("{error:#}"))
}

fn endpoint(explicit: Option<String>) -> Result<String> {
    let endpoint = if let Some(endpoint) = explicit {
        endpoint
    } else {
        let session = env::var("PI_PYTHON_SESSION_ID")
            .context("PI_PYTHON_SESSION_ID is not set; pass endpoint= for standalone access")?;
        if session.is_empty() || session.contains(['/', '\\']) || session == "." || session == ".."
        {
            bail!("PI_PYTHON_SESSION_ID is not a valid session filename");
        }
        let agent_dir = env::var_os("PI_DESIGN_AGENT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".pi/agent")
            });
        let file = agent_dir
            .join("design/sessions")
            .join(format!("{session}.json"));
        let config: Value = serde_json::from_slice(
            &fs::read(&file).with_context(|| format!("Read design endpoint {}", file.display()))?,
        )?;
        config["url"]
            .as_str()
            .context("Design endpoint file has no string url")?
            .to_owned()
    };
    let endpoint = endpoint.trim_end_matches('/').to_owned();
    let parsed = reqwest::Url::parse(&endpoint).context("Invalid design endpoint URL")?;
    if parsed.scheme() != "http" {
        bail!("Design endpoint must be an http:// URL");
    }
    Ok(endpoint)
}

fn request(
    endpoint: &str,
    method: Method,
    route: &str,
    body: Option<&Value>,
    project: Option<&str>,
) -> Result<Value> {
    request_at_revision(endpoint, method, route, body, project, None)
}

fn request_at_revision(
    endpoint: &str,
    method: Method,
    route: &str,
    body: Option<&Value>,
    project: Option<&str>,
    revision: Option<u64>,
) -> Result<Value> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .no_proxy()
        .build()?;
    let mut request = client.request(method, format!("{endpoint}{route}"));
    if let Some(project) = project {
        request = request.header("x-pi-design-project", project);
    }
    if let Some(revision) = revision {
        request = request.header("x-pi-design-revision", revision);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().context("Connect to Pi Design")?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("Pi Design returned {status}: {text}"))?;
    if !status.is_success() {
        bail!("{}", value["error"].as_str().unwrap_or(&text));
    }
    Ok(value)
}

fn python_json(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    Ok(match value {
        Value::Null => py.None(),
        Value::Bool(value) => value.into_pyobject(py)?.to_owned().into_any().unbind(),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                value.into_pyobject(py)?.into_any().unbind()
            } else if let Some(value) = value.as_u64() {
                value.into_pyobject(py)?.into_any().unbind()
            } else {
                value
                    .as_f64()
                    .unwrap()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()
            }
        }
        Value::String(value) => value.into_pyobject(py)?.into_any().unbind(),
        Value::Array(values) => {
            let list = PyList::empty(py);
            for value in values {
                list.append(python_json(py, value)?)?;
            }
            list.into_any().unbind()
        }
        Value::Object(values) => {
            let dict = PyDict::new(py);
            for (key, value) in values {
                dict.set_item(key, python_json(py, value)?)?;
            }
            dict.into_any().unbind()
        }
    })
}

#[pyclass(frozen, module = "pi_design")]
pub struct Project {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    path: String,
    #[pyo3(get)]
    url: String,
}

impl Project {
    fn from_state(state: Value, url: String) -> Result<Self> {
        let project = &state["project"];
        if project.is_null() {
            bail!("No active project; call pi_design.open(path) first");
        }
        Ok(Self {
            id: project["id"]
                .as_str()
                .context("State project has no id")?
                .to_owned(),
            path: project["path"]
                .as_str()
                .context("State project has no path")?
                .to_owned(),
            url,
        })
    }

    fn request(&self, method: Method, route: &str, body: Option<&Value>) -> Result<Value> {
        request(&self.url, method, route, body, Some(&self.id))
    }
}

#[pymethods]
impl Project {
    fn __repr__(&self) -> String {
        format!(
            "Project(id={:?}, path={:?}, url={:?})",
            self.id, self.path, self.url
        )
    }

    fn read_state(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let state = py
            .detach(|| self.request(Method::GET, "/api/state", None))
            .map_err(python_error)?;
        python_json(py, &state)
    }

    #[pyo3(signature = (since=0))]
    fn read_changes(&self, py: Python<'_>, since: u64) -> PyResult<Py<PyAny>> {
        let changes = py
            .detach(|| self.request(Method::GET, &format!("/api/changes?since={since}"), None))
            .map_err(python_error)?;
        python_json(py, &changes)
    }

    fn read_element(&self, py: Python<'_>, element: &str) -> PyResult<Py<PyAny>> {
        let state = py
            .detach(|| self.request(Method::GET, "/api/state", None))
            .map_err(python_error)?;
        let value = state["design"]["elements"]
            .get(element)
            .ok_or_else(|| PyKeyError::new_err(element.to_owned()))?;
        python_json(py, value)
    }

    fn read_artboard(&self, py: Python<'_>, artboard: &str) -> PyResult<Py<PyAny>> {
        let state = py
            .detach(|| self.request(Method::GET, "/api/state", None))
            .map_err(python_error)?;
        let value = state["workspace"]["artboards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["id"] == artboard)
            .ok_or_else(|| PyKeyError::new_err(artboard.to_owned()))?;
        python_json(py, value)
    }

    fn resolve_comment(&self, py: Python<'_>, comment: &str) -> PyResult<Py<PyAny>> {
        let value = py
            .detach(|| {
                let state = self.request(Method::GET, "/api/state", None)?;
                let mut comment = state["workspace"]["comments"]
                    .as_array()
                    .context("State has no comments")?
                    .iter()
                    .find(|value| value["id"] == comment)
                    .cloned()
                    .context("Unknown comment")?;
                comment["resolved"] = json!(true);
                request_at_revision(
                    &self.url,
                    Method::POST,
                    "/api/edit",
                    Some(&json!({
                        "edits": [{"type": "comment", "comment": comment}]
                    })),
                    Some(&self.id),
                    state["revision"].as_u64(),
                )
            })
            .map_err(python_error)?;
        python_json(py, &value)
    }

    fn undo(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let value = py
            .detach(|| self.request(Method::POST, "/api/undo", None))
            .map_err(python_error)?;
        python_json(py, &value)
    }

    fn redo(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let value = py
            .detach(|| self.request(Method::POST, "/api/redo", None))
            .map_err(python_error)?;
        python_json(py, &value)
    }
}

#[pyfunction]
#[pyo3(signature = (*, endpoint=None))]
fn list_projects(py: Python<'_>, endpoint: Option<String>) -> PyResult<Py<PyAny>> {
    let value = py
        .detach(|| {
            request(
                &self::endpoint(endpoint)?,
                Method::GET,
                "/api/projects",
                None,
                None,
            )
        })
        .map_err(python_error)?;
    python_json(py, &value["projects"])
}

#[pyfunction]
#[pyo3(signature = (path, *, name=None, start_preview=true, endpoint=None))]
fn open(
    py: Python<'_>,
    path: PathBuf,
    name: Option<String>,
    start_preview: bool,
    endpoint: Option<String>,
) -> PyResult<Project> {
    let path = path
        .canonicalize()
        .map_err(|error| PyValueError::new_err(format!("Open {}: {error}", path.display())))?;
    py.detach(|| {
        let endpoint = self::endpoint(endpoint)?;
        let mut body = json!({"path": path, "start_preview": start_preview});
        if let Some(name) = name {
            body["name"] = json!(name);
        }
        let state = request(&endpoint, Method::POST, "/api/open", Some(&body), None)?;
        Project::from_state(state, endpoint)
    })
    .map_err(python_error)
}

#[pyfunction]
#[pyo3(signature = (*, endpoint=None))]
fn current(py: Python<'_>, endpoint: Option<String>) -> PyResult<Project> {
    py.detach(|| {
        let endpoint = self::endpoint(endpoint)?;
        let state = request(&endpoint, Method::GET, "/api/state", None, None)?;
        Project::from_state(state, endpoint)
    })
    .map_err(python_error)
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Project>()?;
    module.add_function(wrap_pyfunction!(list_projects, module)?)?;
    module.add_function(wrap_pyfunction!(open, module)?)?;
    module.add_function(wrap_pyfunction!(current, module)?)?;
    Ok(())
}
