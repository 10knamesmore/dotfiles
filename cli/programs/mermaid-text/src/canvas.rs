use std::collections::BTreeMap;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const CONTINUATION: char = '\0';
const MAX_CELLS: usize = 4 * 1024 * 1024;

/// Coordinates are terminal cells. A wide grapheme owns its first cell and
/// reserves the following cells, so a later stroke can replace it as a unit.
pub(crate) struct Canvas {
    width: usize,
    cells: Vec<char>,
    clusters: BTreeMap<usize, String>,
    colors: crate::color::Layer,
}

impl Canvas {
    pub(crate) fn new(width: usize, height: usize) -> Result<Self, String> {
        let size = width
            .checked_mul(height)
            .filter(|&size| width > 0 && height > 0 && size <= MAX_CELLS)
            .ok_or_else(|| {
                format!(
                    "cannot allocate {width} × {height} diagram canvas (limit {MAX_CELLS} cells)"
                )
            })?;
        Ok(Self {
            width,
            cells: vec![' '; size],
            clusters: BTreeMap::new(),
            colors: crate::color::Layer::default(),
        })
    }

    fn index(&self, x: usize, y: usize, columns: usize) -> Result<usize, String> {
        if columns == 0
            || y >= self.cells.len() / self.width
            || x >= self.width
            || columns > self.width - x
        {
            return Err(format!(
                "invalid canvas placement ({x}, {y}), {columns} columns"
            ));
        }
        Ok(y * self.width + x)
    }

    fn erase(&mut self, mut index: usize) {
        while self.cells[index] == CONTINUATION {
            index -= 1;
        }
        self.clusters.remove(&index);
        self.cells[index] = ' ';
        let row_end = (index / self.width + 1) * self.width;
        for cell in &mut self.cells[index + 1..row_end] {
            if *cell != CONTINUATION {
                break;
            }
            *cell = ' ';
        }
    }

    fn place(&mut self, index: usize, width: usize, head: char) {
        for cell in index..index + width {
            if self.cells[cell] != ' ' || self.clusters.contains_key(&cell) {
                self.erase(cell);
            }
        }
        self.cells[index] = head;
        self.cells[index + 1..index + width].fill(CONTINUATION);
    }

    pub(crate) fn put(&mut self, x: usize, y: usize, ch: char) -> Result<(), String> {
        let columns = ch.width().unwrap_or(0);
        let index = self.index(x, y, columns)?;
        self.place(index, columns, ch);
        Ok(())
    }

    pub(crate) fn text(&mut self, mut x: usize, y: usize, text: &str) -> Result<(), String> {
        for cluster in text.graphemes(true) {
            if cluster.chars().any(char::is_control) {
                return Err("canvas labels must be a single line of printable text".into());
            }
            let columns = cluster.width();
            let index = self.index(x, y, columns)?;
            let mut chars = cluster.chars();
            self.place(index, columns, chars.next().unwrap());
            if chars.next().is_some() {
                self.clusters.insert(index, cluster.to_owned());
            }
            x += columns;
        }
        Ok(())
    }

    pub(crate) fn paint(&mut self, x: usize, y: usize, width: usize, paint: crate::color::Paint) {
        self.colors.paint(x, y, width, paint);
    }

    pub(crate) fn paint_box(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        style: crate::color::Style,
    ) {
        if style == crate::color::Style::default() || w == 0 || h == 0 {
            return;
        }
        for row in y..y + h {
            if row == y || row == y + h - 1 {
                self.paint(x, row, w, style.border());
            } else {
                self.paint(x, row, 1, style.border());
                self.paint(x + 1, row, w.saturating_sub(2), style.ink());
                self.paint(x + w - 1, row, 1, style.border());
            }
        }
    }

    pub(crate) fn finish(self) -> String {
        let mut output = String::new();
        for (y, row) in self.cells.chunks(self.width).enumerate() {
            if y > 0 {
                output.push('\n');
            }
            for (x, &cell) in row.iter().enumerate() {
                if cell == CONTINUATION {
                    continue;
                }
                if let Some(cluster) = self.clusters.get(&(y * self.width + x)) {
                    output.push_str(cluster);
                } else {
                    output.push(cell);
                }
            }
            output.truncate(output.trim_end_matches(' ').len());
        }
        if self.colors.is_empty() {
            output
        } else {
            self.colors.apply(&output)
        }
    }
}
