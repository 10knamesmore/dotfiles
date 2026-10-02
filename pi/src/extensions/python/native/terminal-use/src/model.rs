//! Serializable data types shared by the public Python module and PTY internals.

use regex::{Regex, bytes::Regex as BytesRegex};
use serde::Serialize;

/// A rectangular terminal region measured in zero-based cells.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Rect {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl Rect {
    pub(crate) fn from_tuple(value: (u16, u16, u16, u16)) -> Result<Self, String> {
        let (x, y, width, height) = value;
        if width == 0 || height == 0 {
            return Err("rect width and height must be positive".to_owned());
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
}

/// The source used by a wait condition.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WaitSource {
    Screen,
    Raw,
}

/// A precompiled string or regular-expression wait matcher.
#[derive(Clone, Debug)]
pub(crate) enum WaitMatcher {
    Literal(Vec<u8>),
    ScreenRegex(Regex),
    RawRegex(BytesRegex),
}

/// A bounded condition evaluated against screen text or raw PTY bytes.
#[derive(Clone, Debug)]
pub(crate) struct WaitSpec {
    pub(crate) source: WaitSource,
    pub(crate) matcher: Option<WaitMatcher>,
}

impl WaitSpec {
    pub(crate) fn new(pattern: Option<String>, source: &str, regex: bool) -> Result<Self, String> {
        let source = match source {
            "screen" => WaitSource::Screen,
            "raw" => WaitSource::Raw,
            other => return Err(format!("wait_source must be screen or raw, got {other:?}")),
        };
        let matcher = pattern
            .as_deref()
            .map(|pattern| match (source, regex) {
                (_, false) => Ok(WaitMatcher::Literal(pattern.as_bytes().to_vec())),
                (WaitSource::Screen, true) => Regex::new(pattern)
                    .map(WaitMatcher::ScreenRegex)
                    .map_err(|error| format!("invalid wait regex {pattern:?}: {error}")),
                (WaitSource::Raw, true) => BytesRegex::new(pattern)
                    .map(WaitMatcher::RawRegex)
                    .map_err(|error| format!("invalid wait regex {pattern:?}: {error}")),
            })
            .transpose()?;
        Ok(Self { source, matcher })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.matcher.is_none()
    }
}

/// A point in the full terminal screen and its relation to a requested rect.
#[derive(Debug, Serialize)]
pub(crate) struct CursorInfo {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) visible: bool,
    pub(crate) inside_rect: bool,
    pub(crate) relative_x: Option<u16>,
    pub(crate) relative_y: Option<u16>,
}

/// The process state exposed to Python.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ProcessStatus {
    Running,
    Exited { code: u32, signal: Option<String> },
}

/// Metadata for one PTY session.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct SessionInfo {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
    pub(crate) status: ProcessStatus,
    pub(crate) generation: u64,
    pub(crate) reader_done: bool,
    pub(crate) closed: bool,
    pub(crate) error: Option<String>,
}

/// The result of a screen or raw wait attached to a read.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct WaitOutcome {
    pub(crate) matched: bool,
    pub(crate) timed_out: bool,
    pub(crate) source: WaitSource,
    pub(crate) pattern_supplied: bool,
}

/// A screen snapshot returned by `read`.
#[derive(Debug, Serialize)]
pub(crate) struct ScreenSnapshot {
    pub(crate) session_id: String,
    pub(crate) full_size: Size,
    pub(crate) rect: RectResult,
    pub(crate) lines: Vec<String>,
    pub(crate) text: String,
    pub(crate) cursor: CursorInfo,
    pub(crate) alternate_screen: bool,
    pub(crate) application_cursor: bool,
    pub(crate) bracketed_paste: bool,
    pub(crate) cells: Option<Vec<Vec<CellInfo>>>,
    pub(crate) status: ProcessStatus,
    pub(crate) generation: u64,
    pub(crate) raw_dropped_bytes: u64,
    pub(crate) reader_done: bool,
    pub(crate) error: Option<String>,
    pub(crate) wait: WaitOutcome,
}

/// A terminal cell and its render attributes.
#[derive(Debug, Serialize)]
pub(crate) struct CellInfo {
    pub(crate) text: String,
    pub(crate) wide: bool,
    pub(crate) wide_continuation: bool,
    pub(crate) foreground: CellColor,
    pub(crate) background: CellColor,
    pub(crate) bold: bool,
    pub(crate) dim: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) inverse: bool,
}

/// A terminal color projected from the emulator's cell state.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CellColor {
    Default,
    Indexed { index: u8 },
    Rgb { red: u8, green: u8, blue: u8 },
}

/// The dimensions of the complete terminal screen.
#[derive(Debug, Serialize)]
pub(crate) struct Size {
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

/// Both the requested and effective screen rectangle.
#[derive(Debug, Serialize)]
pub(crate) struct RectResult {
    pub(crate) requested: Rect,
    pub(crate) effective: Rect,
}

/// Exact bytes read from the bounded PTY output history.
#[derive(Debug, Serialize)]
pub(crate) struct RawOutput {
    pub(crate) session_id: String,
    #[serde(with = "serde_bytes")]
    pub(crate) data: Vec<u8>,
    pub(crate) bytes: usize,
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) lost_bytes: u64,
    pub(crate) truncated: bool,
    pub(crate) dropped_bytes: u64,
    pub(crate) status: ProcessStatus,
    pub(crate) reader_done: bool,
    pub(crate) error: Option<String>,
}

/// The result of waiting for a process and its PTY output to drain.
#[derive(Debug, Serialize)]
pub(crate) struct DrainOutcome {
    pub(crate) session_id: String,
    pub(crate) status: ProcessStatus,
    pub(crate) exited: bool,
    pub(crate) drained: bool,
    pub(crate) timed_out: bool,
    pub(crate) reader_done: bool,
    pub(crate) error: Option<String>,
}

/// One validated input event, retaining key semantics until the session knows its mode.
pub(crate) enum InputSpec {
    Bytes(Vec<u8>),
    Key(KeySpec),
}

/// A key that may need application-cursor-mode encoding.
#[derive(Clone, Debug)]
pub(crate) enum KeySpec {
    Cursor(CursorKey),
    Bytes(Vec<u8>),
}

/// Cursor and navigation keys whose escape sequence depends on terminal mode.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CursorKey {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}
