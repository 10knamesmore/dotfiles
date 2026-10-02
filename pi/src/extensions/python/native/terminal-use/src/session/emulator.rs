//! Owner thread for one Ghostty terminal core.
//!
//! Ghostty handles are `!Send`, so this module creates, drives, and drops the core on a single
//! thread. Other threads send [`Command`] values and receive owned results; no terminal reference
//! or borrow escapes the thread.

use std::io::Write;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex};

use libghostty_vt::render::{CellIterator, RowIterator};
use libghostty_vt::screen::Screen;
use libghostty_vt::terminal::{
    ColorScheme, ConformanceLevel, DeviceAttributes, DeviceType, Mode, PrimaryDeviceAttributes,
    SecondaryDeviceAttributes, SizeReportSize, TertiaryDeviceAttributes,
};
use libghostty_vt::{RenderState, Terminal, TerminalOptions};

use super::{
    RAW_BUFFER_LIMIT, ScreenGeometry, SharedState, matches_raw, matches_text, process_status,
};
use crate::diagnostics;
use crate::images;
use crate::model::{PixelSize, ScreenSnapshot, WaitOutcome, WaitSource, WaitSpec};
use crate::palette;

/// Upper bound on decoded image bytes the core retains per screen.
pub(crate) const IMAGE_STORAGE_LIMIT_BYTES: u64 = 64 * 1024 * 1024;

/// Commands accepted by the emulator thread.
pub(crate) enum Command {
    /// Feed one chunk of PTY output to the terminal core.
    Feed(Vec<u8>),

    /// Publish end-of-output only after all earlier chunks have been parsed.
    OutputClosed(Option<String>),

    /// Apply new cell dimensions to the terminal core.
    Resize {
        cols: u16,
        rows: u16,
        reply: SyncSender<Result<(), String>>,
    },

    /// Project the current screen text for wait matching.
    ScreenText {
        rect: Option<crate::model::Rect>,
        trim: bool,
        reply: SyncSender<Result<String, String>>,
    },

    /// Build one screen snapshot, optionally with cells and source images.
    Snapshot {
        rect: Option<crate::model::Rect>,
        wait_for: WaitSpec,
        trim: bool,
        cells: bool,
        images: bool,
        reply: SyncSender<Result<ScreenSnapshot, String>>,
    },

    /// Drop the terminal core and stop the thread.
    Shutdown,
}

/// The terminal core and its render state, owned by the emulator thread.
pub(super) struct EmulatorCore {
    pub(super) session_id: String,
    pub(super) terminal: Terminal<'static, 'static>,
    pub(super) render: RenderState<'static>,
    pub(super) row_iterator: RowIterator<'static>,
    pub(super) cell_iterator: CellIterator<'static>,
    pub(super) shared: Arc<SharedState>,
    pub(super) cell_size: PixelSize,
}

const DEVICE_ATTRIBUTES: DeviceAttributes = DeviceAttributes {
    // Report the base terminal identity without advertising Sixel support.
    primary: PrimaryDeviceAttributes::new(ConformanceLevel::VT102, &[]),
    secondary: SecondaryDeviceAttributes {
        device_type: DeviceType::VT100,
        firmware_version: 0,
        rom_cartridge: 1,
    },
    tertiary: TertiaryDeviceAttributes { unit_id: 0 },
};

/// Create the core and run its command loop on the current thread.
///
/// The startup result is sent before the loop starts so the creator can fail session creation
/// without ever observing a half-initialized core.
pub(crate) fn run(
    session_id: String,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    shared: Arc<SharedState>,
    geometry: ScreenGeometry,
    commands: Receiver<Command>,
    ready: SyncSender<Result<(), String>>,
) {
    let mut core = match EmulatorCore::new(session_id, writer, shared, geometry) {
        Ok(core) => core,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    if ready.send(Ok(())).is_err() {
        return;
    }
    core.run(commands);
    diagnostics::event(&core.session_id, "emulator stopped");
}

impl EmulatorCore {
    fn new(
        session_id: String,
        writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
        shared: Arc<SharedState>,
        geometry: ScreenGeometry,
    ) -> Result<Self, String> {
        let rows = geometry.size.rows;
        let cols = geometry.size.cols;
        let cell_size = geometry.cell_size;
        images::install_png_decoder()?;
        let mut terminal: Terminal<'static, 'static> = Terminal::new(TerminalOptions {
            cols,
            rows,
            max_scrollback: 0,
        })
        .map_err(|error| format!("create terminal core failed: {error}"))?;
        terminal
            .set_default_color_palette(Some(palette::xterm_palette()))
            .map_err(|error| format!("set terminal palette failed: {error}"))?;
        terminal
            .set_default_fg_color(Some(palette::DEFAULT_FOREGROUND))
            .map_err(|error| format!("set terminal foreground failed: {error}"))?;
        terminal
            .set_default_bg_color(Some(palette::DEFAULT_BACKGROUND))
            .map_err(|error| format!("set terminal background failed: {error}"))?;
        terminal
            .set_default_cursor_color(Some(palette::DEFAULT_CURSOR))
            .map_err(|error| format!("set terminal cursor color failed: {error}"))?;
        terminal
            .set_kitty_image_storage_limit(IMAGE_STORAGE_LIMIT_BYTES)
            .map_err(|error| format!("enable kitty image storage failed: {error}"))?;
        terminal
            .set_kitty_image_from_file_allowed(true)
            .map_err(|error| format!("enable kitty file images failed: {error}"))?;
        terminal
            .set_kitty_image_from_temp_file_allowed(true)
            .map_err(|error| format!("enable kitty temp-file images failed: {error}"))?;
        terminal
            .set_kitty_image_from_shared_mem_allowed(true)
            .map_err(|error| format!("enable kitty shared-memory images failed: {error}"))?;
        terminal
            .resize(
                cols,
                rows,
                u32::from(cell_size.width),
                u32::from(cell_size.height),
            )
            .map_err(|error| format!("set terminal pixel geometry failed: {error}"))?;

        let output_writer = Arc::clone(&writer);
        let output_session = session_id.clone();
        terminal
            .on_pty_write(move |_terminal, data| {
                write_response(&output_session, &output_writer, data);
            })
            .map_err(|error| format!("register PTY write handler failed: {error}"))?;
        let report_cell = cell_size;
        terminal
            .on_size(move |terminal| {
                Some(SizeReportSize {
                    rows: terminal.rows().ok()?,
                    columns: terminal.cols().ok()?,
                    cell_width: u32::from(report_cell.width),
                    cell_height: u32::from(report_cell.height),
                })
            })
            .map_err(|error| format!("register size handler failed: {error}"))?;
        terminal
            .on_device_attributes(|_| Some(DEVICE_ATTRIBUTES))
            .map_err(|error| format!("register device attributes handler failed: {error}"))?;
        terminal
            .on_color_scheme(|_| Some(ColorScheme::Dark))
            .map_err(|error| format!("register color scheme handler failed: {error}"))?;

        let render =
            RenderState::new().map_err(|error| format!("create render state failed: {error}"))?;
        let row_iterator =
            RowIterator::new().map_err(|error| format!("create row iterator failed: {error}"))?;
        let cell_iterator =
            CellIterator::new().map_err(|error| format!("create cell iterator failed: {error}"))?;
        Ok(Self {
            session_id,
            terminal,
            render,
            row_iterator,
            cell_iterator,
            shared,
            cell_size,
        })
    }

    fn run(&mut self, commands: Receiver<Command>) {
        while let Ok(command) = commands.recv() {
            match command {
                Command::Feed(data) => self.feed(&data),
                Command::OutputClosed(error) => {
                    let mut state = self
                        .shared
                        .state
                        .lock()
                        .expect("terminal state mutex poisoned");
                    state.reader_done = true;
                    if let Some(error) = error {
                        state.reader_error = Some(error);
                    }
                    state.generation = state.generation.wrapping_add(1);
                    self.shared.changed.notify_all();
                }
                Command::Resize { cols, rows, reply } => {
                    let result = self
                        .terminal
                        .resize(
                            cols,
                            rows,
                            u32::from(self.cell_size.width),
                            u32::from(self.cell_size.height),
                        )
                        .map_err(|error| format!("resize terminal core failed: {error}"));
                    if result.is_ok() {
                        let mut state = self
                            .shared
                            .state
                            .lock()
                            .expect("terminal state mutex poisoned");
                        state.rows = rows;
                        state.cols = cols;
                        state.generation = state.generation.wrapping_add(1);
                        self.shared.changed.notify_all();
                    }
                    let _ = reply.send(result);
                }
                Command::ScreenText { rect, trim, reply } => {
                    let result = self
                        .project(rect, trim, false)
                        .map(|projection| projection.lines.join("\n"));
                    let _ = reply.send(result);
                }
                Command::Snapshot {
                    rect,
                    wait_for,
                    trim,
                    cells,
                    images,
                    reply,
                } => {
                    let _ = reply.send(self.snapshot(rect, &wait_for, trim, cells, images));
                }
                Command::Shutdown => break,
            }
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.terminal.vt_write(data);
        let mut state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        for &byte in data {
            if state.raw.len() == RAW_BUFFER_LIMIT {
                state.raw.pop_front();
                state.raw_start_offset += 1;
                state.raw_dropped_bytes += 1;
            }
            state.raw.push_back(byte);
            state.raw_total_bytes += 1;
        }
        state.application_cursor = self
            .terminal
            .mode(Mode::DECCKM)
            .expect("read application cursor mode");
        state.generation = state.generation.wrapping_add(1);
        drop(state);
        self.shared.changed.notify_all();
    }

    fn snapshot(
        &mut self,
        rect: Option<crate::model::Rect>,
        wait_for: &WaitSpec,
        trim: bool,
        include_cells: bool,
        include_images: bool,
    ) -> Result<ScreenSnapshot, String> {
        let projection = self.project(rect, trim, include_cells)?;
        let images = if include_images {
            let images = images::extract(&self.terminal)?;
            diagnostics::event(
                &self.session_id,
                &format!(
                    "images extracted count={} pixel_bytes={}",
                    images.len(),
                    images.iter().map(|image| image.pixels.len()).sum::<usize>()
                ),
            );
            Some(images)
        } else {
            None
        };
        let text = projection.lines.join("\n");
        let state = self
            .shared
            .state
            .lock()
            .expect("terminal state mutex poisoned");
        let matched = match wait_for.source {
            WaitSource::Raw => matches_raw(&state, wait_for),
            WaitSource::Screen => matches_text(wait_for, &text),
        };
        Ok(ScreenSnapshot {
            session_id: self.session_id.clone(),
            cell_size: self.cell_size,
            text,
            status: process_status(&state.exit),
            generation: state.generation,
            raw_dropped_bytes: state.raw_dropped_bytes,
            reader_done: state.reader_done,
            error: state.reader_error.clone(),
            wait: WaitOutcome {
                matched,
                timed_out: !matched,
                source: wait_for.source,
                pattern_supplied: !wait_for.is_empty(),
            },
            full_size: projection.full_size,
            rect: projection.rect,
            lines: projection.lines,
            cells: projection.cells,
            cursor: projection.cursor,
            alternate_screen: projection.alternate_screen,
            application_cursor: projection.application_cursor,
            bracketed_paste: projection.bracketed_paste,
            images,
        })
    }

    /// Whether the alternate screen is the active one.
    pub(super) fn alternate_screen(&self) -> Result<bool, String> {
        self.terminal
            .active_screen()
            .map(|screen| screen == Screen::Alternate)
            .map_err(|error| format!("read active screen failed: {error}"))
    }
}

fn write_response(
    session_id: &str,
    writer: &Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    data: &[u8],
) {
    diagnostics::event(session_id, &format!("pty_write bytes={}", data.len()));
    let result = writer
        .lock()
        .map_err(|_| "terminal writer mutex poisoned".to_owned())
        .and_then(|mut writer| {
            let writer = writer
                .as_mut()
                .ok_or_else(|| "terminal session is closed".to_owned())?;
            writer
                .write_all(data)
                .and_then(|()| writer.flush())
                .map_err(|error| format!("write terminal response failed: {error}"))
        });
    if let Err(error) = result {
        diagnostics::event(session_id, &error);
    }
}
