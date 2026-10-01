//! One Unix PTY process and its continuously drained terminal state.

use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, Rgb};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use portable_pty::{Child, ExitStatus, MasterPty, PtySize};

use crate::diagnostics;
use crate::error::{TerminalError, TerminalResult};
use crate::input::encode_input;
use crate::model::{
    CellColor, CellInfo, CursorInfo, DrainOutcome, InputSpec, ProcessStatus, RawOutput, Rect,
    RectResult, ScreenSnapshot, SessionInfo, Size, WaitMatcher, WaitOutcome, WaitSource, WaitSpec,
};
use crate::palette;

const RAW_BUFFER_LIMIT: usize = 1_048_576;
const COLOR_COUNT: usize = 269;

#[derive(Clone, Debug)]
struct ExitState {
    code: u32,
    signal: Option<String>,
}

struct TerminalDimensions {
    rows: u16,
    cols: u16,
}

impl Dimensions for TerminalDimensions {
    fn total_lines(&self) -> usize {
        usize::from(self.rows)
    }

    fn screen_lines(&self) -> usize {
        usize::from(self.rows)
    }

    fn columns(&self) -> usize {
        usize::from(self.cols)
    }
}

struct TerminalEventListener {
    session_id: String,
    pending_events: Arc<Mutex<Vec<Event>>>,
}

impl EventListener for TerminalEventListener {
    fn send_event(&self, event: Event) {
        // Parsing holds the terminal state lock. Resolve requests afterward so the listener
        // does not recursively lock the Term to read its palette or dimensions.
        if matches!(
            event,
            Event::PtyWrite(_) | Event::ColorRequest(_, _) | Event::TextAreaSizeRequest(_)
        ) {
            if let Err(error) = self
                .pending_events
                .lock()
                .map(|mut pending| pending.push(event))
                .map_err(|_| "terminal event queue mutex poisoned".to_owned())
            {
                diagnostics::event(&self.session_id, &error);
            }
        } else if matches!(
            event,
            Event::ClipboardLoad(_, _) | Event::ClipboardStore(_, _)
        ) {
            diagnostics::event(&self.session_id, "ignored unsupported clipboard event");
        }
    }
}

struct TerminalState {
    processor: Processor,
    term: Term<TerminalEventListener>,
    session_id: String,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    pending_events: Arc<Mutex<Vec<Event>>>,
}

impl TerminalState {
    fn new(
        session_id: String,
        writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
        rows: u16,
        cols: u16,
    ) -> Self {
        let dimensions = TerminalDimensions { rows, cols };
        let pending_events = Arc::new(Mutex::new(Vec::new()));
        let listener = TerminalEventListener {
            session_id: session_id.clone(),
            pending_events: Arc::clone(&pending_events),
        };
        let config = Config {
            scrolling_history: 0,
            ..Config::default()
        };
        Self {
            processor: Processor::new(),
            term: Term::new(config, &dimensions, listener),
            session_id,
            writer,
            pending_events,
        }
    }

    fn process(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
        self.flush_events();
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        self.term.resize(TerminalDimensions { rows, cols });
    }

    fn flush_events(&mut self) {
        let events = match self.pending_events.lock() {
            Ok(mut pending) => std::mem::take(&mut *pending),
            Err(_) => {
                diagnostics::event(&self.session_id, "terminal event queue mutex poisoned");
                return;
            }
        };
        for event in events {
            match event {
                Event::PtyWrite(text) => self.write_response(&text),
                Event::ColorRequest(index, format) => {
                    let color = if index < COLOR_COUNT {
                        self.term.colors()[index].unwrap_or_else(|| palette::default_color(index))
                    } else {
                        palette::default_color(index)
                    };
                    diagnostics::event(&self.session_id, &format!("color_query index={index}"));
                    self.write_response(&format(color));
                }
                Event::TextAreaSizeRequest(format) => {
                    // No pixel surface exists; agree with the PTY's zero pixel dimensions.
                    diagnostics::event(&self.session_id, "pixel_size_query width=0 height=0");
                    self.write_response(&format(WindowSize {
                        num_lines: self.term.screen_lines() as u16,
                        num_cols: self.term.columns() as u16,
                        cell_width: 0,
                        cell_height: 0,
                    }));
                }
                _ => {}
            }
        }
    }

    fn write_response(&self, text: &str) {
        let result = self
            .writer
            .lock()
            .map_err(|_| "terminal writer mutex poisoned".to_owned())
            .and_then(|mut writer| {
                let writer = writer
                    .as_mut()
                    .ok_or_else(|| "terminal session is closed".to_owned())?;
                writer
                    .write_all(text.as_bytes())
                    .map_err(|error| format!("write terminal response failed: {error}"))?;
                writer
                    .flush()
                    .map_err(|error| format!("flush terminal response failed: {error}"))
            });
        if let Err(error) = result {
            diagnostics::event(&self.session_id, &error);
        }
    }
}

struct SessionState {
    terminal: TerminalState,
    rows: u16,
    cols: u16,
    generation: u64,
    raw: VecDeque<u8>,
    raw_start_offset: u64,
    raw_total_bytes: u64,
    raw_dropped_bytes: u64,
    exit: Option<ExitState>,
    reader_done: bool,
    reader_error: Option<String>,
    closed: bool,
}

/// Owns one child process, PTY handles, reader thread, and parsed screen state.
pub(crate) struct Session {
    id: String,
    pid: u32,
    process_group: i32,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    state: Mutex<SessionState>,
    changed: Condvar,
}

impl Session {
    pub(crate) fn new(
        id: String,
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        child: Box<dyn Child + Send + Sync>,
        reader: Box<dyn Read + Send>,
        rows: u16,
        cols: u16,
    ) -> TerminalResult<Arc<Self>> {
        let pid = child
            .process_id()
            .ok_or_else(|| TerminalError::runtime("PTY child did not report a process id"))?;
        let process_group = master.process_group_leader().unwrap_or(pid as i32);
        let writer = Arc::new(Mutex::new(Some(writer)));
        let session = Arc::new(Self {
            id: id.clone(),
            pid,
            process_group,
            master: Mutex::new(Some(master)),
            writer: Arc::clone(&writer),
            state: Mutex::new(SessionState {
                terminal: TerminalState::new(id, Arc::clone(&writer), rows, cols),
                rows,
                cols,
                generation: 0,
                raw: VecDeque::with_capacity(RAW_BUFFER_LIMIT),
                raw_start_offset: 0,
                raw_total_bytes: 0,
                raw_dropped_bytes: 0,
                exit: None,
                reader_done: false,
                reader_error: None,
                closed: false,
            }),
            changed: Condvar::new(),
        });
        session.start_reader(reader)?;
        session.start_waiter(child)?;
        Ok(session)
    }

    fn start_reader(self: &Arc<Self>, mut reader: Box<dyn Read + Send>) -> TerminalResult<()> {
        let session = Arc::clone(self);
        let name = format!("pi-terminal-reader-{}", session.id);
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                let mut buffer = [0_u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(length) => {
                            let mut state =
                                session.state.lock().expect("terminal state mutex poisoned");
                            for byte in &buffer[..length] {
                                if state.raw.len() == RAW_BUFFER_LIMIT {
                                    state.raw.pop_front();
                                    state.raw_start_offset += 1;
                                    state.raw_dropped_bytes += 1;
                                }
                                state.raw.push_back(*byte);
                                state.raw_total_bytes += 1;
                            }
                            state.terminal.process(&buffer[..length]);
                            state.generation = state.generation.wrapping_add(1);
                            session.changed.notify_all();
                        }
                        Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                        Err(error) => {
                            let mut state =
                                session.state.lock().expect("terminal state mutex poisoned");
                            let message = error.to_string();
                            diagnostics::event(&session.id, &format!("reader_error={message}"));
                            state.reader_error = Some(message);
                            break;
                        }
                    }
                }
                let mut state = session.state.lock().expect("terminal state mutex poisoned");
                state.reader_done = true;
                state.generation = state.generation.wrapping_add(1);
                session.changed.notify_all();
            })
            .map(|_| ())
            .map_err(|error| TerminalError::runtime(format!("start PTY reader failed: {error}")))
    }

    fn start_waiter(
        self: &Arc<Self>,
        mut child: Box<dyn Child + Send + Sync>,
    ) -> TerminalResult<()> {
        let session = Arc::clone(self);
        let name = format!("pi-terminal-waiter-{}", session.id);
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                // Signals use the process group, so no other thread needs this child handle.
                match child.wait() {
                    Ok(status) => session.record_exit(status),
                    Err(error) => {
                        let mut state =
                            session.state.lock().expect("terminal state mutex poisoned");
                        let message = format!("waiting for child failed: {error}");
                        diagnostics::event(&session.id, &message);
                        state.reader_error = Some(message);
                        session.changed.notify_all();
                    }
                }
            })
            .map(|_| ())
            .map_err(|error| TerminalError::runtime(format!("start PTY waiter failed: {error}")))
    }

    fn record_exit(&self, status: ExitStatus) {
        let mut state = self.state.lock().expect("terminal state mutex poisoned");
        let code = status.exit_code();
        let signal = status.signal().map(ToOwned::to_owned);
        state.exit = Some(ExitState {
            code,
            signal: signal.clone(),
        });
        diagnostics::event(&self.id, &format!("exited code={code} signal={signal:?}"));
        self.changed.notify_all();
    }

    pub(crate) fn info(&self) -> SessionInfo {
        let state = self.state.lock().expect("terminal state mutex poisoned");
        self.info_locked(&state)
    }

    fn info_locked(&self, state: &SessionState) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            pid: self.pid,
            rows: state.rows,
            cols: state.cols,
            status: process_status(&state.exit),
            generation: state.generation,
            reader_done: state.reader_done,
            closed: state.closed,
            error: state.reader_error.clone(),
        }
    }

    pub(crate) fn read(
        &self,
        rect: Option<Rect>,
        wait_for: &WaitSpec,
        timeout: Duration,
        trim_trailing_spaces: bool,
        include_cells: bool,
    ) -> ScreenSnapshot {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self.state.lock().expect("terminal state mutex poisoned");
        let mut matched = wait_for.is_empty();
        let mut timed_out = false;
        while !matched {
            matched = self.matches(&state, rect, wait_for, trim_trailing_spaces);
            if matched {
                break;
            }
            let Some(deadline) = deadline else {
                timed_out = true;
                break;
            };
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                timed_out = true;
                break;
            };
            if remaining.is_zero() {
                timed_out = true;
                break;
            }
            let (next_state, timeout_result) = self
                .changed
                .wait_timeout(state, remaining.min(Duration::from_millis(50)))
                .expect("terminal state mutex poisoned");
            state = next_state;
            if timeout_result.timed_out() && Instant::now() >= deadline {
                timed_out = true;
            }
        }
        let wait = WaitOutcome {
            matched,
            timed_out: timed_out && !matched,
            source: wait_for.source,
            pattern_supplied: !wait_for.is_empty(),
        };
        snapshot_locked(
            self,
            &state,
            rect,
            trim_trailing_spaces,
            include_cells,
            wait,
        )
    }

    pub(crate) fn wait(&self, timeout: Duration) -> DrainOutcome {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self.state.lock().expect("terminal state mutex poisoned");
        while state.exit.is_none() || !state.reader_done {
            let Some(deadline) = deadline else { break };
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            if remaining.is_zero() {
                break;
            }
            let (next_state, _) = self
                .changed
                .wait_timeout(state, remaining.min(Duration::from_millis(50)))
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

    fn matches(
        &self,
        state: &SessionState,
        rect: Option<Rect>,
        wait_for: &WaitSpec,
        trim_trailing_spaces: bool,
    ) -> bool {
        match (&wait_for.matcher, wait_for.source) {
            (Some(WaitMatcher::Literal(pattern)), WaitSource::Raw) => {
                let raw: Vec<u8> = state.raw.iter().copied().collect();
                raw.windows(pattern.len()).any(|window| window == pattern)
            }
            (Some(WaitMatcher::RawRegex(regex)), WaitSource::Raw) => {
                let raw: Vec<u8> = state.raw.iter().copied().collect();
                regex.is_match(&raw)
            }
            (Some(WaitMatcher::Literal(pattern)), WaitSource::Screen) => {
                screen_text(&state.terminal.term, rect, trim_trailing_spaces)
                    .as_bytes()
                    .windows(pattern.len())
                    .any(|window| window == pattern)
            }
            (Some(WaitMatcher::ScreenRegex(regex)), WaitSource::Screen) => regex.is_match(
                &screen_text(&state.terminal.term, rect, trim_trailing_spaces),
            ),
            (None, _) => true,
            _ => false,
        }
    }

    pub(crate) fn read_raw(&self, max_bytes: usize, since: Option<u64>) -> RawOutput {
        let state = self.state.lock().expect("terminal state mutex poisoned");
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
            .state
            .lock()
            .map_err(|_| TerminalError::runtime("terminal state mutex poisoned"))?
            .terminal
            .term
            .mode()
            .contains(TermMode::APP_CURSOR);
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
        writer
            .write_all(&payload)
            .map_err(|error| TerminalError::runtime(format!("write to PTY failed: {error}")))?;
        writer
            .flush()
            .map_err(|error| TerminalError::runtime(format!("flush PTY input failed: {error}")))
    }

    pub(crate) fn resize(&self, rows: u16, cols: u16) -> TerminalResult<SessionInfo> {
        if rows == 0 || cols == 0 {
            return Err(TerminalError::invalid("rows and cols must be positive"));
        }
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
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| TerminalError::runtime(format!("resize PTY failed: {error}")))?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| TerminalError::runtime("terminal state mutex poisoned"))?;
        state.terminal.resize(rows, cols);
        state.rows = rows;
        state.cols = cols;
        state.generation = state.generation.wrapping_add(1);
        self.changed.notify_all();
        Ok(self.info_locked(&state))
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
        Ok(self.info())
    }

    fn wait_for_shutdown(&self, timeout: Duration) {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self.state.lock().expect("terminal state mutex poisoned");
        while !is_finished(&state) {
            let Some(deadline) = deadline else { break };
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            if remaining.is_zero() {
                break;
            }
            let (next_state, _) = self
                .changed
                .wait_timeout(state, remaining)
                .expect("terminal state mutex poisoned");
            state = next_state;
        }
    }

    fn is_finished(&self) -> bool {
        let state = self.state.lock().expect("terminal state mutex poisoned");
        is_finished(&state)
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

fn snapshot_locked(
    session: &Session,
    state: &SessionState,
    requested: Option<Rect>,
    trim: bool,
    include_cells: bool,
    wait: WaitOutcome,
) -> ScreenSnapshot {
    let term = &state.terminal.term;
    let grid = term.grid();
    let rows = u16::try_from(grid.screen_lines()).unwrap_or(u16::MAX);
    let cols = u16::try_from(grid.columns()).unwrap_or(u16::MAX);
    let requested = requested.unwrap_or(Rect {
        x: 0,
        y: 0,
        width: cols,
        height: rows,
    });
    let x = requested.x.min(cols);
    let y = requested.y.min(rows);
    let width = requested.width.min(cols.saturating_sub(x));
    let height = requested.height.min(rows.saturating_sub(y));
    let right = x.saturating_add(width);
    let bottom = y.saturating_add(height);
    let top_line = -(grid.display_offset() as i32);
    let mut lines = Vec::with_capacity(usize::from(height));
    let mut cell_rows = include_cells.then(|| Vec::with_capacity(usize::from(height)));
    for row in y..bottom {
        let mut line = String::new();
        let mut cells = Vec::with_capacity(usize::from(width));
        let grid_row = Line(top_line + i32::from(row));
        for col in x..right {
            let cell = &grid[grid_row][Column(usize::from(col))];
            let partial_wide =
                cell.flags.contains(Flags::WIDE_CHAR) && col.saturating_add(1) >= right;
            append_screen_text(&mut line, cell, partial_wide);
            if include_cells {
                cells.push(cell_info(cell));
            }
        }
        if trim {
            line = line.trim_end_matches(' ').to_owned();
        }
        lines.push(line);
        if let Some(ref mut cell_rows) = cell_rows {
            cell_rows.push(cells);
        }
    }
    let text = lines.join("\n");
    let cursor = grid.cursor.point;
    let cursor_x = u16::try_from(cursor.column.0).unwrap_or(u16::MAX);
    let cursor_y = u16::try_from(cursor.line.0 - top_line).unwrap_or(u16::MAX);
    let inside = cursor_x >= x && cursor_x < right && cursor_y >= y && cursor_y < bottom;
    let mode = term.mode();
    ScreenSnapshot {
        session_id: session.id.clone(),
        full_size: Size { rows, cols },
        rect: RectResult {
            requested,
            effective: Rect {
                x,
                y,
                width,
                height,
            },
        },
        lines,
        text,
        cursor: CursorInfo {
            x: cursor_x,
            y: cursor_y,
            visible: mode.contains(TermMode::SHOW_CURSOR),
            inside_rect: inside,
            relative_x: inside.then_some(cursor_x - x),
            relative_y: inside.then_some(cursor_y - y),
        },
        alternate_screen: mode.contains(TermMode::ALT_SCREEN),
        application_cursor: mode.contains(TermMode::APP_CURSOR),
        bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
        cells: cell_rows,
        status: process_status(&state.exit),
        generation: state.generation,
        raw_dropped_bytes: state.raw_dropped_bytes,
        reader_done: state.reader_done,
        error: state.reader_error.clone(),
        wait,
    }
}

fn screen_text(term: &Term<TerminalEventListener>, rect: Option<Rect>, trim: bool) -> String {
    let grid = term.grid();
    let rows = u16::try_from(grid.screen_lines()).unwrap_or(u16::MAX);
    let cols = u16::try_from(grid.columns()).unwrap_or(u16::MAX);
    let requested = rect.unwrap_or(Rect {
        x: 0,
        y: 0,
        width: cols,
        height: rows,
    });
    let x = requested.x.min(cols);
    let y = requested.y.min(rows);
    let width = requested.width.min(cols.saturating_sub(x));
    let height = requested.height.min(rows.saturating_sub(y));
    let right = x.saturating_add(width);
    let top_line = -(grid.display_offset() as i32);
    (y..y.saturating_add(height))
        .map(|row| {
            let mut line = String::new();
            let grid_row = Line(top_line + i32::from(row));
            for col in x..right {
                let cell = &grid[grid_row][Column(usize::from(col))];
                let partial_wide =
                    cell.flags.contains(Flags::WIDE_CHAR) && col.saturating_add(1) >= right;
                append_screen_text(&mut line, cell, partial_wide);
            }
            if trim {
                line.trim_end_matches(' ').to_owned()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Project screen text as readable content; grid fidelity remains available through `cells`.
fn append_screen_text(line: &mut String, cell: &Cell, partial_wide: bool) {
    if !partial_wide && !is_wide_continuation(cell) {
        line.push_str(&cell_text(cell));
    }
}

fn cell_text(cell: &Cell) -> String {
    if is_wide_continuation(cell) {
        return " ".to_owned();
    }
    let mut text = cell.c.to_string();
    if let Some(zerowidth) = cell.zerowidth() {
        text.extend(zerowidth);
    }
    text
}

fn is_wide_continuation(cell: &Cell) -> bool {
    cell.flags
        .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
}

fn cell_info(cell: &Cell) -> CellInfo {
    CellInfo {
        text: cell_text(cell),
        wide: cell.flags.contains(Flags::WIDE_CHAR),
        wide_continuation: is_wide_continuation(cell),
        foreground: color(cell.fg),
        background: color(cell.bg),
        bold: cell.flags.contains(Flags::BOLD),
        dim: cell.flags.contains(Flags::DIM),
        italic: cell.flags.contains(Flags::ITALIC),
        underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
        inverse: cell.flags.contains(Flags::INVERSE),
    }
}

fn color(color: Color) -> CellColor {
    match color {
        Color::Named(named) => match named {
            NamedColor::Foreground | NamedColor::Background => CellColor::Default,
            NamedColor::Black
            | NamedColor::Red
            | NamedColor::Green
            | NamedColor::Yellow
            | NamedColor::Blue
            | NamedColor::Magenta
            | NamedColor::Cyan
            | NamedColor::White
            | NamedColor::BrightBlack
            | NamedColor::BrightRed
            | NamedColor::BrightGreen
            | NamedColor::BrightYellow
            | NamedColor::BrightBlue
            | NamedColor::BrightMagenta
            | NamedColor::BrightCyan
            | NamedColor::BrightWhite => CellColor::Indexed { index: named as u8 },
            NamedColor::DimBlack => CellColor::Indexed { index: 0 },
            NamedColor::DimRed => CellColor::Indexed { index: 1 },
            NamedColor::DimGreen => CellColor::Indexed { index: 2 },
            NamedColor::DimYellow => CellColor::Indexed { index: 3 },
            NamedColor::DimBlue => CellColor::Indexed { index: 4 },
            NamedColor::DimMagenta => CellColor::Indexed { index: 5 },
            NamedColor::DimCyan => CellColor::Indexed { index: 6 },
            NamedColor::DimWhite => CellColor::Indexed { index: 7 },
            _ => CellColor::Default,
        },
        Color::Indexed(index) => CellColor::Indexed { index },
        Color::Spec(Rgb { r, g, b }) => CellColor::Rgb {
            red: r,
            green: g,
            blue: b,
        },
    }
}
