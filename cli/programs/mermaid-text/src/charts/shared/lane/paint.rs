use super::{
    Diagram, Line, Tip,
    layout::{Block, Plan, Rect},
    routes::{Point, Routes},
};
use crate::{Options, canvas::Canvas, color};
use unicode_width::UnicodeWidthStr;

struct Page {
    canvas: Canvas,
    routes: Routes,
    ascii: bool,
}

impl Page {
    fn new(w: usize, h: usize, ascii: bool) -> Result<Self, String> {
        Ok(Self {
            canvas: Canvas::new(w, h)?,
            routes: Routes::new(w, h),
            ascii,
        })
    }

    fn block(&mut self, b: &Block, shape: &str, options: &Options) -> Result<(), String> {
        let Rect { x, y, w, h } = b.rect;
        self.routes.block(b.rect, b.frame);
        let oval = matches!(shape, "stadium" | "circle" | "double-circle");
        let event = matches!(shape, "bpmn-start" | "bpmn-end");
        let rounded = oval || event || matches!(shape, "rounded" | "task");
        let diamond = matches!(shape, "decision" | "diamond");
        let corners = if diamond {
            ['/', '\\', '\\', '/']
        } else if self.ascii {
            ['+'; 4]
        } else if rounded && !b.frame {
            ['╭', '╮', '╰', '╯']
        } else {
            ['┌', '┐', '└', '┘']
        };
        let corners = if diamond || !b.frame && (oval || event) {
            corners
        } else {
            options.corners.resolve(self.ascii, corners)
        };
        let horizontal = if self.ascii { '-' } else { '─' };
        let vertical = if self.ascii { '|' } else { '│' };
        for dy in 0..h {
            for dx in 0..w {
                if dx != 0 && dy != 0 && dx + 1 != w && dy + 1 != h {
                    continue;
                }
                let ch = match (dx, dy) {
                    (0, 0) => corners[0],
                    (_, 0) if dx + 1 == w => corners[1],
                    (0, _) if dy + 1 == h => corners[2],
                    (_, _) if dx + 1 == w && dy + 1 == h => corners[3],
                    (_, 0) => horizontal,
                    (_, _) if dy + 1 == h => horizontal,
                    (0, _) if oval && dy == h / 2 => '(',
                    (_, _) if oval && dy == h / 2 => ')',
                    (0, _) if diamond && dy == h / 2 => '<',
                    (_, _) if diamond && dy == h / 2 => '>',
                    _ => vertical,
                };
                self.canvas.put(x + dx, y + dy, ch)?;
            }
        }
        for (i, line) in b.lines.iter().enumerate() {
            let dx = if b.frame { 2 } else { (w - line.width()) / 2 };
            let p = Point(x + dx, y + i + 1);
            self.routes.block_text(p, line.width());
            self.canvas.text(p.0, p.1, line)?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<String, String> {
        let unicode = [
            ' ', '─', '│', '┌', '─', '─', '┐', '┬', '│', '└', '│', '├', '┘', '┴', '┤', '┼',
        ];
        let arrow = if self.ascii {
            ['>', 'v', '<', '^']
        } else {
            ['▶', '▼', '◀', '▲']
        };
        for (Point(x, y), c) in self.routes.strokes() {
            let mut ch = unicode[c.strokes as usize];
            if matches!(ch, '─' | '│') {
                ch = match (c.line, ch, self.ascii) {
                    (Some(Line::Dotted), _, true) => '.',
                    (Some(Line::Dotted), '─', false) => '┄',
                    (Some(Line::Dotted), _, false) => '┆',
                    (Some(Line::Thick), '─', true) => '=',
                    (Some(Line::Thick), '─', false) => '━',
                    (Some(Line::Thick), _, false) => '┃',
                    (_, '─', true) => '-',
                    (_, _, true) => '|',
                    _ => ch,
                };
            } else if self.ascii {
                ch = '+';
            }
            let ch = match c.tip {
                Some((Tip::Arrow, direction)) => arrow[direction],
                Some((Tip::Cross, _)) if self.ascii => 'x',
                Some((Tip::Cross, _)) => '×',
                _ => ch,
            };
            self.canvas.put(x, y, ch)?;
        }
        Ok(self.canvas.finish())
    }
}

pub(super) fn draw(d: &Diagram, plan: &Plan, options: &Options) -> Result<String, String> {
    let Rect { w, h, .. } = plan.blocks[0].rect;
    let mut page = Page::new(w, h, options.ascii)?;
    for &id in &plan.order[1..] {
        let block = &plan.blocks[id];
        page.block(block, &d.items[id].shape, options)?;
        if options.color {
            let Rect { x, y, w, h } = block.rect;
            page.canvas
                .paint_box(x, y, w, h, options.theme_style().overlay(d.items[id].style));
        }
    }
    let mut notes = vec![];
    for e in &d.edges {
        let style = if options.color {
            color::Style {
                stroke: options.theme_line(),
                ..Default::default()
            }
            .overlay(e.style)
        } else {
            Default::default()
        };
        let label = e.label.replace('\n', " ");
        if let Some((path, source, target)) = page.routes.route(
            plan.blocks[e.from].rect,
            plan.blocks[e.to].rect,
            d.direction,
            e.from == e.to,
        ) {
            page.routes.trace(&path, source, target, e);
            if let Some(fg) = style.stroke {
                let ink = color::Paint {
                    fg: Some(fg),
                    bg: None,
                };
                for &Point(x, y) in &path {
                    page.canvas.paint(x, y, 1, ink);
                }
            }
            if label.is_empty() {
                continue;
            }
            if let Some(Point(x, y)) = page.routes.label_position(&path, label.width()) {
                page.canvas.text(x, y, &label)?;
                page.canvas.paint(x, y, label.width(), style.ink());
                continue;
            }
        }
        let symbol = match (e.line, e.tip, options.ascii) {
            (_, Tip::Cross, true) => "--x",
            (_, Tip::Cross, false) => "─×",
            (Line::Dotted, Tip::None, true) => "...",
            (Line::Dotted, Tip::None, false) => "┄┄",
            (_, Tip::None, true) => "--",
            (_, Tip::None, false) => "──",
            (_, _, true) => "->",
            (_, _, false) => "→",
        };
        let mut relation = format!(
            "{} {symbol} {}",
            d.items[e.from].label.replace('\n', " "),
            d.items[e.to].label.replace('\n', " ")
        );
        if !label.is_empty() {
            relation.push_str(": ");
            relation.push_str(&label);
        }
        notes.push(color::with_base(&relation, style.ink())?);
    }
    let picture = page.finish()?;
    let mut result = Vec::new();
    if !d.title.is_empty() {
        result.push(d.title.as_str());
    }
    if !d.description.is_empty() {
        result.push(d.description.as_str());
    }
    result.push(picture.trim_matches('\n'));
    result.extend(notes.iter().map(String::as_str));
    Ok(result.join("\n"))
}
