use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rustc_hash::FxHashMap;
use unicode_width::UnicodeWidthStr;

use super::{
    Diagram, Edge, End, LinkPaint, Palette, Side,
    layout::{Rect, wrap},
};
use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint, Style},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Empty,
    Solid,
    RimH,
    RimV,
    WireH,
    WireV,
}

impl Cell {
    fn crossing_cost(self, dir: usize) -> Option<usize> {
        match self {
            Self::Empty => Some(0),
            Self::RimV if dir % 2 == 1 => Some(12),
            Self::RimH if dir.is_multiple_of(2) => Some(12),
            Self::WireV if dir % 2 == 1 => Some(16),
            Self::WireH if dir.is_multiple_of(2) => Some(16),
            _ => None,
        }
    }
}

pub(super) struct Drawing {
    canvas: Canvas,
    router: Router,
    ascii: bool,
    corners: [char; 4],
}

pub(super) struct Router {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    search_budget: usize,
}

pub(super) struct Route {
    pub path: Vec<usize>,
    pub pins: [usize; 2],
    pub sides: [Side; 2],
}

const SIDES: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];

fn frame_corners(options: &Options) -> [char; 4] {
    let natural = if options.ascii {
        ['+'; 4]
    } else {
        ['╭', '╮', '╰', '╯']
    };
    options.corners.resolve(options.ascii, natural)
}

impl Drawing {
    pub fn new(width: usize, height: usize, options: &Options) -> Result<Self, String> {
        Ok(Self {
            canvas: Canvas::new(width, height)?,
            router: Router::new(width, height),
            ascii: options.ascii,
            corners: frame_corners(options),
        })
    }

    fn put(&mut self, x: usize, y: usize, u: char, a: char) -> Result<(), String> {
        self.canvas.put(x, y, if self.ascii { a } else { u })
    }

    pub fn frame(
        &mut self,
        r: Rect,
        lines: &[String],
        boundary: bool,
        align_right: bool,
        style: Style,
    ) -> Result<(), String> {
        self.router
            .frame(r, boundary, align_right, lines.iter().map(|s| s.width()));
        let (left, right, top, bottom) = (r.x, r.x + r.w - 1, r.y, r.y + r.h - 1);
        for x in left + 1..right {
            for y in [top, bottom] {
                self.put(x, y, if boundary { '╌' } else { '─' }, '-')?;
            }
        }
        for y in top + 1..bottom {
            for x in [left, right] {
                self.put(x, y, if boundary { '╎' } else { '│' }, '|')?;
            }
        }
        let [tl, tr, bl, br] = self.corners;
        for (x, y, ch) in [
            (left, top, tl),
            (right, top, tr),
            (left, bottom, bl),
            (right, bottom, br),
        ] {
            self.put(x, y, ch, '+')?;
        }
        let border = style.border();
        let fill = Paint {
            fg: None,
            bg: style.fill,
        };
        for y in [top, bottom] {
            self.canvas.paint(left, y, r.w, border);
        }
        for y in top + 1..bottom {
            self.canvas.paint(left, y, 1, border);
            self.canvas.paint(left + 1, y, r.w - 2, fill);
            self.canvas.paint(right, y, 1, border);
        }
        for (dy, s) in lines.iter().enumerate() {
            let width = s.width();
            let x = if align_right {
                right - 1 - width
            } else {
                left + 2
            };
            let y = top + 1 + dy;
            self.canvas.text(x, y, s)?;
            self.canvas.paint(x, y, width, style.ink());
        }
        Ok(())
    }
}

impl Router {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::Empty; width * height],
            search_budget: 1_500_000,
        }
    }

    pub fn frame(
        &mut self,
        r: Rect,
        boundary: bool,
        align_right: bool,
        lines: impl Iterator<Item = usize>,
    ) {
        let (left, right, top, bottom) = (r.x, r.x + r.w - 1, r.y, r.y + r.h - 1);
        for y in top..=bottom {
            let row = &mut self.cells[y * self.width + left..=y * self.width + right];
            if !boundary {
                row.fill(Cell::Solid);
            } else if y == top || y == bottom {
                row.fill(Cell::RimH);
                row[0] = Cell::Solid;
                row[r.w - 1] = Cell::Solid;
            } else {
                row[0] = Cell::RimV;
                row[r.w - 1] = Cell::RimV;
            }
        }
        if boundary {
            for (dy, width) in lines.enumerate() {
                let x = if align_right {
                    right - 1 - width
                } else {
                    left + 2
                };
                let y = top + 1 + dy;
                self.cells[y * self.width + x - 1..=y * self.width + x + width].fill(Cell::Solid);
            }
        }
    }

    pub fn coordinates(&self, i: usize) -> (usize, usize) {
        (i % self.width, i / self.width)
    }

    fn step(&self, i: usize, dir: usize) -> Option<usize> {
        match dir {
            0 => i.checked_sub(self.width),
            1 => (i % self.width + 1 < self.width).then_some(i + 1),
            2 => (i / self.width + 1 < self.height).then_some(i + self.width),
            _ => (!i.is_multiple_of(self.width)).then(|| i - 1),
        }
    }

    fn distance(&self, a: usize, b: usize) -> usize {
        (a % self.width).abs_diff(b % self.width) + (a / self.width).abs_diff(b / self.width)
    }

    fn port(&self, side: Side, box_: Rect, member: Rect) -> Option<(usize, usize)> {
        let dir = SIDES.iter().position(|&s| s == side)?;
        let (start, len, preferred) = if dir % 2 == 0 {
            (box_.x + 1, box_.w - 2, member.x + member.w / 2)
        } else {
            (box_.y + 1, box_.h - 2, member.y + member.h / 2)
        };
        let mut best = None;
        for t in start..start + len {
            let (x, y) = match side {
                Side::Left => (box_.x, t),
                Side::Right => (box_.x + box_.w - 1, t),
                Side::Top => (t, box_.y),
                Side::Bottom => (t, box_.y + box_.h - 1),
            };
            let pin = y * self.width + x;
            let outer = self.step(pin, dir)?;
            if self.cells[outer] != Cell::Empty {
                continue;
            }
            if self
                .step(outer, dir)
                .and_then(|i| self.cells[i].crossing_cost(dir))
                .is_none()
            {
                continue;
            }
            let key = (t.abs_diff(preferred), pin, outer);
            if best.is_none_or(|v| key < v) {
                best = Some(key);
            }
        }
        best.map(|(_, pin, outer)| (pin, outer))
    }

    fn path(&mut self, start: usize, dir: usize, end: usize, end_dir: usize) -> Option<Vec<usize>> {
        if start == end {
            return None;
        }
        let initial = start * 4 + dir;
        let mut seen = FxHashMap::default();
        seen.insert(initial, (0usize, initial));
        let mut open = BinaryHeap::from([Reverse((self.distance(start, end) * 2, 0, initial))]);
        let mut local = 0;
        while let Some(Reverse((_, cost, state))) = open.pop() {
            if self.search_budget == 0 || local >= 6500 {
                return None;
            }
            self.search_budget -= 1;
            local += 1;
            if seen[&state].0 != cost {
                continue;
            }
            let (i, heading) = (state / 4, state % 4);
            if i == end && heading == (end_dir + 2) % 4 {
                let mut result = vec![i];
                let mut s = state;
                while s != initial {
                    s = seen[&s].1;
                    result.push(s / 4);
                }
                result.reverse();
                return Some(result);
            }
            for next_dir in 0..4 {
                let turn = heading != next_dir;
                if next_dir == (heading + 2) % 4
                    || (turn && (i == start || self.cells[i] != Cell::Empty))
                {
                    continue;
                }
                let Some(j) = self.step(i, next_dir) else {
                    continue;
                };
                if j == start || (j == end && next_dir != (end_dir + 2) % 4) {
                    continue;
                }
                let Some(extra) = self.cells[j].crossing_cost(next_dir) else {
                    continue;
                };
                let g = cost + 2 + extra + usize::from(turn) * 5;
                let s = j * 4 + next_dir;
                if seen.get(&s).is_none_or(|(old, _)| g < *old) {
                    seen.insert(s, (g, state));
                    open.push(Reverse((g + self.distance(j, end) * 2, g, s)));
                }
            }
        }
        None
    }

    pub fn label(&mut self, path: &[usize], w: usize, h: usize) -> Option<(usize, usize)> {
        if w == 0 || w > self.width {
            return None;
        }
        let mut best = None;
        for (i, &p) in path.iter().enumerate() {
            let (x, y) = (p % self.width, p / self.width);
            let candidates = match self.cells[p] {
                Cell::WireH => [
                    (Some(x.saturating_sub(w / 2)), y.checked_sub(h)),
                    (Some(x.saturating_sub(w / 2)), Some(y + 1)),
                ],
                Cell::WireV => [
                    (Some(x + 2), y.checked_sub(h / 2)),
                    (x.checked_sub(w + 1), y.checked_sub(h / 2)),
                ],
                _ => continue,
            };
            for (rank, (cx, cy)) in candidates.into_iter().enumerate() {
                let Some(cx) = cx else { continue };
                let Some(cy) = cy.filter(|&y| y + h <= self.height) else {
                    continue;
                };
                if cx + w > self.width {
                    continue;
                }
                if (cy..cy + h).contains(&y)
                    && self.cells[y * self.width + if cx > x { x + 1 } else { x - 1 }]
                        != Cell::Empty
                {
                    continue;
                }
                if !(cy..cy + h).all(|y| {
                    let start = y * self.width + cx;
                    self.cells[start..start + w]
                        .iter()
                        .all(|&c| c == Cell::Empty)
                }) {
                    continue;
                }
                let key = (i.abs_diff(path.len() / 2) + rank, cx, cy);
                if best.is_none_or(|v| key < v) {
                    best = Some(key);
                }
            }
        }
        let (_, x, y) = best?;
        for row in y..y + h {
            let start = row * self.width + x;
            self.cells[start..start + w].fill(Cell::Solid);
        }
        Some((x, y))
    }

    pub fn route(
        &mut self,
        d: &Diagram,
        edge: &Edge,
        boxes: &[Rect],
    ) -> Result<Option<Route>, String> {
        let (a, b) = (boxes[d.endpoint(&edge.from)?], boxes[d.endpoint(&edge.to)?]);
        let (ax, ay) = (a.x + a.w / 2, a.y + a.h / 2);
        let (bx, by) = (b.x + b.w / 2, b.y + b.h / 2);
        let default = if ax.abs_diff(bx) > ay.abs_diff(by) * 2 {
            if bx >= ax { Side::Right } else { Side::Left }
        } else if by >= ay {
            Side::Bottom
        } else {
            Side::Top
        };
        let fs = edge.from.side.unwrap_or(default);
        let ts = edge.to.side.unwrap_or(fs.opposite());
        let mut sides = vec![(fs, ts)];
        if edge.from.side.is_none() && edge.to.side.is_none() {
            sides.extend([
                (Side::Right, Side::Right),
                (Side::Left, Side::Left),
                (Side::Bottom, Side::Bottom),
                (Side::Top, Side::Top),
                (ts, fs),
            ]);
        }
        let dir = |s| SIDES.iter().position(|&v| v == s).unwrap();
        Ok(sides.into_iter().find_map(|(fs, ts)| {
            let (ap, ao) = self.port(fs, a, boxes[d.ids[&edge.from.id]])?;
            let (bp, bo) = self.port(ts, b, boxes[d.ids[&edge.to.id]])?;
            if ap == bp {
                return None;
            }
            self.path(ao, dir(fs), bo, dir(ts)).map(|path| Route {
                path,
                pins: [ap, bp],
                sides: [fs, ts],
            })
        }))
    }

    fn wire_cell(&mut self, before: usize, p: usize, after: usize, arrow: bool) -> (u8, Cell) {
        let mut mask = 0;
        for j in 0..4 {
            if self.step(p, j).is_some_and(|v| v == before || v == after) {
                mask |= 1 << j;
            }
        }
        let old = self.cells[p];
        self.cells[p] = match (old, arrow, mask) {
            (Cell::Empty, false, 10) => Cell::WireH,
            (Cell::Empty, false, 5) => Cell::WireV,
            _ => Cell::Solid,
        };
        (mask, old)
    }

    pub fn reserve(&mut self, route: &Route, arrows: [bool; 2]) {
        for (i, &p) in route.path.iter().enumerate() {
            let (prev, next) = route.neighbors(i);
            self.wire_cell(prev, p, next, route.tip(i, arrows).is_some());
        }
        for pin in route.pins {
            self.cells[pin] = Cell::Solid;
        }
    }
}

impl Route {
    fn neighbors(&self, i: usize) -> (usize, usize) {
        (
            i.checked_sub(1)
                .map_or(self.pins[0], |previous| self.path[previous]),
            self.path.get(i + 1).copied().unwrap_or(self.pins[1]),
        )
    }

    fn tip(&self, i: usize, arrows: [bool; 2]) -> Option<Side> {
        if i == 0 && arrows[0] {
            Some(self.sides[0].opposite())
        } else if i + 1 == self.path.len() && arrows[1] {
            Some(self.sides[1].opposite())
        } else {
            None
        }
    }
}

impl Drawing {
    fn label(&mut self, path: &[usize], s: &str, paint: Paint) -> Result<bool, String> {
        if let Some((x, y)) = self.router.label(path, s.width(), 1) {
            self.canvas.text(x, y, s)?;
            self.canvas.paint(x, y, s.width(), paint);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn connect(
        &mut self,
        d: &Diagram,
        edge: &Edge,
        boxes: &[Rect],
        n: usize,
        paint: LinkPaint,
    ) -> Result<bool, String> {
        let Some(route) = self.router.route(d, edge, boxes)? else {
            return Ok(false);
        };
        for (idx, &p) in route.path.iter().enumerate() {
            let (prev, next) = route.neighbors(idx);
            let arrow = route.tip(idx, [edge.from.arrow, edge.to.arrow]);
            let (mask, cell) = self.router.wire_cell(prev, p, next, arrow.is_some());
            let (mut u, mut a) = match mask {
                5 => ('│', '|'),
                10 => ('─', '-'),
                3 => ('╰', '+'),
                6 => ('╭', '+'),
                9 => ('╯', '+'),
                12 => ('╮', '+'),
                _ => ('┼', '+'),
            };
            if cell != Cell::Empty {
                (u, a) = if matches!(cell, Cell::WireH | Cell::WireV) {
                    ('╪', '+')
                } else {
                    ('┼', '+')
                };
            }
            if let Some(side) = arrow {
                (u, a) = match side {
                    Side::Left => ('◀', '<'),
                    Side::Right => ('▶', '>'),
                    Side::Top => ('▲', '^'),
                    Side::Bottom => ('▼', 'v'),
                };
            }
            let (x, y) = self.router.coordinates(p);
            self.put(x, y, u, a)?;
            self.canvas.paint(
                x,
                y,
                1,
                if arrow.is_some() {
                    paint.arrow
                } else {
                    paint.line
                },
            );
        }
        for (p, s) in route.pins.into_iter().zip(route.sides) {
            let (x, y) = self.router.coordinates(p);
            self.router.cells[p] = Cell::Solid;
            self.put(
                x,
                y,
                match s {
                    Side::Top => '┴',
                    Side::Bottom => '┬',
                    Side::Left => '┤',
                    Side::Right => '├',
                },
                '+',
            )?;
            self.canvas.paint(x, y, 1, paint.line);
        }
        let label = edge
            .label
            .replace("<br/>", " ")
            .replace("<br />", " ")
            .replace("<br>", " ")
            .replace(['\n', '\t'], " ");
        let group = edge.from.group || edge.to.group;
        if !group && (label.is_empty() || self.label(&route.path, &label, paint.text)?) {
            return Ok(true);
        }
        self.label(&route.path, &format!("[{n}]"), paint.text)?;
        Ok(false)
    }

    pub fn finish(self) -> String {
        self.canvas.finish()
    }
}

pub(super) fn connection_panel(
    d: &Diagram,
    n: usize,
    edge: &Edge,
    options: &Options,
    palette: &Palette,
    paint: LinkPaint,
) -> Result<(String, usize), String> {
    let endpoint = |end: &End| {
        let node = &d.nodes[d.endpoint(end)?];
        let mut s = if end.group {
            format!("{} / {}", node.id, end.id)
        } else {
            end.id.clone()
        };
        if let Some(side) = end.side {
            s.push(':');
            s.push_str(side.letter());
        }
        Ok::<_, String>((s, palette.element(node)))
    };
    let (left, left_style) = endpoint(&edge.from)?;
    let (right, right_style) = endpoint(&edge.to)?;
    let (line, front, back, v, send, receive) = if options.ascii {
        ("-", "<", ">", "|", "+", "+")
    } else {
        ("─", "◀", "▶", "│", "├", "┤")
    };
    let [tl, tr, bl, br] = frame_corners(options);
    let (left_w, right_w) = (left.width() + 4, right.width() + 4);
    let a = line.repeat(left_w - 2);
    let b = line.repeat(right_w - 2);
    let mut lines = wrap(&format!("[{n}] {}", edge.label), options.width.max(16));
    let mut layer = Layer::default();
    for (y, text) in lines.iter().enumerate() {
        layer.paint(0, y, text.width(), paint.text);
    }
    let y = lines.len();
    lines.push(format!("    {tl}{a}{tr}    {tl}{b}{tr}"));
    lines.push(format!(
        "    {v} {left} {send}{}{line}{line}{}{receive} {right} {v}",
        if edge.from.arrow { front } else { line },
        if edge.to.arrow { back } else { line }
    ));
    lines.push(format!("    {bl}{a}{br}    {bl}{b}{br}"));
    let left_x = 4;
    let right_x = left_x + left_w + 4;
    for (x, w, style) in [
        (left_x, left_w, left_style),
        (right_x, right_w, right_style),
    ] {
        let border = style.border();
        layer.paint(x, y, w, border);
        layer.paint(x, y + 1, 1, border);
        layer.paint(x + 1, y + 1, w - 2, style.ink());
        layer.paint(x + w - 1, y + 1, 1, border);
        layer.paint(x, y + 2, w, border);
    }
    let left_port = left_x + left_w - 1;
    let right_port = right_x;
    layer.paint(left_port, y + 1, right_port - left_port + 1, paint.line);
    if edge.from.arrow {
        layer.paint(left_port + 1, y + 1, 1, paint.arrow);
    }
    if edge.to.arrow {
        layer.paint(right_port - 1, y + 1, 1, paint.arrow);
    }
    let plain = lines.join("\n");
    Ok((layer.apply(&plain), plain.len() + 1))
}
