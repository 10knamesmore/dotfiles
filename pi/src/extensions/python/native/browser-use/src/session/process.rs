//! Supervise SDK-launched Chrome in a separate process group, surviving cell interrupts.

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use anyhow::{Context, bail};

use crate::discovery;

pub(super) struct OwnedBrowser {
    /// Process handle is taken once when reaping, so a recycled PID cannot be signalled.
    child: Mutex<Option<Child>>,

    /// Group leader PID reported immediately after spawning, before connecting CDP.
    pub pid: u32,

    /// Profile root used to find this process's DevToolsActivePort file.
    pub directory: PathBuf,

    /// Previous run's endpoint must not be mistaken for the new process being ready.
    previous_endpoint: Option<String>,

    /// Owns only temporary profiles; explicit user directories are never removed.
    _profile: Option<tempfile::TempDir>,
}

impl OwnedBrowser {
    pub fn spawn(
        executable: Option<PathBuf>,
        directory: Option<PathBuf>,
        headless: bool,
    ) -> anyhow::Result<Self> {
        let profile = if directory.is_none() {
            Some(tempfile::Builder::new().prefix("browser-use-").tempdir()?)
        } else {
            None
        };
        let directory = directory.unwrap_or_else(|| profile.as_ref().unwrap().path().to_owned());
        std::fs::create_dir_all(&directory)?;
        let directory = std::fs::canonicalize(directory)?;
        let previous_endpoint = std::fs::read_to_string(directory.join("DevToolsActivePort")).ok();
        let executable = executable.map(Ok).unwrap_or_else(|| {
            chromiumoxide::detection::default_executable(Default::default())
                .map_err(anyhow::Error::msg)
        })?;
        let mut command = Command::new(executable);
        command
            .args([
                "--remote-debugging-port=0",
                "--remote-debugging-address=127.0.0.1",
                "--no-first-run",
                "--no-default-browser-check",
            ])
            .arg(format!("--user-data-dir={}", directory.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        if headless {
            command.arg("--headless=new");
        }
        command.arg("about:blank");
        let child = command.spawn().context("Could not launch Chrome")?;
        let pid = child.id();
        Ok(Self {
            child: Mutex::new(Some(child)),
            pid,
            directory,
            previous_endpoint,
            _profile: profile,
        })
    }

    pub async fn endpoint(&self) -> anyhow::Result<String> {
        loop {
            if let Some(status) = self
                .child
                .lock()
                .unwrap()
                .as_mut()
                .context("Browser process is closed")?
                .try_wait()?
            {
                bail!("Chrome exited before its debugging endpoint was ready: {status}");
            }
            match std::fs::read_to_string(self.directory.join("DevToolsActivePort")) {
                Ok(contents) if Some(&contents) != self.previous_endpoint.as_ref() => {
                    return discovery::parse_endpoint(&contents);
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    }

    /// Reap an exited process or terminate its group once; attachments never own this object.
    pub fn terminate(&self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.lock().unwrap().take()
            && child.try_wait()?.is_none()
        {
            // The owned Chrome is the group leader created by process_group(0).
            let result = unsafe { libc::kill(-(self.pid as i32), libc::SIGKILL) };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error.into());
                }
            }
            child.wait()?;
        }
        Ok(())
    }

    pub fn exited(&self) -> anyhow::Result<bool> {
        let mut child = self.child.lock().unwrap();
        match child.as_mut() {
            Some(child) => Ok(child.try_wait()?.is_some()),
            None => Ok(true),
        }
    }
}

impl Drop for OwnedBrowser {
    fn drop(&mut self) {
        if self.terminate().is_err() {
            crate::diagnostics::event("browser_process_cleanup", "failed");
        }
    }
}
