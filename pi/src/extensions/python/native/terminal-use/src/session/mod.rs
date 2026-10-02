//! One Unix PTY process and the terminal core that parses its output.
//!
//! The Ghostty terminal core is `!Send`, so each session owns a dedicated emulator thread that
//! creates, uses, and drops the core. The reader forwards output through a bounded queue; the
//! emulator publishes raw bytes, screen changes, and end-of-output in order. Waiters compare
//! generations under the shared mutex so an update cannot be lost before sleeping.

mod emulator;
mod projection;

use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use portable_pty::{Child, ExitStatus, MasterPty, PtySize};

use self::emulator::Command;
use crate::diagnostics;
use crate::error::{TerminalError, TerminalResult};
use crate::input::encode_input;
use crate::model::{
    DrainOutcome, InputSpec, PixelSize, ProcessStatus, RawOutput, Rect, ScreenSnapshot,
    SessionInfo, Size, WaitMatcher, WaitSource, WaitSpec,
};

const RAW_BUFFER_LIMIT: usize = 1_048_576;
const WAIT_SLICE: Duration = Duration::from_millis(50);

/// Initial cell grid and pixel geometry shared by the PTY and terminal core.
#[derive(Clone, Copy)]
pub(crate) struct ScreenGeometry {
    /// Number of rows and columns.
    pub(crate) size: Size,

    /// Virtual pixel dimensions of one cell.
    pub(crate) cell_size: PixelSize,
}

struct ExitState {
    code: u32,
    signal: Option<String>,
}

/// State shared between the Python caller, reader thread, waiter thread, and emulator thread.
struct SharedState {
    state: Mutex<SessionState>,
    changed: Condvar,
}

struct SessionState {
    rows: u16,
    cols: u16,
    generation: u64,
    raw: VecDeque<u8>,
    raw_start_offset: u64,
    raw_total_bytes: u64,
    raw_dropped_bytes: u64,
    application_cursor: bool,
    exit: Option<ExitState>,
    reader_done: bool,
    reader_error: Option<String>,
    closed: bool,
}

/// Owns one child process, PTY handles, reader thread, and emulator thread.
pub(crate) struct Session {
    id: String,
    pid: u32,
    process_group: i32,
    cell_size: PixelSize,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    shared: Arc<SharedState>,
    commands: SyncSender<Command>,
    emulator: Mutex<Option<JoinHandle<()>>>,
}

impl Session {
    pub(crate) fn new(
        id: String,
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        mut child: Box<dyn Child + Send + Sync>,
        reader: Box<dyn Read + Send>,
        geometry: ScreenGeometry,
    ) -> TerminalResult<Arc<Self>> {
        let ScreenGeometry {
            size: Size { rows, cols },
            cell_size,
        } = geometry;
        let pid = child
            .process_id()
            .ok_or_else(|| TerminalError::runtime("PTY child did not report a process id"))?;
        let process_group = master.process_group_leader().unwrap_or(pid as i32);
        let writer = Arc::new(Mutex::new(Some(writer)));
        let shared = Arc::new(SharedState {
            state: Mutex::new(SessionState {
                rows,
                cols,
                generation: 0,
                raw: VecDeque::with_capacity(RAW_BUFFER_LIMIT),
                raw_start_offset: 0,
                raw_total_bytes: 0,
                raw_dropped_bytes: 0,
                application_cursor: false,
                exit: None,
                reader_done: false,
                reader_error: None,
                closed: false,
            }),
            changed: Condvar::new(),
        });
        // Bound pending PTY output as well as retained raw history.
        let (commands, command_rx) = sync_channel(8);
        let emulator = match start_emulator(
            id.clone(),
            writer.clone(),
            shared.clone(),
            geometry,
            command_rx,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                let _ = killpg(Pid::from_raw(process_group), Signal::SIGKILL);
                let _ = child.wait();
                return Err(error);
            }
        };
        let session = Arc::new(Self {
            id,
            pid,
            process_group,
            cell_size,
            master: Mutex::new(Some(master)),
            writer,
            shared,
            commands,
            emulator: Mutex::new(Some(emulator)),
        });
        session.start_reader(reader)?;
        session.start_waiter(child)?;
        Ok(session)
    }

    fn start_reader(self: &Arc<Self>, mut reader: Box<dyn Read + Send>) -> TerminalResult<()> {
        let session = Arc::clone(self);
        let name = format!("terminal-use-reader-{}", session.id);
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                let mut buffer = [0_u8; 8192];
                let mut reader_error = None;
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(length) => {
                            if session
                                .commands
                                .send(Command::Feed(buffer[..length].to_vec()))
                                .is_err()
                            {
                                session.record_reader_error("terminal emulator thread stopped");
                                break;
                            }
                        }
                        Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                        Err(error) => {
                            let message = error.to_string();
                            diagnostics::event(&session.id, &format!("reader_error={message}"));
                            reader_error = Some(message);
                            break;
                        }
                    }
                }
                let _ = session.commands.send(Command::OutputClosed(reader_error));
            })
            .map(|_| ())
            .map_err(|error| TerminalError::runtime(format!("start PTY reader failed: {error}")))
    }

    fn record_reader_error(&self, message: &str) {
        let mut state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        state.reader_error = Some(message.to_owned());
        state.generation = state.generation.wrapping_add(1);
        self.shared.changed.notify_all();
    }

    fn start_waiter(
        self: &Arc<Self>,
        mut child: Box<dyn Child + Send + Sync>,
    ) -> TerminalResult<()> {
        let session = Arc::clone(self);
        let name = format!("terminal-use-waiter-{}", session.id);
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                // Signals use the process group, so no other thread needs this child handle.
                match child.wait() {
                    Ok(status) => session.record_exit(status),
                    Err(error) => {
                        let message = format!("waiting for child failed: {error}");
                        diagnostics::event(&session.id, &message);
                        session.record_reader_error(&message);
                    }
                }
            })
            .map(|_| ())
            .map_err(|error| TerminalError::runtime(format!("start PTY waiter failed: {error}")))
    }

    fn record_exit(&self, status: ExitStatus) {
        let mut state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        let code = status.exit_code();
        let signal = status.signal().map(ToOwned::to_owned);
        state.exit = Some(ExitState {
            code,
            signal: signal.clone(),
        });
        diagnostics::event(&self.id, &format!("exited code={code} signal={signal:?}"));
        drop(state);
        self.shared.changed.notify_all();
    }

    pub(crate) fn info(&self) -> SessionInfo {
        let state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        self.info_locked(&state)
    }

    fn info_locked(&self, state: &SessionState) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            pid: self.pid,
            rows: state.rows,
            cols: state.cols,
            cell_size: self.cell_size,
            status: process_status(&state.exit),
            generation: state.generation,
            reader_done: state.reader_done,
            closed: state.closed,
            error: state.reader_error.clone(),
        }
    }

    /// Wait until the screen or raw output matches, or the slice expires.
    pub(crate) fn wait_screen(
        &self,
        rect: Option<Rect>,
        wait_for: &WaitSpec,
        timeout: Duration,
        trim: bool,
    ) -> TerminalResult<bool> {
        if wait_for.is_empty() {
            return Ok(true);
        }
        let deadline = Instant::now().checked_add(timeout);
        loop {
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Ok(false);
            }
            let generation = self
                .shared
                .state
                .lock()
                .expect("terminal state mutex poisoned")
                .generation;
            let matched = match wait_for.source {
                WaitSource::Raw => self.matches_raw(wait_for),
                WaitSource::Screen => {
                    let text = self.emulator_screen_text(rect, trim)?;
                    matches_text(wait_for, &text)
                }
            };
            if matched {
                return Ok(true);
            }
            let Some(deadline) = deadline else {
                return Ok(false);
            };
            let mut state = self
                .shared
                .state
                .lock()
                .expect("terminal state mutex poisoned");
            while state.generation == generation {
                let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                    return Ok(false);
                };
                if remaining.is_zero() {
                    return Ok(false);
                }
                let (next_state, _) = self
                    .shared
                    .changed
                    .wait_timeout(state, remaining.min(WAIT_SLICE))
                    .expect("terminal state mutex poisoned");
                state = next_state;
            }
        }
    }

    pub(crate) fn read(
        &self,
        rect: Option<Rect>,
        wait_for: &WaitSpec,
        trim_trailing_spaces: bool,
        include_cells: bool,
        include_images: bool,
    ) -> TerminalResult<ScreenSnapshot> {
        self.emulator_snapshot(
            rect,
            wait_for.clone(),
            trim_trailing_spaces,
            include_cells,
            include_images,
        )
    }

    pub(crate) fn wait(&self, timeout: Duration) -> DrainOutcome {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        while state.exit.is_none() || !state.reader_done {
            let Some(deadline) = deadline else { break };
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            if remaining.is_zero() {
                break;
            }
            let (next_state, _) = self
                .shared
                .changed
                .wait_timeout(state, remaining.min(WAIT_SLICE))
                .expect("terminal state mutex poisoned");
            state = next_state;
        }
        DrainOutcome {
            session_id: self.id.clone(),
            status: process_status(&state.exit),
            exited: state.exit.is_some(),
            drained: state.exit.is_some() && state.reader_done,
            timed_out: state.exit.is_none() || !state.reader_done,
            reader_done: state.reader_done,
            error: state.reader_error.clone(),
        }
    }

    /// Match the wait condition against the retained raw ring.
    fn matches_raw(&self, wait_for: &WaitSpec) -> bool {
        let state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        matches_raw(&state, wait_for)
    }

    pub(crate) fn read_raw(&self, max_bytes: usize, since: Option<u64>) -> RawOutput {
        let state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        let max_bytes = max_bytes.min(RAW_BUFFER_LIMIT);
        let requested = since.unwrap_or(state.raw_start_offset);
        let offset = requested
            .max(state.raw_start_offset)
            .min(state.raw_total_bytes);
        let lost_bytes = state.raw_start_offset.saturating_sub(requested);
        let skip = usize::try_from(offset - state.raw_start_offset)
            .unwrap_or(usize::MAX)
            .min(state.raw.len());
        let data: Vec<u8> = state
            .raw
            .iter()
            .skip(skip)
            .take(max_bytes)
            .copied()
            .collect();
        let next_offset = offset + data.len() as u64;
        RawOutput {
            session_id: self.id.clone(),
            bytes: data.len(),
            data,
            start: offset,
            end: next_offset,
            lost_bytes,
            truncated: next_offset < state.raw_total_bytes,
            dropped_bytes: state.raw_dropped_bytes,
            status: process_status(&state.exit),
            reader_done: state.reader_done,
            error: state.reader_error.clone(),
        }
    }

    pub(crate) fn write(&self, input: &InputSpec) -> TerminalResult<()> {
        self.write_many(std::slice::from_ref(input))
    }

    pub(crate) fn write_many(&self, inputs: &[InputSpec]) -> TerminalResult<()> {
        let application_cursor = self
            .shared
            .state
            .lock()
            .map_err(|_| TerminalError::runtime("terminal state mutex poisoned"))?
            .application_cursor;
        let mut payload = Vec::new();
        for input in inputs {
            encode_input(input, application_cursor, &mut payload);
        }
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| TerminalError::runtime("terminal writer mutex poisoned"))?;
        let writer = writer
            .as_mut()
            .ok_or_else(|| TerminalError::runtime("terminal session is closed"))?;
        let result = writer
            .write_all(&payload)
            .map_err(|error| TerminalError::runtime(format!("write to PTY failed: {error}")))
            .and_then(|()| {
                writer.flush().map_err(|error| {
                    TerminalError::runtime(format!("flush PTY input failed: {error}"))
                })
            });
        diagnostics::event(
            &self.id,
            if result.is_ok() {
                "input.write ok"
            } else {
                "input.write failed"
            },
        );
        result
    }

    pub(crate) fn resize(&self, rows: u16, cols: u16) -> TerminalResult<SessionInfo> {
        if rows == 0 || cols == 0 {
            return Err(TerminalError::invalid("rows and cols must be positive"));
        }
        let (pixel_width, pixel_height) = pty_pixel_size(cols, rows, self.cell_size)?;
        {
            let master = self
                .master
                .lock()
                .map_err(|_| TerminalError::runtime("terminal master mutex poisoned"))?;
            let master = master
                .as_ref()
                .ok_or_else(|| TerminalError::runtime("terminal session is closed"))?;
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width,
                    pixel_height,
                })
                .map_err(|error| TerminalError::runtime(format!("resize PTY failed: {error}")))?;
        }
        self.emulator_resize(rows, cols)?;
        diagnostics::event(&self.id, &format!("resized cols={cols} rows={rows}"));
        Ok(self.info())
    }

    pub(crate) fn signal(&self, signal: Signal) -> TerminalResult<()> {
        killpg(Pid::from_raw(self.process_group), signal).map_err(|error| {
            TerminalError::runtime(format!(
                "send {signal:?} to PTY process group failed: {error}"
            ))
        })
    }

    pub(crate) fn close(&self, grace: Duration) -> TerminalResult<SessionInfo> {
        let should_signal = {
            let mut state = self
                .shared
                .state
                .lock()
                .map_err(|_| TerminalError::runtime("terminal state mutex poisoned"))?;
            if state.closed {
                return Ok(self.info_locked(&state));
            }
            state.closed = true;
            state.exit.is_none()
        };
        if should_signal {
            diagnostics::event(&self.id, "signal=SIGTERM");
            let _ = self.signal(Signal::SIGTERM);
        }
        self.wait_for_shutdown(grace);
        if !self.is_finished() {
            diagnostics::event(&self.id, "signal=SIGKILL");
            let _ = self.signal(Signal::SIGKILL);
            self.wait_for_shutdown(Duration::from_millis(250));
        }
        self.writer
            .lock()
            .map_err(|_| TerminalError::runtime("terminal writer mutex poisoned"))?
            .take();
        self.master
            .lock()
            .map_err(|_| TerminalError::runtime("terminal master mutex poisoned"))?
            .take();
        self.shutdown_emulator();
        Ok(self.info())
    }

    fn wait_for_shutdown(&self, timeout: Duration) {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        while !is_finished(&state) {
            let Some(deadline) = deadline else { break };
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            if remaining.is_zero() {
                break;
            }
            let (next_state, _) = self
                .shared
                .changed
                .wait_timeout(state, remaining)
                .expect("terminal state mutex poisoned");
            state = next_state;
        }
    }

    fn is_finished(&self) -> bool {
        let state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        is_finished(&state)
    }

    /// Stop the emulator thread and drop the terminal core on that thread.
    fn shutdown_emulator(&self) {
        let Some(handle) = self
            .emulator
            .lock()
            .expect("terminal emulator handle mutex poisoned")
            .take()
        else {
            return;
        };
        let _ = self.commands.send(Command::Shutdown);
        let _ = handle.join();
    }

    fn emulator_screen_text(&self, rect: Option<Rect>, trim: bool) -> TerminalResult<String> {
        let (reply_tx, reply_rx) = sync_channel(1);
        self.commands
            .send(Command::ScreenText {
                rect,
                trim,
                reply: reply_tx,
            })
            .map_err(|_| TerminalError::runtime("terminal emulator thread is not running"))?;
        reply_rx
            .recv()
            .map_err(|_| TerminalError::runtime("terminal emulator thread stopped"))?
            .map_err(TerminalError::runtime)
    }

    fn emulator_snapshot(
        &self,
        rect: Option<Rect>,
        wait_for: WaitSpec,
        trim: bool,
        cells: bool,
        images: bool,
    ) -> TerminalResult<ScreenSnapshot> {
        let (reply_tx, reply_rx) = sync_channel(1);
        self.commands
            .send(Command::Snapshot {
                rect,
                wait_for,
                trim,
                cells,
                images,
                reply: reply_tx,
            })
            .map_err(|_| TerminalError::runtime("terminal emulator thread is not running"))?;
        reply_rx
            .recv()
            .map_err(|_| TerminalError::runtime("terminal emulator thread stopped"))?
            .map_err(TerminalError::runtime)
    }

    fn emulator_resize(&self, rows: u16, cols: u16) -> TerminalResult<()> {
        let (reply_tx, reply_rx) = sync_channel(1);
        self.commands
            .send(Command::Resize {
                cols,
                rows,
                reply: reply_tx,
            })
            .map_err(|_| TerminalError::runtime("terminal emulator thread is not running"))?;
        reply_rx
            .recv()
            .map_err(|_| TerminalError::runtime("terminal emulator thread stopped"))?
            .map_err(TerminalError::runtime)
    }
}

fn start_emulator(
    session_id: String,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    shared: Arc<SharedState>,
    geometry: ScreenGeometry,
    commands: std::sync::mpsc::Receiver<Command>,
) -> TerminalResult<JoinHandle<()>> {
    let (ready_tx, ready_rx) = sync_channel(1);
    let name = format!("terminal-use-emulator-{session_id}");
    let handle = thread::Builder::new()
        .name(name)
        .spawn(move || emulator::run(session_id, writer, shared, geometry, commands, ready_tx))
        .map_err(|error| {
            TerminalError::runtime(format!("start terminal emulator thread failed: {error}"))
        })?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(error)) => {
            let _ = handle.join();
            Err(TerminalError::runtime(error))
        }
        Err(_) => {
            let _ = handle.join();
            Err(TerminalError::runtime(
                "terminal emulator thread exited during startup",
            ))
        }
    }
}

fn is_finished(state: &SessionState) -> bool {
    state.exit.is_some() && state.reader_done
}

fn process_status(exit: &Option<ExitState>) -> ProcessStatus {
    match exit {
        Some(exit) => ProcessStatus::Exited {
            code: exit.code,
            signal: exit.signal.clone(),
        },
        None => ProcessStatus::Running,
    }
}

/// Match a wait condition against raw bytes published by the emulator.
fn matches_raw(state: &SessionState, wait_for: &WaitSpec) -> bool {
    match &wait_for.matcher {
        Some(WaitMatcher::Literal(pattern)) => {
            let raw: Vec<u8> = state.raw.iter().copied().collect();
            raw.windows(pattern.len()).any(|window| window == pattern)
        }
        Some(WaitMatcher::RawRegex(regex)) => {
            let raw: Vec<u8> = state.raw.iter().copied().collect();
            regex.is_match(&raw)
        }
        _ => true,
    }
}

/// Match a wait condition against projected screen text.
fn matches_text(wait_for: &WaitSpec, text: &str) -> bool {
    match &wait_for.matcher {
        Some(WaitMatcher::Literal(pattern)) => text
            .as_bytes()
            .windows(pattern.len())
            .any(|window| window == pattern),
        Some(WaitMatcher::ScreenRegex(regex)) => regex.is_match(text),
        _ => true,
    }
}

/// Convert cell and row counts into PTY pixel dimensions.
///
/// The PTY window size carries pixel dimensions as `u16`, and the same geometry is given to the
/// terminal core, so both must fit before the session is created or resized.
pub(crate) fn pty_pixel_size(
    cols: u16,
    rows: u16,
    cell_size: PixelSize,
) -> TerminalResult<(u16, u16)> {
    let width = u32::from(cols) * u32::from(cell_size.width);
    let height = u32::from(rows) * u32::from(cell_size.height);
    if width > u32::from(u16::MAX) || height > u32::from(u16::MAX) {
        return Err(TerminalError::invalid(format!(
            "terminal pixel size {width}x{height} exceeds the PTY maximum of 65535x65535; reduce cols, rows, or cell_size"
        )));
    }
    Ok((width as u16, height as u16))
}
