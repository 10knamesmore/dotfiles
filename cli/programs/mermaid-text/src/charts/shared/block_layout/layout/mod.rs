use super::*;
use crate::canvas::Canvas;
use crate::color::{Layer, Paint};
use crate::text::wrap;
use unicode_width::UnicodeWidthStr;

mod image;
pub use image::image;

#[derive(Clone, Copy, Default)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn right(self) -> usize {
        self.x + self.w - 1
    }
    pub fn bottom(self) -> usize {
        self.y + self.h - 1
    }
}

#[derive(Default)]
struct Block {
    w: usize,
    h: usize,
    header: Vec<String>,
    body: Vec<String>,
    children: Option<Layout>,
}

struct Slot {
    node: usize,
    col: usize,
    row: usize,
    span: usize,
}

struct Layout {
    cols: Vec<usize>,
    rows: Vec<usize>,
    slots: Vec<Slot>,
    dx: usize,
    dy: usize,
}

impl Layout {
    fn width(&self) -> usize {
        self.cols.iter().sum::<usize>() + self.dx * (self.cols.len() - 1)
    }
    fn height(&self) -> usize {
        self.rows.iter().sum::<usize>() + self.dy * (self.rows.len() - 1)
    }

    fn place(&self, blocks: &[Block], rects: &mut [Rect], x: usize, y: usize, width: usize) {
        let mut cols = self.cols.clone();
        distribute(&mut cols, width - self.width());
        let offsets = |values: &[usize], gap: usize, mut start: usize| {
            values
                .iter()
                .map(|v| {
                    let p = start;
                    start += v + gap;
                    p
                })
                .collect::<Vec<_>>()
        };
        let xs = offsets(&cols, self.dx, x);
        let ys = offsets(&self.rows, self.dy, y);
        for s in &self.slots {
            let b = &blocks[s.node];
            let w = cols[s.col..s.col + s.span].iter().sum::<usize>() + self.dx * (s.span - 1);
            let rect = Rect {
                x: xs[s.col],
                y: ys[s.row] + (self.rows[s.row] - b.h) / 2,
                w,
                h: b.h,
            };
            rects[s.node] = rect;
            if let Some(child) = &b.children {
                child.place(
                    blocks,
                    rects,
                    rect.x + 2,
                    rect.y + 2 + b.header.len(),
                    rect.w - 4,
                );
            }
        }
    }

    fn paint(
        &self,
        nodes: &[Node],
        blocks: &[Block],
        rects: &[Rect],
        theme: Style,
        paint: &mut impl FnMut(usize, usize, usize, Paint),
    ) {
        for slot in &self.slots {
            let i = slot.node;
            let r = rects[i];
            let block = &blocks[i];
            let style = theme.overlay(nodes[i].style);
            let shape = node_shape(&nodes[i]);
            let cylinder = usize::from(matches!(shape, Shape::Cylinder));
            let side = if matches!(shape, Shape::Routine | Shape::DoubleCircle) {
                2
            } else {
                1
            };
            for y in r.y..=r.bottom() {
                if y <= r.y + cylinder || y == r.bottom() {
                    paint(r.x, y, r.w, style.border());
                } else {
                    paint(r.x, y, side, style.border());
                    paint(
                        r.x + side,
                        y,
                        r.w - side * 2,
                        Paint {
                            fg: None,
                            bg: style.fill,
                        },
                    );
                    paint(r.x + r.w - side, y, side, style.border());
                }
            }
            for (row, line) in block.header.iter().enumerate() {
                let width = line.width();
                paint(
                    r.x + (r.w - width) / 2,
                    r.y + 1 + cylinder + row,
                    width,
                    style.ink(),
                );
            }
            if !block.body.is_empty() {
                let divider = r.y + 1 + block.header.len() + cylinder;
                paint(r.x, divider, r.w, style.border());
                for (row, line) in block.body.iter().enumerate() {
                    paint(r.x + 2, divider + 1 + row, line.width(), style.ink());
                }
            }
            if let Some(children) = &block.children {
                children.paint(nodes, blocks, rects, theme, paint);
            }
        }
    }
}

fn distribute(values: &mut [usize], extra: usize) {
    let n = values.len();
    for (i, w) in values.iter_mut().enumerate() {
        *w += extra / n + usize::from(i < extra % n);
    }
}

struct Measure<'a> {
    diagram: &'a Diagram,
    blocks: Vec<Block>,
    degrees: Vec<usize>,
    dx: usize,
    dy: usize,
    ascii: bool,
    scene: Option<&'a crate::graphics::Scene>,
}

impl Measure<'_> {
    fn grid(&mut self, grid: &Grid, available: usize) -> Result<Layout, String> {
        let limit = grid
            .columns
            .unwrap_or_else(|| grid.slots.iter().map(|s| s.1).sum::<usize>().max(1));
        let (mut col, mut row, mut ncols) = (0, 0, 1);
        let mut slots = Vec::new();
        for &(node, span) in &grid.slots {
            if col > 0 && col + span > limit {
                row += 1;
                col = 0;
            }
            if let Some(node) = node {
                slots.push(Slot {
                    node,
                    col,
                    row,
                    span,
                });
            }
            col += span;
            ncols = ncols.max(col);
        }
        if ncols > 65536 || row >= 65536 {
            return Err("block grid exceeds 65536 rows or columns".into());
        }
        let mut cols = vec![3; ncols];
        let mut rows = vec![1; row + 1];
        let each = available.saturating_sub((ncols - 1) * self.dx) / ncols;
        for s in &slots {
            let width = each * s.span + self.dx * (s.span - 1);
            let n = &self.diagram.nodes[s.node];
            let mut b = Block::default();
            let inner = width.saturating_sub(4).clamp(6, 60);
            match &n.kind {
                Kind::Block(shape) => {
                    b.header = wrap(&n.label, inner);
                    b.h = b.header.len() + 2 + usize::from(matches!(shape, Shape::Cylinder));
                }
                Kind::Group(g) => {
                    if !n.label.is_empty() {
                        b.header = wrap(&n.label, inner);
                    }
                    let child = self.grid(g, width.saturating_sub(4))?;
                    b.w = child.width() + 4;
                    b.h = child.height() + b.header.len() + 4;
                    b.children = Some(child);
                }
                Kind::Requirement { kind, fields } => {
                    b.header.push(if self.ascii {
                        format!("<<{kind}>>")
                    } else {
                        format!("«{kind}»")
                    });
                    b.header.extend(wrap(&n.label, inner.max(14)));
                    for (k, v) in fields {
                        b.body
                            .extend(wrap(&format!("{k}: {v}"), inner.clamp(14, 44)));
                    }
                    b.h = b.header.len() + b.body.len() + 2 + usize::from(!b.body.is_empty());
                }
            }
            let text = b
                .header
                .iter()
                .chain(&b.body)
                .map(|s| {
                    self.scene
                        .map_or_else(|| s.width(), |scene| image::columns(scene, s))
                })
                .max()
                .unwrap_or(0);
            let padding = if matches!(n.kind, Kind::Block(Shape::Routine | Shape::DoubleCircle)) {
                6
            } else {
                4
            };
            b.w = b.w.max(text + padding).max(self.degrees[s.node] + 3).max(5);
            if self.scene.is_some()
                && let Kind::Block(shape) = n.kind
            {
                image::fit_shape(&mut b, shape, text);
            }
            rows[s.row] = rows[s.row].max(b.h);
            self.blocks[s.node] = b;
        }
        // Satisfy single-column widths first; a spanning block only grows the
        // columns under it if their combined width still falls short.
        let mut by_span: Vec<_> = slots.iter().collect();
        by_span.sort_by_key(|s| s.span);
        for s in by_span {
            let covered = &mut cols[s.col..s.col + s.span];
            let w = covered.iter().sum::<usize>() + self.dx * (s.span - 1);
            distribute(covered, self.blocks[s.node].w.saturating_sub(w));
        }
        let layout = Layout {
            cols,
            rows,
            slots,
            dx: self.dx,
            dy: self.dy,
        };
        if layout.width() > 4_194_304 || layout.height() > 4_194_304 {
            return Err("block layout exceeds canvas dimensions".into());
        }
        Ok(layout)
    }
}

fn frame(c: &mut Canvas, r: Rect, shape: Shape, options: &Options) -> Result<(), String> {
    let ascii = options.ascii;
    let (l, t, rt, b) = (r.x, r.y, r.right(), r.bottom());
    let (h, v) = if ascii { ('-', '|') } else { ('─', '│') };
    let square = if ascii {
        ['+'; 4]
    } else {
        ['┌', '┐', '└', '┘']
    };
    let rounded = if ascii {
        ['/', '\\', '\\', '/']
    } else {
        ['╭', '╮', '╰', '╯']
    };
    let mut sides = [v, v];
    let corners = match shape {
        Shape::Box | Shape::Routine | Shape::Arrows(_) => options.corners.resolve(ascii, square),
        Shape::Round => options.corners.resolve(ascii, rounded),
        Shape::Stadium | Shape::Circle | Shape::DoubleCircle => {
            sides = ['(', ')'];
            rounded
        }
        Shape::Cylinder => rounded,
        Shape::Diamond | Shape::Hexagon => {
            if matches!(shape, Shape::Diamond) {
                sides = ['<', '>'];
            }
            ['/', '\\', '\\', '/']
        }
        Shape::Flag => {
            sides[0] = '>';
            ['\\', square[1], '/', square[3]]
        }
        Shape::Slanted(a, b) => {
            sides = [a, b];
            [a, b, a, b]
        }
    };
    for x in l + 1..rt {
        c.put(x, t, h)?;
        c.put(x, b, h)?;
    }
    for y in t + 1..b {
        c.put(l, y, sides[0])?;
        c.put(rt, y, sides[1])?;
        if matches!(shape, Shape::Routine | Shape::DoubleCircle) {
            c.put(l + 1, y, sides[0])?;
            c.put(rt - 1, y, sides[1])?;
        }
    }
    for (i, (x, y)) in [(l, t), (rt, t), (l, b), (rt, b)].into_iter().enumerate() {
        c.put(x, y, corners[i])?;
    }
    if matches!(shape, Shape::Routine) {
        for x in [l + 1, rt - 1] {
            c.put(x, t, if ascii { '+' } else { '┬' })?;
            c.put(x, b, if ascii { '+' } else { '┴' })?;
        }
    }
    if matches!(shape, Shape::Cylinder) {
        c.put(l, t + 1, if ascii { '\\' } else { '╰' })?;
        c.put(rt, t + 1, if ascii { '/' } else { '╯' })?;
        for x in l + 1..rt {
            c.put(x, t + 1, h)?;
        }
    }
    if let Shape::Arrows(mask) = shape {
        for (bit, x, y, glyph) in [
            (NORTH, l + r.w / 2, t, if ascii { '^' } else { '▲' }),
            (SOUTH, l + r.w / 2, b, if ascii { 'v' } else { '▼' }),
            (WEST, l, t + r.h / 2, if ascii { '<' } else { '◀' }),
            (EAST, rt, t + r.h / 2, if ascii { '>' } else { '▶' }),
        ] {
            if mask & bit != 0 {
                c.put(x, y, glyph)?;
            }
        }
    }
    Ok(())
}

fn node_shape(node: &Node) -> Shape {
    if let Kind::Block(shape) = node.kind {
        shape
    } else {
        Shape::Box
    }
}

fn boxes(
    c: &mut Canvas,
    d: &Diagram,
    blocks: &[Block],
    rects: &[Rect],
    options: &Options,
) -> Result<(), String> {
    let ascii = options.ascii;
    for (i, n) in d.nodes.iter().enumerate() {
        let r = rects[i];
        if r.w == 0 {
            continue;
        }
        let b = &blocks[i];
        let shape = node_shape(n);
        frame(c, r, shape, options)?;
        let mut y = r.y + 1 + usize::from(matches!(shape, Shape::Cylinder));
        for line in &b.header {
            c.text(r.x + (r.w - line.width()) / 2, y, line)?;
            y += 1;
        }
        if !b.body.is_empty() {
            c.put(r.x, y, if ascii { '+' } else { '├' })?;
            for x in r.x + 1..r.right() {
                c.put(x, y, if ascii { '-' } else { '─' })?;
            }
            c.put(r.right(), y, if ascii { '+' } else { '┤' })?;
            y += 1;
            for line in &b.body {
                c.text(r.x + 2, y, line)?;
                y += 1;
            }
        }
    }
    Ok(())
}

pub fn render(
    d: Diagram,
    grid: Grid,
    options: &Options,
    theme: Option<super::DiagramColors>,
) -> Result<String, String> {
    if grid.slots.is_empty() {
        return Err("block diagram has no blocks".into());
    }
    if d.edges.len() > 4096 {
        return Err("diagram exceeds 4096 connections".into());
    }
    let mut degree = vec![0; d.nodes.len()];
    for e in &d.edges {
        degree[e.from] += 1;
        degree[e.to] += 1;
    }
    let label_width = options.width.clamp(12, 56);
    let edge_labels: Vec<_> = d
        .edges
        .iter()
        .map(|e| wrap(&e.label, label_width))
        .collect();
    let longest = edge_labels
        .iter()
        .flatten()
        .map(|s| s.width())
        .max()
        .unwrap_or(0);
    let max_degree = degree.iter().copied().max().unwrap_or(0);
    let connected = !d.edges.is_empty();
    let mut work = 24_000_000usize;
    let mut order: Vec<_> = (0..d.edges.len()).collect();
    for attempt in 0..6 {
        let extra = if attempt == 0 { 0 } else { 1 << attempt };
        let mut measure = Measure {
            diagram: &d,
            blocks: (0..d.nodes.len()).map(|_| Block::default()).collect(),
            degrees: degree.clone(),
            dx: if connected {
                longest.max(1) + 2 + extra
            } else {
                2
            },
            dy: if connected {
                edge_labels.iter().map(Vec::len).max().unwrap_or(1) + 2 + extra
            } else {
                1
            },
            ascii: options.ascii,
            scene: None,
        };
        let measured = measure.grid(&grid, options.width)?;
        let margin = if connected {
            (max_degree + 2).clamp(3, 48) + extra
        } else {
            0
        };
        let w = measured.width() + margin * 2 + longest;
        let h = measured.height() + margin * 2;
        let mut canvas = Canvas::new(w, h)?;
        let mut rects = vec![Rect::default(); d.nodes.len()];
        measured.place(
            &measure.blocks,
            &mut rects,
            margin,
            margin,
            measured.width(),
        );
        boxes(&mut canvas, &d, &measure.blocks, &rects, options)?;
        let wires = if connected {
            let mut wires = wires::Router::new(w, h)?;
            for (i, b) in measure.blocks.iter().enumerate() {
                if rects[i].w > 0 {
                    wires.obstacle(rects[i], b.children.is_some(), b.header.len());
                }
            }
            let mut completed = true;
            for position in 0..order.len() {
                let i = order[position];
                if !wires.connect(
                    &d.edges[i],
                    &rects,
                    &d.nodes,
                    (
                        &edge_labels[i],
                        edge_labels[i].iter().map(|s| s.width()).max().unwrap_or(0),
                    ),
                    options.ascii,
                    &mut work,
                )? {
                    order[..=position].rotate_right(1);
                    completed = false;
                    break;
                }
            }
            if !completed {
                continue;
            }
            wires.draw(&mut canvas, options.ascii)?;
            Some(wires)
        } else {
            None
        };
        let output = canvas.finish();
        let mut lines: Vec<_> = output.split('\n').collect();
        let top = lines
            .iter()
            .take_while(|s| s.is_empty())
            .count()
            .min(margin);
        let bottom = lines
            .iter()
            .rev()
            .take_while(|s| s.is_empty())
            .count()
            .min(margin);
        lines.truncate(lines.len().saturating_sub(bottom));
        let lines = &lines[top.min(lines.len())..];
        let indent = lines
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.bytes().take_while(|c| *c == b' ').count())
            .min()
            .unwrap_or(0)
            .min(margin);
        let body = lines
            .iter()
            .map(|s| &s[indent.min(s.len())..])
            .collect::<Vec<_>>()
            .join("\n");
        let body = if let Some(theme) = theme {
            let mut colors = Layer::default();
            let mut paint = |x: usize, y: usize, width: usize, style| {
                let left = x.max(indent);
                if y >= top && x + width > left {
                    colors.paint(left - indent, y - top, x + width - left, style);
                }
            };
            measured.paint(&d.nodes, &measure.blocks, &rects, theme.nodes, &mut paint);
            if let Some(wires) = wires {
                wires.paint(theme.line, theme.label, paint);
            }
            colors.apply(&body)
        } else {
            body
        };
        return Ok(if d.caption.is_empty() {
            body
        } else {
            format!("{}\n\n{body}", d.caption.join("\n"))
        });
    }
    Err("connections exceed the available routing lanes after six grid expansions".into())
}
