//! Project terminal core state into the owned screen data returned to Python.
//!
//! The render state is the core's supported path for reading the visible viewport; projections
//! copy cells, styles, and cursor state out so no core borrow survives the emulator thread.

use libghostty_vt::screen::CellWide;
use libghostty_vt::style::{Style, StyleColor, Underline};
use libghostty_vt::terminal::Mode;

use super::emulator::EmulatorCore;
use crate::model::{CellColor, CellInfo, CursorInfo, Rect, RectResult, Size};

/// Owned projection of one visible screen region.
pub(super) struct Projection {
    pub(super) full_size: Size,

    pub(super) rect: RectResult,

    pub(super) lines: Vec<String>,

    pub(super) cells: Option<Vec<Vec<CellInfo>>>,

    pub(super) cursor: CursorInfo,

    pub(super) alternate_screen: bool,

    pub(super) application_cursor: bool,

    pub(super) bracketed_paste: bool,
}

impl EmulatorCore {
    /// Project the requested rectangle of the current viewport.
    ///
    /// `rect` defaults to the full screen and is clipped to it. Text joins wide characters as one
    /// string element while keeping spacer cells in `cells`.
    pub(super) fn project(
        &mut self,
        requested: Option<Rect>,
        trim: bool,
        include_cells: bool,
    ) -> Result<Projection, String> {
        let snapshot = self
            .render
            .update(&self.terminal)
            .map_err(|error| format!("update render state failed: {error}"))?;
        let full_cols = snapshot
            .cols()
            .map_err(|error| format!("read render state columns failed: {error}"))?;
        let full_rows = snapshot
            .rows()
            .map_err(|error| format!("read render state rows failed: {error}"))?;
        let requested = requested.unwrap_or(Rect {
            x: 0,
            y: 0,
            width: full_cols,
            height: full_rows,
        });
        let x = requested.x.min(full_cols);
        let y = requested.y.min(full_rows);
        let width = requested.width.min(full_cols.saturating_sub(x));
        let height = requested.height.min(full_rows.saturating_sub(y));
        let right = x.saturating_add(width);
        let bottom = y.saturating_add(height);

        let mut lines = Vec::with_capacity(usize::from(height));
        let mut cell_rows = include_cells.then(|| Vec::with_capacity(usize::from(height)));
        let mut row_iter = self
            .row_iterator
            .update(&snapshot)
            .map_err(|error| format!("update row iterator failed: {error}"))?;
        let mut row_index: u16 = 0;
        while let Some(row) = row_iter.next() {
            if row_index >= y && row_index < bottom {
                let mut line = String::new();
                let mut cells = include_cells.then(|| Vec::with_capacity(usize::from(width)));
                let mut cell_iter = self
                    .cell_iterator
                    .update(row)
                    .map_err(|error| format!("update cell iterator failed: {error}"))?;
                let mut column: u16 = 0;
                while let Some(cell) = cell_iter.next() {
                    if column >= right {
                        break;
                    }
                    if column >= x {
                        let raw = cell
                            .raw_cell()
                            .map_err(|error| format!("read cell failed: {error}"))?;
                        let wide = raw
                            .wide()
                            .map_err(|error| format!("read cell width failed: {error}"))?;
                        let continuation =
                            matches!(wide, CellWide::SpacerTail | CellWide::SpacerHead);
                        let partial_wide =
                            matches!(wide, CellWide::Wide) && column.saturating_add(1) >= right;
                        let mut text = String::new();
                        cell.graphemes_utf8(&mut text)
                            .map_err(|error| format!("read cell graphemes failed: {error}"))?;
                        if text.is_empty() {
                            text.push(' ');
                        }
                        if !partial_wide && !continuation {
                            line.push_str(&text);
                        }
                        if let Some(cells) = cells.as_mut() {
                            let style = cell
                                .style()
                                .map_err(|error| format!("read cell style failed: {error}"))?;
                            cells.push(cell_info(
                                if continuation { " ".to_owned() } else { text },
                                wide,
                                style,
                            ));
                        }
                    }
                    column += 1;
                }
                if trim {
                    line = line.trim_end_matches(' ').to_owned();
                }
                lines.push(line);
                if let (Some(cells), Some(row_cells)) = (cells, cell_rows.as_mut()) {
                    row_cells.push(cells);
                }
            }
            row_index += 1;
        }

        Ok(Projection {
            full_size: Size {
                rows: full_rows,
                cols: full_cols,
            },
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
            cells: cell_rows,
            cursor: self.cursor_info(x, y, right, bottom)?,
            alternate_screen: self.alternate_screen()?,
            application_cursor: self
                .terminal
                .mode(Mode::DECCKM)
                .map_err(|error| format!("read application cursor mode failed: {error}"))?,
            bracketed_paste: self
                .terminal
                .mode(Mode::BRACKETED_PASTE)
                .map_err(|error| format!("read bracketed paste mode failed: {error}"))?,
        })
    }

    /// Resolve the cursor into viewport coordinates and the requested rectangle.
    fn cursor_info(&self, x: u16, y: u16, right: u16, bottom: u16) -> Result<CursorInfo, String> {
        let cursor_x = self
            .terminal
            .cursor_x()
            .map_err(|error| format!("read cursor column failed: {error}"))?;
        let cursor_y = self
            .terminal
            .cursor_y()
            .map_err(|error| format!("read cursor row failed: {error}"))?;
        let scrollbar = self
            .terminal
            .scrollbar()
            .map_err(|error| format!("read scrollbar failed: {error}"))?;
        let active_top = scrollbar.total.saturating_sub(scrollbar.len);
        let viewport_y = active_top
            .saturating_add(u64::from(cursor_y))
            .saturating_sub(scrollbar.offset);
        let cursor_y = u16::try_from(viewport_y).unwrap_or(u16::MAX);
        let visible = self
            .terminal
            .is_cursor_visible()
            .map_err(|error| format!("read cursor visibility failed: {error}"))?;
        let inside = cursor_x >= x && cursor_x < right && cursor_y >= y && cursor_y < bottom;
        Ok(CursorInfo {
            x: cursor_x,
            y: cursor_y,
            visible,
            inside_rect: inside,
            relative_x: inside.then(|| cursor_x - x),
            relative_y: inside.then(|| cursor_y - y),
        })
    }
}

fn cell_info(text: String, wide: CellWide, style: Style) -> CellInfo {
    CellInfo {
        text,
        wide: matches!(wide, CellWide::Wide),
        wide_continuation: matches!(wide, CellWide::SpacerTail | CellWide::SpacerHead),
        foreground: color(style.fg_color),
        background: color(style.bg_color),
        bold: style.bold,
        dim: style.faint,
        italic: style.italic,
        underline: style.underline != Underline::None,
        inverse: style.inverse,
    }
}

/// Project an explicit cell color, keeping palette indices for styled cells.
fn color(value: StyleColor) -> CellColor {
    match value {
        StyleColor::None => CellColor::Default,
        StyleColor::Palette(index) => CellColor::Indexed { index: index.0 },
        StyleColor::Rgb(rgb) => CellColor::Rgb {
            red: rgb.r,
            green: rgb.g,
            blue: rgb.b,
        },
    }
}
