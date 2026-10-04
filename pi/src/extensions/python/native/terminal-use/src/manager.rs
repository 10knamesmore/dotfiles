//! Process-wide PTY session registry for one Python worker.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use rustc_hash::FxHashMap;

use crate::diagnostics;
use crate::error::{TerminalError, TerminalResult};
use crate::model::{PixelSize, SessionInfo, Size};
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
    ) -> TerminalResult<Arc<Session>> {
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
        Ok(session)
    }

    pub(crate) fn list(&self) -> Vec<Arc<Session>> {
        let mut sessions: Vec<_> = self
            .sessions
            .lock()
            .expect("terminal session registry mutex poisoned")
            .iter()
            .map(|(id, session)| (id.clone(), Arc::clone(session)))
            .collect();
        sessions.sort_by(|left, right| left.0.cmp(&right.0));
        sessions.into_iter().map(|(_, session)| session).collect()
    }

    /// Return metadata once per registry removal so aliases cannot duplicate ownership events.
    pub(crate) fn close(
        &self,
        session: &Session,
        grace_ms: u64,
    ) -> TerminalResult<Option<SessionInfo>> {
        let info = session.close(Duration::from_millis(grace_ms.min(30_000)))?;
        let removed = self
            .sessions
            .lock()
            .map_err(|_| TerminalError::runtime("terminal session registry mutex poisoned"))?
            .remove(&info.id);
        if removed.is_some() {
            diagnostics::event(&info.id, "closed");
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}

/// Return the singleton registry owned by this Python worker.
pub(crate) fn global_manager() -> &'static Manager {
    static MANAGER: OnceLock<Manager> = OnceLock::new();
    MANAGER.get_or_init(Manager::new)
}
