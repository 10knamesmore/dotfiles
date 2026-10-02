//! Map desktop window process IDs to Chromium profile roots without connecting to Chrome.

use std::path::PathBuf;

use anyhow::{Context, bail};
use pyo3::prelude::*;
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Describe one browser main process; it can own multiple desktop windows and tabs.
#[pyclass(frozen, get_all, skip_from_py_object, module = "browser_use")]
#[derive(Clone)]
pub(crate) struct BrowserInstance {
    /// Main browser PID accepted by connect(pid=...).
    pub pid: u32,

    /// Process name reported by the operating system.
    pub name: String,

    /// Absolute path to the browser executable.
    pub executable: String,

    /// Browser data root, not the Default or Profile N subdirectory.
    pub user_data_dir: String,
}

#[pymethods]
impl BrowserInstance {
    fn __repr__(&self) -> String {
        format!(
            "BrowserInstance(pid={}, name={:?}, user_data_dir={:?})",
            self.pid, self.name, self.user_data_dir
        )
    }
}

fn processes() -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always)
            .with_environ(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_user(UpdateKind::Always),
    );
    system
}

fn flag(process: &Process, name: &str) -> Option<String> {
    if name == "--user-data-dir" && process.cmd().len() == 1 {
        let title = process.cmd()[0].to_str()?;
        for separator in [" --user-data-dir=", " --user-data-dir "] {
            if let Some((_, remainder)) = title.split_once(separator) {
                let value = remainder.split(" --").next()?;
                // Flattened Chrome titles lose argv boundaries. The live profile
                // directory identifies where a path with spaces actually ends.
                for end in std::iter::once(value.len())
                    .chain(value.match_indices(' ').map(|(index, _)| index).rev())
                {
                    let candidate = &value[..end];
                    let path = PathBuf::from(candidate);
                    let path = if path.is_absolute() {
                        path
                    } else {
                        process.cwd()?.join(path)
                    };
                    if path.is_dir() {
                        return Some(candidate.to_owned());
                    }
                }
            }
        }
    }
    // Chrome can rewrite argv into one space-separated process title on Linux.
    // sysinfo retains that single string; normalize it as psutil.cmdline does.
    let arguments: Vec<&str> = if process.cmd().len() == 1 {
        process.cmd()[0]
            .to_str()?
            .split_ascii_whitespace()
            .collect()
    } else {
        process
            .cmd()
            .iter()
            .filter_map(|value| value.to_str())
            .collect()
    };
    arguments.iter().enumerate().find_map(|(index, value)| {
        value
            .strip_prefix(&format!("{name}="))
            .map(str::to_owned)
            .or_else(|| {
                (*value == name)
                    .then(|| arguments.get(index + 1).map(|value| (*value).to_owned()))
                    .flatten()
            })
    })
}

fn environment(process: &Process, key: &str) -> Option<String> {
    process.environ().iter().find_map(|value| {
        value
            .to_str()?
            .strip_prefix(&format!("{key}="))
            .map(str::to_owned)
    })
}

fn instance(process: &Process) -> Option<BrowserInstance> {
    // Discovery stays within the current user's applications.
    if process
        .user_id()
        .is_none_or(|uid| **uid != unsafe { libc::getuid() })
    {
        return None;
    }
    let name = process.name().to_str()?;
    let (linux, macos) = match name.to_ascii_lowercase().as_str() {
        "chrome" | "google chrome" => ("google-chrome", "Google/Chrome"),
        "google chrome beta" => ("google-chrome-beta", "Google/Chrome Beta"),
        "google chrome dev" => ("google-chrome-unstable", "Google/Chrome Dev"),
        "google chrome canary" => ("google-chrome-canary", "Google/Chrome Canary"),
        "chromium" => ("chromium", "Chromium"),
        "brave" | "brave browser" => ("BraveSoftware/Brave-Browser", "BraveSoftware/Brave-Browser"),
        "msedge" | "microsoft edge" => ("microsoft-edge", "Microsoft Edge"),
        _ => return None,
    };
    if flag(process, "--type").is_some() {
        return None;
    }
    let executable = process.exe()?;
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let explicit = flag(process, "--user-data-dir").or_else(|| {
        if cfg!(target_os = "linux") {
            environment(process, "CHROME_USER_DATA_DIR")
        } else {
            None
        }
    });
    let directory = if let Some(path) = explicit {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else {
            process.cwd()?.join(path)
        }
    } else if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(macos)
    } else {
        let root = environment(process, "CHROME_CONFIG_HOME")
            .or_else(|| environment(process, "XDG_CONFIG_HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let channel = executable.parent()?.file_name()?.to_str()?;
        root.join(match (name, channel) {
            ("chrome", "chrome-beta") => "google-chrome-beta",
            ("chrome", "chrome-unstable") => "google-chrome-unstable",
            _ => linux,
        })
    };
    Some(BrowserInstance {
        pid: process.pid().as_u32(),
        name: name.to_owned(),
        executable: executable.to_string_lossy().into_owned(),
        user_data_dir: directory.to_string_lossy().into_owned(),
    })
}

/// List running browsers without opening a remote-debugging permission dialog.
#[pyfunction]
fn discover(py: Python<'_>) -> Vec<BrowserInstance> {
    py.detach(|| {
        let system = processes();
        let mut browsers: Vec<_> = system.processes().values().filter_map(instance).collect();
        browsers.sort_by_key(|browser| browser.pid);
        browsers
    })
}

/// Resolve a window or renderer PID and read the owning browser's current CDP endpoint.
pub(crate) fn resolve(pid: u32) -> anyhow::Result<(BrowserInstance, String)> {
    let system = processes();
    let mut current = system.process(Pid::from_u32(pid));
    while let Some(process) = current {
        if let Some(browser) = instance(process) {
            let port_file = PathBuf::from(&browser.user_data_dir).join("DevToolsActivePort");
            let endpoint = match std::fs::read_to_string(&port_file) {
                Ok(contents) => parse_endpoint(&contents)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if let Some(port) = flag(process, "--remote-debugging-port")
                        .and_then(|port| port.parse::<u16>().ok())
                        .filter(|port| *port != 0)
                    {
                        format!("http://127.0.0.1:{port}")
                    } else {
                        bail!(
                            "PID {} has no CDP endpoint. Open chrome://inspect/#remote-debugging in Chrome and enable remote debugging, then connect again. An independently started browser needs --remote-debugging-port and a separate --user-data-dir.",
                            browser.pid
                        );
                    }
                }
                Err(error) => return Err(error.into()),
            };
            return Ok((browser, endpoint));
        }
        current = process.parent().and_then(|parent| system.process(parent));
    }
    bail!("PID {pid} does not belong to a supported Chromium browser")
}

pub(crate) fn parse_endpoint(contents: &str) -> anyhow::Result<String> {
    let mut lines = contents.lines();
    let port: u16 = lines
        .next()
        .context("Missing Chrome debugging port")?
        .parse()?;
    let path = lines
        .next()
        .context("Missing Chrome debugging WebSocket path")?;
    if port == 0 || !path.starts_with("/devtools/browser") {
        bail!("Invalid Chrome endpoint file");
    }
    Ok(format!("ws://127.0.0.1:{port}{path}"))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<BrowserInstance>()?;
    module.add_function(wrap_pyfunction!(discover, module)?)?;
    Ok(())
}
