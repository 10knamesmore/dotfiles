//! Desktop ownership and the host's continuously monitored control channel.

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use pyo3::prelude::*;
use serde_json::{Value, json};

use crate::{hyprland, logging, wait};

pub(super) const ACTIVE: u8 = 0;
pub(super) const CLOSED: u8 = 1;
const REVOKED: u8 = 2;
const DISCONNECTED: u8 = 3;

/// Share terminal lifecycle state without exposing the execution thread's protocol objects.
pub(super) struct Shared {
    /// The first transition away from ACTIVE wins; close does not overwrite why control was lost.
    pub status: AtomicU8,
}

impl Shared {
    pub fn status(&self) -> &'static str {
        match self.status.load(Ordering::Acquire) {
            ACTIVE => "active",
            CLOSED => "closed",
            REVOKED => "revoked",
            _ => "disconnected",
        }
    }

    pub fn invalidate(&self, state: u8) {
        if self
            .status
            .compare_exchange(ACTIVE, state, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            logging::event("desktop.state", self.status());
        }
    }

    pub fn check(&self) -> PyResult<()> {
        if self.status.load(Ordering::Acquire) == ACTIVE {
            Ok(())
        } else {
            Err(wait::runtime(format!(
                "desktop is {}; create a new desktop explicitly",
                self.status()
            )))
        }
    }
}

/// Freeze Wayland and Hyprland addressing together so later environment changes cannot reroute input.
#[derive(Clone)]
pub(crate) struct Target {
    /// Runtime root containing Wayland sockets and Hyprland instance directories.
    pub runtime: PathBuf,

    /// Wayland socket name or absolute path selected when this desktop opens.
    pub display: String,

    /// Hyprland instance whose monitor geometry and dispatcher match that Wayland connection.
    pub signature: String,
}

impl Target {
    pub(super) fn host() -> PyResult<Self> {
        let required =
            |name| std::env::var(name).map_err(|_| wait::runtime(format!("{name} is required")));
        Ok(Self {
            runtime: PathBuf::from(required("XDG_RUNTIME_DIR")?),
            display: required("WAYLAND_DISPLAY")?,
            signature: required("HYPRLAND_INSTANCE_SIGNATURE")?,
        })
    }

    pub fn instance_dir(&self) -> PathBuf {
        self.runtime.join("hypr").join(&self.signature)
    }
    pub fn hypr_socket(&self) -> PathBuf {
        self.instance_dir().join(".socket.sock")
    }
}

/// Accessed only on the native desktop thread, including between Python calls.
pub(crate) struct Control {
    /// Lifecycle state visible to Python handles and the executor.
    pub(super) shared: Arc<Shared>,

    /// Cancellation flag for the currently executing Python request, absent while idle.
    pub(super) cancel: RefCell<Option<Arc<AtomicBool>>>,

    /// Host-only overlay channel; both EOF and explicit revoke end control.
    prompt: RefCell<Option<Prompt>>,

    /// Host-only flock ownership, retained until all input cleanup has finished.
    lock: Option<File>,
}

impl Control {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            cancel: RefCell::new(None),
            prompt: RefCell::new(None),
            lock: None,
        }
    }

    pub(super) fn connect_host(&mut self, target: &Target) -> PyResult<()> {
        let mut lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC)
            .open(target.instance_dir().join("computer-use.lock"))
            .map_err(wait::runtime)?;
        // Each connection opens its own description: even this process's second connection is busy.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            return Err(wait::runtime(
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    "host desktop is busy".to_string()
                } else {
                    error.to_string()
                },
            ));
        }
        lock.set_len(0).map_err(wait::runtime)?;
        writeln!(lock, "{}", std::process::id()).map_err(wait::runtime)?;
        self.lock = Some(lock);
        let deadline = Instant::now() + Duration::from_secs(5);
        let socket = hyprland::socket_connect(
            self,
            &target.instance_dir().join("computer-use.sock"),
            deadline,
        )?;
        let mut prompt = Prompt {
            socket,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        };
        prompt
            .send(json!({"type":"hello", "pid":std::process::id()}))
            .map_err(wait::runtime)?;
        loop {
            self.check()?;
            prompt.flush().map_err(wait::runtime)?;
            if let Some(frame) = prompt.read().map_err(wait::runtime)? {
                match frame.get("type").and_then(Value::as_str) {
                    Some("ready") => break,
                    Some("busy") => return Err(wait::runtime("host desktop is busy")),
                    Some("revoke") => {
                        self.shared.invalidate(REVOKED);
                        return self.shared.check();
                    }
                    _ => return Err(wait::runtime("invalid host control handshake")),
                }
            }
            wait::poll(
                self,
                prompt.socket.as_raw_fd(),
                libc::POLLIN
                    | if prompt.outgoing.is_empty() {
                        0
                    } else {
                        libc::POLLOUT
                    },
                deadline,
                true,
            )?;
        }
        *self.prompt.borrow_mut() = Some(prompt);
        logging::event("host.connect", "ready");
        Ok(())
    }

    pub fn check(&self) -> PyResult<()> {
        self.shared.check()?;
        if let Some(prompt) = self.prompt.borrow_mut().as_mut() {
            let result = (|| {
                prompt.flush()?;
                while let Some(frame) = prompt.read()? {
                    if frame.get("type").and_then(Value::as_str) == Some("revoke") {
                        self.shared.invalidate(REVOKED);
                        logging::event("host.revoke", "received");
                    } else {
                        return Err(std::io::Error::other("unexpected host control frame"));
                    }
                }
                Ok(())
            })();
            if result.is_err() {
                self.disconnect();
            }
        }
        self.shared.check()?;
        if self
            .cancel
            .borrow()
            .as_ref()
            .is_some_and(|cancel| cancel.load(Ordering::Acquire))
        {
            return Err(wait::runtime("desktop operation was cancelled"));
        }
        Ok(())
    }

    pub fn event(&self, frame: Value) -> PyResult<()> {
        if let Some(prompt) = self.prompt.borrow_mut().as_mut()
            && let Err(error) = prompt.send(frame)
        {
            self.disconnect();
            return Err(wait::runtime(error));
        }
        Ok(())
    }

    pub fn disconnect(&self) {
        self.shared.invalidate(DISCONNECTED);
    }

    /// Called only after input releases and Wayland device destruction.
    pub(super) fn close(&mut self) {
        if let Some(mut prompt) = self.prompt.borrow_mut().take() {
            let _ = prompt.send(json!({"type":"close"}));
            logging::event("host.disconnect", "closed");
        }
        // Never unlink: replacing the inode could let two controllers acquire different locks.
        self.lock.take();
    }
}

struct Prompt {
    socket: UnixStream,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
}

impl Prompt {
    fn send(&mut self, frame: Value) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.outgoing, &frame)?;
        self.outgoing.push(b'\n');
        self.flush()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        while !self.outgoing.is_empty() {
            match self.socket.write(&self.outgoing) {
                Ok(0) => return Err(std::io::Error::other("host control connection closed")),
                Ok(count) => {
                    self.outgoing.drain(..count);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn read(&mut self) -> std::io::Result<Option<Value>> {
        loop {
            if let Some(end) = self.incoming.iter().position(|byte| *byte == b'\n') {
                let frame: Vec<_> = self.incoming.drain(..=end).collect();
                return Ok(Some(serde_json::from_slice(&frame)?));
            }
            let mut chunk = [0; 4096];
            match self.socket.read(&mut chunk) {
                Ok(0) => return Err(std::io::Error::other("host control connection closed")),
                Ok(count) => self.incoming.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }
}
