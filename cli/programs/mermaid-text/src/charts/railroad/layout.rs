use crate::Options;
use crate::canvas::Canvas;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{Atom, Colors, Expr, Grammar};

pub(super) fn render(grammar: Grammar<'_>, options: &Options) -> Result<String, String> {
    let colors = Colors::new(options);
    let mut sections = Vec::from_iter(grammar.title);
    for (name, expr) in grammar.rules {
        let drawing = Tile::from_expr(expr, options)
            .diagram(options.ascii, &colors)
            .map_err(|e| format!("railroad rule `{name}`: {e}"))?;
        let heading = crate::color::with_base(&format!("{name}:"), colors.rule_name)?;
        sections.push(format!("{heading}\n{drawing}"));
    }
    Ok(sections.join("\n\n"))
}

enum Ink {
    Text(String),
    Atom(String, Atom),
    Horizontal(char, usize),
    Vertical(char, usize),
    Child(Tile),
}

/// Entry and exit are terminal rows on the left and right edges. A wrapped
/// sequence can leave on a lower row than it entered.
struct Tile {
    w: usize,
    h: usize,
    entry: usize,
    exit: usize,
    ink: Vec<(usize, usize, Ink)>,
}

impl Tile {
    fn from_expr(expr: Expr, options: &Options) -> Self {
        let children = |items: Vec<_>| {
            items
                .into_iter()
                .map(|item| Self::from_expr(item, options))
                .collect()
        };
        match expr {
            Expr::Atom(text, kind) => {
                let text = match kind {
                    Atom::Nonterminal => format!("[{text}]"),
                    Atom::Special => format!("?{text}?"),
                    _ => text,
                };
                Self::atom(text, kind)
            }
            Expr::Sequence(items) => {
                Self::sequence(children(items), options.width.saturating_sub(6))
            }
            Expr::Choice(items, ordered) => Self::choice(children(items), ordered),
            Expr::Repeat(item, min, max) => Self::from_expr(*item, options).repeat(min, max),
            Expr::Scope(item, caption, exclusion) => Self::from_expr(*item, options).scope(
                caption,
                exclusion.map(|item| Self::from_expr(*item, options)),
                options,
            ),
        }
    }

    fn blank(w: usize, h: usize, entry: usize, exit: usize) -> Self {
        Self {
            w,
            h,
            entry,
            exit,
            ink: Vec::new(),
        }
    }

    fn atom(text: String, kind: Atom) -> Self {
        let columns = text.graphemes(true).map(UnicodeWidthStr::width).sum();
        let mut tile = Self::blank(columns, 1, 0, 0);
        tile.ink.push((0, 0, Ink::Atom(text, kind)));
        tile
    }

    fn text(&mut self, x: usize, y: usize, text: String) {
        self.ink.push((x, y, Ink::Text(text)));
    }

    fn hline(&mut self, start: usize, end: usize, y: usize) {
        self.ink.push((start, y, Ink::Horizontal('─', end - start)));
    }

    fn vline(&mut self, x: usize, start: usize, end: usize) {
        self.ink.push((x, start, Ink::Vertical('│', end - start)));
    }

    fn mark(&mut self, x: usize, y: usize, glyph: char) {
        self.ink.push((x, y, Ink::Horizontal(glyph, 1)));
    }

    fn child(&mut self, x: usize, y: usize, tile: Self) {
        self.ink.push((x, y, Ink::Child(tile)));
    }

    fn row(mut items: Vec<Self>) -> Self {
        if items.len() == 1 {
            return items.pop().unwrap();
        }
        let mut entry = 0;
        let mut descent = 0;
        for item in &items {
            entry = entry.max(item.entry.saturating_sub(descent));
            descent += item.exit - item.entry;
        }
        let mut tile = Self::blank(0, 1, entry, entry + descent);
        let mut y = entry;
        for item in items {
            if tile.w > 0 {
                tile.hline(tile.w, tile.w + 2, y);
                tile.w += 2;
            }
            let top = y - item.entry;
            tile.h = tile.h.max(top + item.h);
            y = top + item.exit;
            let x = tile.w;
            tile.w += item.w;
            tile.child(x, top, item);
        }
        tile
    }

    fn sequence(items: Vec<Self>, width: usize) -> Self {
        let natural_width = items.iter().map(|t| t.w).sum::<usize>() + 2 * (items.len() - 1);
        if natural_width <= width || items.len() == 1 {
            return Self::row(items);
        }
        let mut rows = Vec::new();
        let mut items_on_row = Vec::new();
        let mut used = 0;
        for item in items {
            if used > 0 && used + 2 + item.w > width.saturating_sub(4) {
                rows.push(Self::row(std::mem::take(&mut items_on_row)));
                used = 0;
            }
            used += item.w + if used == 0 { 0 } else { 2 };
            items_on_row.push(item);
        }
        rows.push(Self::row(items_on_row));
        let w = rows.iter().map(|row| row.w).max().unwrap() + 4;
        let mut tile = Self::blank(w, 0, rows[0].entry, 0);
        let last = rows.len() - 1;
        for (i, row) in rows.into_iter().enumerate() {
            let y = tile.h;
            let entry = y + row.entry;
            let exit = y + row.exit;
            tile.hline(0, 2, entry);
            tile.hline(2 + row.w, w, exit);
            tile.h += row.h;
            if i > 0 {
                tile.vline(0, y, entry);
                tile.mark(0, entry, '└');
            }
            if i < last {
                tile.mark(w - 1, exit, '┐');
                tile.vline(w - 1, exit + 1, tile.h);
                tile.hline(1, w - 1, tile.h);
                tile.mark(w / 2, tile.h, '←');
                tile.mark(w - 1, tile.h, '┘');
                tile.mark(0, tile.h, '┌');
                tile.h += 1;
            }
            tile.exit = exit;
            tile.child(2, y, row);
        }
        tile
    }

    fn choice(mut items: Vec<Self>, ordered: bool) -> Self {
        if items.len() == 1 {
            return items.pop().unwrap();
        }
        let last = items.len() - 1;
        let prefix = if ordered {
            items.len().to_string().len() + 2
        } else {
            0
        };
        let w = items.iter().map(|c| c.w).max().unwrap() + prefix + 4;
        let h = items.iter().map(|c| c.h).sum();
        let mut tile = Self::blank(w, h, items[0].entry, items[0].exit);
        let bottom = &items[last];
        tile.vline(0, tile.entry, h - bottom.h + bottom.entry);
        tile.vline(w - 1, tile.exit, h - bottom.h + bottom.exit);
        let mut y = 0;
        for (i, item) in items.into_iter().enumerate() {
            let (entry, exit) = (y + item.entry, y + item.exit);
            tile.hline(0, 2, entry);
            tile.hline(2 + prefix + item.w, w, exit);
            let (l, r) = if i == 0 {
                ('┬', '┬')
            } else if i == last {
                ('└', '┘')
            } else {
                ('├', '┤')
            };
            tile.mark(0, entry, l);
            tile.mark(w - 1, exit, r);
            if ordered {
                let number = format!("{}: ", i + 1);
                tile.hline(2 + number.len(), 2 + prefix, entry);
                tile.text(2, entry, number);
            }
            let top = y;
            y += item.h;
            tile.child(2 + prefix, top, item);
        }
        tile
    }

    fn repeat(self, min: usize, max: Option<usize>) -> Self {
        if max == Some(0) {
            let mut tile = Self::blank(7, 1, 0, 0);
            tile.text(0, 0, "(empty)".into());
            return tile;
        }
        if min == 1 && max == Some(1) {
            return self;
        }
        let skip = min == 0;
        let back = max != Some(1);
        let count = match (min, max) {
            (0 | 1, None | Some(1)) => String::new(),
            (n, Some(m)) if n == m => format!(" {n} total "),
            (n, Some(m)) => format!(" {n}..{m} total "),
            (n, None) => format!(" {n}+ total "),
        };
        let w = (self.w + 4).max(count.len() + 5);
        let y = usize::from(skip);
        let h = y + self.h + usize::from(back);
        let mut tile = Self::blank(w, h, y + self.entry, y + self.exit);
        let (entry, exit) = (tile.entry, tile.exit);
        tile.hline(0, 2, entry);
        tile.hline(2 + self.w, w, exit);
        if skip {
            tile.hline(0, w, 0);
            tile.mark(0, 0, '┌');
            tile.mark(w / 2, 0, '→');
            tile.mark(w - 1, 0, '┐');
            tile.vline(0, 1, entry);
            tile.vline(w - 1, 1, exit);
        }
        if back {
            tile.vline(0, entry + 1, h - 1);
            tile.vline(w - 1, exit + 1, h - 1);
            let arrow = (w - count.len()) / 2;
            tile.hline(0, arrow, h - 1);
            tile.hline(arrow + 1 + count.len(), w, h - 1);
            tile.mark(0, h - 1, '└');
            tile.mark(arrow, h - 1, '←');
            tile.text(arrow + 1, h - 1, count);
            tile.mark(w - 1, h - 1, '┘');
        }
        let joint = match (skip, back) {
            (true, true) => '┼',
            (true, false) => '┴',
            _ => '┬',
        };
        tile.mark(0, entry, joint);
        tile.mark(w - 1, exit, joint);
        tile.child(2, y, self);
        tile
    }

    /// The test track is enclosed and labeled so its operand cannot be
    /// mistaken for another input-consuming element of the outer sequence.
    fn scope(self, caption: &str, exclusion: Option<Self>, options: &crate::Options) -> Self {
        let w = (self.w + 4)
            .max(caption.len() + 4)
            .max(exclusion.as_ref().map_or(0, |e| (e.w + 4).max(10)));
        let h = self.h + 2 + exclusion.as_ref().map_or(0, |e| e.h + 1);
        let mut tile = Self::blank(w, h, self.entry + 1, self.exit + 1);
        tile.vline(0, 1, h - 1);
        tile.vline(w - 1, 1, h - 1);
        let [tl, tr, bl, br] = options.corners.resolve(options.ascii, ['┌', '┐', '└', '┘']);
        for (y, l, r) in [(0, tl, tr), (h - 1, bl, br)] {
            if y == 0 {
                tile.hline(0, 2, y);
                tile.hline(2 + caption.len(), w, y);
            } else {
                tile.hline(0, w, y);
            }
            tile.mark(0, y, l);
            tile.mark(w - 1, y, r);
        }
        tile.text(2, 0, caption.to_owned());
        tile.hline(1, 2, tile.entry);
        tile.hline(2 + self.w, w - 1, tile.exit);
        tile.mark(0, tile.entry, '┼');
        tile.mark(w - 1, tile.exit, '┼');
        if let Some(except) = exclusion {
            let y = 1 + self.h;
            tile.hline(1, 2, y);
            tile.hline(2 + "except".len(), w - 1, y);
            tile.text(2, y, "except".into());
            tile.child(2, y + 1, except);
        }
        tile.child(2, 1, self);
        tile
    }

    fn paint(
        &self,
        canvas: &mut Canvas,
        x: usize,
        y: usize,
        ascii: bool,
        colors: &Colors,
    ) -> Result<(), String> {
        let glyph = |c| {
            if ascii {
                match c {
                    '─' => '-',
                    '│' => '|',
                    '←' => '<',
                    '→' => '>',
                    _ => '+',
                }
            } else {
                c
            }
        };
        for (dx, dy, ink) in &self.ink {
            let (x, y) = (x + dx, y + dy);
            match ink {
                Ink::Text(s) => canvas.text(x, y, s)?,
                Ink::Atom(s, kind) => {
                    canvas.text(x, y, s)?;
                    let style = colors.atom(*kind);
                    canvas.paint(x, y, self.w, style.ink());
                    if !matches!(kind, Atom::Numeric) {
                        canvas.paint(x, y, 1, style.border());
                        canvas.paint(x + self.w - 1, y, 1, style.border());
                    }
                }
                Ink::Child(t) => t.paint(canvas, x, y, ascii, colors)?,
                Ink::Horizontal(c, n) => {
                    canvas.paint(x, y, *n, colors.line);
                    for i in 0..*n {
                        canvas.put(x + i, y, glyph(*c))?;
                    }
                }
                Ink::Vertical(c, n) => {
                    for i in 0..*n {
                        canvas.put(x, y + i, glyph(*c))?;
                        canvas.paint(x, y + i, 1, colors.line);
                    }
                }
            }
        }
        Ok(())
    }

    fn diagram(self, ascii: bool, colors: &Colors) -> Result<String, String> {
        let mut canvas = Canvas::new(self.w + 6, self.h)?;
        self.paint(&mut canvas, 3, 0, ascii, colors)?;
        canvas.text(0, self.entry, if ascii { "o->" } else { "●─→" })?;
        canvas.text(3 + self.w, self.exit, if ascii { "->o" } else { "─→●" })?;
        canvas.paint(0, self.entry, 3, colors.line);
        canvas.paint(3 + self.w, self.exit, 3, colors.line);
        Ok(canvas.finish())
    }
}
