use std::env;
use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Write};

use base64::{Engine, engine::general_purpose::STANDARD};
use rustix::termios::{Winsize, tcgetwinsize};

use crate::graphics::ImageSize;

#[derive(Clone, Copy)]
pub(super) struct Terminal(Winsize);

impl Terminal {
    pub fn detect() -> Option<Self> {
        let valid = |size: Winsize| (size.ws_col > 0 && size.ws_row > 0).then_some(Self(size));
        tcgetwinsize(io::stdout())
            .ok()
            .and_then(valid)
            .or_else(|| tcgetwinsize(io::stderr()).ok().and_then(valid))
            .or_else(|| tcgetwinsize(io::stdin()).ok().and_then(valid))
            .or_else(|| {
                tcgetwinsize(File::open("/dev/tty").ok()?)
                    .ok()
                    .and_then(valid)
            })
    }

    fn bounds(self, max_cols: Option<u16>) -> (u32, u32) {
        let size = self.0;
        let cols = max_cols.unwrap_or(size.ws_col).min(size.ws_col);
        let width = self.cell_width() * f64::from(cols);
        let height = cell_size(size.ws_ypixel, size.ws_row, 16.0)
            * f64::from(size.ws_row.saturating_sub(1).max(1));
        (
            width.floor().max(1.0) as u32,
            height.floor().max(1.0) as u32,
        )
    }

    fn cell_width(self) -> f64 {
        cell_size(self.0.ws_xpixel, self.0.ws_col, 8.0)
    }
}

pub(super) fn image_size(
    width: Option<u16>,
    height: Option<u16>,
    terminal: Option<Terminal>,
    max_cols: Option<u16>,
) -> ImageSize {
    match (width, height) {
        (Some(w), Some(h)) => ImageSize::Exact(w, h),
        (Some(w), None) => ImageSize::Width(w),
        (None, Some(h)) => ImageSize::Height(h),
        (None, None) => terminal.map_or(ImageSize::Natural, |term| {
            let (w, h) = term.bounds(max_cols);
            ImageSize::Fit(w, h)
        }),
    }
}

pub(super) enum Output {
    Png,
    Kitty(Option<Terminal>),
}

impl Output {
    pub fn terminal(terminal: Option<Terminal>) -> Result<Self, String> {
        let stdout = io::stdout();
        if !stdout.is_terminal() {
            return Err(
                "--kitty-image needs a terminal on stdout; use --png for files and pipes".into(),
            );
        }
        let term = env::var("TERM").unwrap_or_default();
        if term.starts_with("screen") || term.starts_with("tmux") || env::var_os("TMUX").is_some() {
            return Err("open a Kitty terminal outside the multiplexer or use --png".into());
        }
        if !(term.contains("kitty")
            || term.contains("ghostty")
            || env::var_os("KITTY_WINDOW_ID").is_some()
            || matches!(
                env::var("TERM_PROGRAM").as_deref(),
                Ok("ghostty" | "WezTerm")
            ))
        {
            return Err(
                "--kitty-image needs a Kitty graphics terminal; use --png to save the image".into(),
            );
        }
        Ok(Self::Kitty(terminal))
    }

    pub fn write(&self, png: &[u8], max_cols: Option<u16>) -> Result<(), String> {
        let mut out = BufWriter::new(io::stdout().lock());
        let written = match self {
            Self::Png => out.write_all(png),
            Self::Kitty(terminal) => display(&mut out, png, *terminal, max_cols),
        };
        written
            .and_then(|()| out.flush())
            .map_err(|err| format!("writing image: {err}"))
    }
}

fn display(
    out: &mut impl Write,
    png: &[u8],
    terminal: Option<Terminal>,
    max_cols: Option<u16>,
) -> io::Result<()> {
    // PNG IHDR stores the canvas dimensions before the compressed image data.
    let w = u32::from_be_bytes(png[16..20].try_into().unwrap()) as f64;
    let h = u32::from_be_bytes(png[20..24].try_into().unwrap()) as f64;
    out.write_all(b"\r\x1b_Ga=T,f=100,t=d,q=2,")?;
    if let Some(terminal) = terminal {
        let (width, height) = terminal.bounds(max_cols);
        let scale = (f64::from(width) / w).min(f64::from(height) / h).min(1.0);
        let columns = (w * scale / terminal.cell_width()).floor().max(1.0) as usize;
        write!(out, "c={columns},")?;
    }
    let mut chunks = png.chunks(3072).peekable();
    let mut encoded = [0; 4096];
    while let Some(chunk) = chunks.next() {
        let more = chunks.peek().is_some();
        let len = STANDARD.encode_slice(chunk, &mut encoded).unwrap();
        write!(out, "m={};", u8::from(more))?;
        out.write_all(&encoded[..len])?;
        out.write_all(b"\x1b\\")?;
        if more {
            out.write_all(b"\x1b_G")?;
        }
    }
    out.write_all(b"\r\n")
}

fn cell_size(pixels: u16, cells: u16, fallback: f64) -> f64 {
    if pixels == 0 || cells == 0 {
        fallback
    } else {
        f64::from(pixels) / f64::from(cells)
    }
}
