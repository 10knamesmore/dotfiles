//! A subprocess supervisor owns background processes even if the Python worker is killed.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use pyo3::prelude::*;
use serde_json::{Value, json};

use super::{Control, Target};
use crate::{hyprland, logging, wait};

pub(super) struct Background {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    incoming: Vec<u8>,
}

impl Background {
    pub fn start(control: &Control, python: &str, size: (u32, u32)) -> PyResult<(Self, Target)> {
        logging::event("background.start", "starting");
        let mut child = Command::new(python)
            .args([
                "-u",
                "-c",
                include_str!("supervisor.py"),
                &size.0.to_string(),
                &size.1.to_string(),
            ])
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(wait::runtime)?;
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let mut background = Self {
            child,
            input,
            output,
            incoming: Vec::new(),
        };
        for fd in [
            background.input.as_ref().unwrap().as_raw_fd(),
            background.output.as_raw_fd(),
        ] {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags == -1
                || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1
            {
                return Err(wait::runtime(std::io::Error::last_os_error()));
            }
        }
        let value = background.response(control, Instant::now() + Duration::from_secs(25))?;
        let field = |name| {
            value
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| wait::runtime("background supervisor returned an invalid target"))
        };
        let target = Target {
            runtime: PathBuf::from(field("runtime")?),
            display: field("display")?,
            signature: field("signature")?,
        };
        // IPC can be listening while the nested output still has zero geometry.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let monitors = hyprland::query_value(control, &target, "monitors")?;
            if monitors.as_array().is_some_and(|monitors| {
                monitors.iter().any(|monitor| {
                    monitor.get("width").and_then(Value::as_u64) == Some(size.0.into())
                        && monitor.get("height").and_then(Value::as_u64) == Some(size.1.into())
                        && monitor.get("focused").and_then(Value::as_bool) == Some(true)
                })
            }) {
                break;
            }
            if Instant::now() >= deadline || !background.alive()? {
                return Err(wait::runtime("background output did not become ready"));
            }
            wait::pause(control, Duration::from_millis(20))?;
        }
        logging::event("background.start", "ready");
        Ok((background, target))
    }

    pub fn alive(&mut self) -> PyResult<bool> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(wait::runtime)
    }

    pub fn launch(
        &mut self,
        control: &Control,
        argv: Vec<String>,
        cwd: Option<String>,
    ) -> PyResult<u32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut bytes = serde_json::to_vec(&json!({"type":"launch", "argv":argv, "cwd":cwd}))
            .map_err(wait::runtime)?;
        bytes.push(b'\n');
        let input = self.input.as_mut().unwrap();
        let mut slice = bytes.as_slice();
        while !slice.is_empty() {
            control.check()?;
            match input.write(slice) {
                Ok(0) => return Err(wait::runtime("background supervisor closed")),
                Ok(count) => slice = &slice[count..],
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    wait::poll(control, input.as_raw_fd(), libc::POLLOUT, deadline, true)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(wait::runtime(error)),
            }
        }
        let response = self.response(control, deadline)?;
        response
            .get("pid")
            .and_then(Value::as_u64)
            .map(|pid| pid as u32)
            .ok_or_else(|| wait::runtime("background supervisor returned an invalid process id"))
    }

    fn response(&mut self, control: &Control, deadline: Instant) -> PyResult<Value> {
        loop {
            control.check()?;
            if let Some(end) = self.incoming.iter().position(|byte| *byte == b'\n') {
                let line: Vec<_> = self.incoming.drain(..=end).collect();
                let value: Value = serde_json::from_slice(&line).map_err(wait::runtime)?;
                if let Some(error) = value.get("error").and_then(Value::as_str) {
                    return Err(wait::runtime(error));
                }
                return Ok(value);
            }
            let mut chunk = [0; 4096];
            match self.output.read(&mut chunk) {
                Ok(0) => return Err(wait::runtime("background supervisor exited")),
                Ok(count) => self.incoming.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    wait::poll(
                        control,
                        self.output.as_raw_fd(),
                        libc::POLLIN,
                        deadline,
                        true,
                    )?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(wait::runtime(error)),
            }
        }
    }
}

impl Drop for Background {
    fn drop(&mut self) {
        // EOF is also delivered by the kernel on SIGKILL of the worker; no Python GC is involved.
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    logging::event(
                        "background.close",
                        if status.success() {
                            "reaped"
                        } else {
                            "supervisor-failed"
                        },
                    );
                    break;
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    logging::event("background.close", "supervisor-unresponsive");
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break;
                }
            }
        }
    }
}
