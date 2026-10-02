//! Process-wide PTY session registry for one Python worker.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use nix::sys::signal::Signal;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use rustc_hash::FxHashMap;

use crate::diagnostics;
use crate::error::{TerminalError, TerminalResult};
use crate::model::{
    DrainOutcome, InputSpec, PixelSize, RawOutput, Rect, ScreenSnapshot, SessionInfo, Size,
    WaitSpec,
};
use crate::session::{ScreenGeometry, Session, pty_pixel_size};

/// A worker-local registry of child PTY sessions.
pub(crate) struct Manager {
    next_id: AtomicU64,
    sessions: Mutex<FxHashMap<String, Arc<Session>>>,
}

impl Manager {
    fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            sessions: Mutex::new(FxHashMap::default()),
        }
    }

    pub(crate) fn start(
        &self,
        argv: Vec<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        rows: u16,
        cols: u16,
        cell_size: PixelSize,
    ) -> TerminalResult<SessionInfo> {
        if argv.is_empty() || argv[0].is_empty() {
            return Err(TerminalError::invalid(
                "argv must contain a non-empty program",
            ));
        }
        if rows == 0 || cols == 0 {
            return Err(TerminalError::invalid("rows and cols must be positive"));
        }
        if cell_size.width == 0 || cell_size.height == 0 {
            return Err(TerminalError::invalid(
                "cell_size width and height must be positive",
            ));
        }
        let (pixel_width, pixel_height) = pty_pixel_size(cols, rows, cell_size)?;
        if let Some(cwd) = cwd.as_deref()
            && !Path::new(cwd).is_dir()
        {
            return Err(TerminalError::invalid(format!(
                "working directory does not exist or is not a directory: {cwd}"
            )));
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width,
                pixel_height,
            })
            .map_err(|error| TerminalError::runtime(format!("open PTY failed: {error}")))?;

        let mut command = CommandBuilder::new(&argv[0]);
        command.args(argv.iter().skip(1));
        if let Some(cwd) = cwd.as_deref() {
            command.cwd(cwd);
        }
        let mut has_term = false;
        let mut has_pwd = false;
        if let Some(env) = env {
            for (key, value) in env {
                has_term |= key == "TERM";
                has_pwd |= key == "PWD";
                command.env(key, value);
            }
        }
        if let Some(cwd) = cwd.as_deref()
            && !has_pwd
        {
            // TUI programs may prefer PWD over getcwd() when choosing their initial directory.
            command.env("PWD", cwd);
        }
        if !has_term {
            command.env("TERM", "xterm-256color");
        }

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| TerminalError::runtime(format!("clone PTY reader failed: {error}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| TerminalError::runtime(format!("take PTY writer failed: {error}")))?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| TerminalError::runtime(format!("spawn PTY child failed: {error}")))?;
        let id = format!("term-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let session = Session::new(
            id.clone(),
            pair.master,
            writer,
            child,
            reader,
            ScreenGeometry {
                size: Size { rows, cols },
                cell_size,
            },
        )?;
        let info = session.info();
        self.sessions
            .lock()
            .map_err(|_| TerminalError::runtime("terminal session registry mutex poisoned"))?
            .insert(id.clone(), Arc::clone(&session));
        diagnostics::event(
            &id,
            &format!(
                "opened pid={} cols={cols} rows={rows} cell_width={} cell_height={}",
                info.pid, cell_size.width, cell_size.height
            ),
        );
        Ok(info)
    }

    pub(crate) fn list(&self) -> Vec<SessionInfo> {
        let mut sessions: Vec<_> = self
            .sessions
            .lock()
            .expect("terminal session registry mutex poisoned")
            .values()
            .map(|session| session.info())
            .collect();
        sessions.sort_by(|left, right| left.id.cmp(&right.id));
        sessions
    }

    pub(crate) fn inspect(&self, session_id: &str) -> TerminalResult<SessionInfo> {
        Ok(self.session(session_id)?.info())
    }

    pub(crate) fn read(
        &self,
        session_id: &str,
        rect: Option<Rect>,
        wait_for: &WaitSpec,
        trim_trailing_spaces: bool,
        include_cells: bool,
        include_images: bool,
    ) -> TerminalResult<ScreenSnapshot> {
        self.session(session_id)?.read(
            rect,
            wait_for,
            trim_trailing_spaces,
            include_cells,
            include_images,
        )
    }

    /// Wait for a screen or raw pattern for up to `timeout` and report whether it matched.
    pub(crate) fn wait_screen(
        &self,
        session_id: &str,
        rect: Option<Rect>,
        wait_for: &WaitSpec,
        timeout: f64,
        trim_trailing_spaces: bool,
    ) -> TerminalResult<bool> {
        if !timeout.is_finite() || timeout < 0.0 {
            return Err(TerminalError::invalid(
                "timeout must be a finite non-negative number",
            ));
        }
        let deadline = Instant::now()
            .checked_add(Duration::from_secs_f64(timeout))
            .ok_or_else(|| TerminalError::invalid("timeout is too large"))?;
        self.session(session_id)?.wait_screen(
            rect,
            wait_for,
            deadline.saturating_duration_since(Instant::now()),
            trim_trailing_spaces,
        )
    }

    pub(crate) fn read_raw(
        &self,
        session_id: &str,
        max_bytes: usize,
        since: Option<u64>,
    ) -> TerminalResult<RawOutput> {
        Ok(self.session(session_id)?.read_raw(max_bytes, since))
    }

    pub(crate) fn wait(&self, session_id: &str, timeout: f64) -> TerminalResult<DrainOutcome> {
        if !timeout.is_finite() || timeout < 0.0 {
            return Err(TerminalError::invalid(
                "timeout must be a finite non-negative number",
            ));
        }
        Ok(self
            .session(session_id)?
            .wait(Duration::from_secs_f64(timeout)))
    }

    pub(crate) fn write(&self, session_id: &str, input: &InputSpec) -> TerminalResult<()> {
        self.session(session_id)?.write(input)
    }

    pub(crate) fn write_many(&self, session_id: &str, inputs: &[InputSpec]) -> TerminalResult<()> {
        self.session(session_id)?.write_many(inputs)
    }

    pub(crate) fn resize(
        &self,
        session_id: &str,
        rows: u16,
        cols: u16,
    ) -> TerminalResult<SessionInfo> {
        self.session(session_id)?.resize(rows, cols)
    }

    pub(crate) fn signal(&self, session_id: &str, signal_name: &str) -> TerminalResult<()> {
        let signal = parse_signal(signal_name)?;
        self.session(session_id)?.signal(signal)
    }

    pub(crate) fn close(&self, session_id: &str, grace_ms: u64) -> TerminalResult<SessionInfo> {
        let session = self.session(session_id)?;
        let info = session.close(Duration::from_millis(grace_ms.min(30_000)))?;
        diagnostics::event(session_id, "closed");
        self.sessions
            .lock()
            .map_err(|_| TerminalError::runtime("terminal session registry mutex poisoned"))?
            .remove(session_id);
        Ok(info)
    }

    fn session(&self, session_id: &str) -> TerminalResult<Arc<Session>> {
        self.sessions
            .lock()
            .map_err(|_| TerminalError::runtime("terminal session registry mutex poisoned"))?
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                TerminalError::runtime(format!("unknown terminal session: {session_id}"))
            })
    }
}

/// Return the singleton registry owned by this Python worker.
pub(crate) fn global_manager() -> &'static Manager {
    static MANAGER: OnceLock<Manager> = OnceLock::new();
    MANAGER.get_or_init(Manager::new)
}

fn parse_signal(name: &str) -> TerminalResult<Signal> {
    match name
        .trim()
        .trim_start_matches("SIG")
        .to_ascii_uppercase()
        .as_str()
    {
        "HUP" => Ok(Signal::SIGHUP),
        "INT" => Ok(Signal::SIGINT),
        "TERM" => Ok(Signal::SIGTERM),
        "KILL" => Ok(Signal::SIGKILL),
        "QUIT" => Ok(Signal::SIGQUIT),
        other => Err(TerminalError::invalid(format!(
            "unsupported Unix signal: {other}"
        ))),
    }
}
