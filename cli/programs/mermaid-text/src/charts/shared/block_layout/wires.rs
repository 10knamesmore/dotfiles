use super::layout::Rect;
use super::*;
use crate::canvas::Canvas;
use crate::color::Paint;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use unicode_width::UnicodeWidthStr;

// Clockwise directions; the opposite of d is (d + 2) % 4.
const H: u8 = EAST | WEST;
const V: u8 = NORTH | SOUTH;
const SOLID: u8 = 255;

#[derive(Clone, Copy)]
struct Port {
    p: usize,
    anchor: usize,
    d: usize,
    wall: u8,
    bias: u32,
}

pub(super) struct Router {
    w: usize,
    h: usize,
    wall: Vec<u8>,
    clearance: Vec<u8>,
    ink: Vec<u8>,
    styles: Vec<Line>,
    symbols: FxHashMap<usize, char>,
    pub(super) labels: Vec<(usize, usize, String)>,
    pub(super) routes: Option<Vec<Vec<(usize, usize)>>>,
    search: Vec<(u32, u32)>,
    visited: Vec<usize>,
}

impl Router {
    pub fn new(w: usize, h: usize) -> Result<Self, String> {
        if w * h > 1_000_000 {
            return Err("block routing canvas exceeds one million cells".into());
        }
        Ok(Self {
            w,
            h,
            wall: vec![0; w * h],
            clearance: vec![0; w * h],
            ink: vec![0; w * h],
            styles: vec![Line::Solid; w * h],
            symbols: FxHashMap::default(),
            labels: Vec::new(),
            routes: None,
            search: Vec::new(),
            visited: Vec::new(),
        })
    }
    pub fn obstacle(&mut self, r: Rect, group: bool, header: usize) {
        for x in r.x + 1..r.right() {
            self.clearance[(r.y - 1) * self.w + x] |= H;
            self.clearance[(r.bottom() + 1) * self.w + x] |= H;
        }
        for y in r.y + 1..r.bottom() {
            self.clearance[y * self.w + r.x - 1] |= V;
            self.clearance[y * self.w + r.right() + 1] |= V;
        }
        for y in r.y..=r.bottom() {
            let row = &mut self.wall[y * self.w + r.x..=y * self.w + r.right()];
            if !group || (y > r.y && y <= r.y + header) {
                row.fill(SOLID);
            } else if y == r.y || y == r.bottom() {
                row.fill(H);
                row[0] = SOLID;
                row[r.w - 1] = SOLID;
            } else {
                row[0] = V;
                row[r.w - 1] = V;
            }
        }
    }
    fn step(&self, p: usize, d: usize) -> Option<usize> {
        match d {
            0 if p >= self.w => Some(p - self.w),
            1 if p % self.w + 1 < self.w => Some(p + 1),
            2 if p + self.w < self.wall.len() => Some(p + self.w),
            3 if !p.is_multiple_of(self.w) => Some(p - 1),
            _ => None,
        }
    }
    fn ports(&self, r: Rect, group: bool) -> Vec<Port> {
        let mut ports = Vec::new();
        let mut add = |x, y, normal, bias| {
            let anchor = y * self.w + x;
            if self.symbols.contains_key(&anchor) || self.ink[anchor] != 0 {
                return;
            }
            for d in [normal, (normal + 2) % 4]
                .into_iter()
                .take(if group { 2 } else { 1 })
            {
                if let Some(p) = self.step(anchor, d)
                    && self.wall[p] == 0
                    && self.ink[p] == 0
                    && !self.symbols.contains_key(&p)
                {
                    ports.push(Port {
                        p,
                        anchor,
                        d,
                        wall: if normal % 2 == 0 { H } else { V },
                        bias,
                    });
                }
            }
        };
        for x in r.x + 1..r.right() {
            let bias = (x.abs_diff(r.x + r.w / 2) as u32) * 3;
            add(x, r.y, 0, bias);
            add(x, r.bottom(), 2, bias);
        }
        for y in r.y + 1..r.bottom() {
            let bias = (y.abs_diff(r.y + r.h / 2) as u32) * 4;
            add(r.right(), y, 1, bias);
            add(r.x, y, 3, bias);
        }
        ports
    }
    // Search on (cell, incoming direction). Crossing a straight wire or a
    // container wall is legal only at right angles, without turning on it.
    fn path(
        &mut self,
        starts: &[Port],
        ends: &[Port],
        target: Rect,
        label: (usize, usize),
        work: &mut usize,
    ) -> Result<Option<(Port, Port, Vec<usize>)>, String> {
        if starts.is_empty() || ends.is_empty() {
            return Ok(None);
        }
        // A labeled route must pass a place where the full caption fits.
        // The extra search state distinguishes paths that already passed one.
        let modes = if label.0 == 0 { 1 } else { 2 };
        let states = self.wall.len() * 4 * modes;
        for key in self.visited.drain(..) {
            self.search[key].0 = u32::MAX;
        }
        if self.search.len() < states {
            self.search.resize(states, (u32::MAX, 0));
        }
        let mut queue = BinaryHeap::new();
        let goals: FxHashMap<_, _> = ends.iter().map(|g| (g.p * 4 + (g.d + 2) % 4, *g)).collect();
        let distance = |p: usize| {
            let (x, y) = (p % self.w, p / self.w);
            ((target.x.saturating_sub(x)
                + x.saturating_sub(target.right())
                + target.y.saturating_sub(y)
                + y.saturating_sub(target.bottom()))
            .saturating_sub(1)
                * 10) as u32
        };
        for s in starts {
            let key = (s.p * 4 + s.d) * modes;
            self.search[key] = (s.bias, key as u32);
            self.visited.push(key);
            queue.push(Reverse((s.bias + distance(s.p), s.bias, key)));
        }
        let mut best = None;
        let mut best_cost = u32::MAX;
        while let Some(Reverse((bound, c, key))) = queue.pop() {
            if bound >= best_cost {
                break;
            }
            if c != self.search[key].0 {
                continue;
            }
            if *work == 0 {
                return Err("block connection search exceeded 24 million routing states".into());
            }
            *work -= 1;
            let state = key / modes;
            let placed = key % modes == modes - 1;
            if placed && let Some(&end) = goals.get(&state) {
                let total = c + end.bias;
                if total < best_cost {
                    best_cost = total;
                    best = Some((key, end));
                }
            }
            let p = state / 4;
            let incoming = state % 4;
            for d in 0..4 {
                if d == (incoming + 2) % 4
                    || (d != incoming
                        && (self.wall[p] != 0 || self.ink[p] != 0 || self.clearance[p] != 0))
                {
                    continue;
                }
                let Some(q) = self.step(p, d) else { continue };
                if self.symbols.contains_key(&q) {
                    continue;
                }
                let across = if d % 2 == 0 { H } else { V };
                if (self.wall[q] != 0 && self.wall[q] != across)
                    || (self.clearance[q] != 0 && self.clearance[q] != across)
                    || (self.ink[q] != 0 && self.ink[q] != across)
                {
                    continue;
                }
                let nc = c
                    + if d % 2 == 0 { 14 } else { 10 }
                    + if d == incoming { 0 } else { 18 }
                    + if self.ink[q] != 0 { 55 } else { 0 }
                    + if self.wall[q] != 0 { 100 } else { 0 };
                let room = modes == 2
                    && (placed
                        || (incoming == d
                            && self.wall[p] == 0
                            && self.ink[p] == 0
                            && self.label_site(p, d % 2 == 0, label.0, label.1).is_some()));
                let state = (q * 4 + d) * modes + usize::from(room);
                if nc < self.search[state].0 {
                    if self.search[state].0 == u32::MAX {
                        self.visited.push(state);
                    }
                    self.search[state] = (nc, key as u32);
                    queue.push(Reverse((nc + distance(q), nc, state)));
                }
            }
        }
        let Some((mut key, end)) = best else {
            return Ok(None);
        };
        let mut path = vec![key / modes / 4];
        while self.search[key].1 != key as u32 {
            key = self.search[key].1 as usize;
            path.push(key / modes / 4);
        }
        path.reverse();
        let start = *starts
            .iter()
            .find(|s| s.p * 4 + s.d == key / modes)
            .unwrap();
        Ok(Some((start, end, path)))
    }
    fn direction(&self, a: usize, b: usize) -> usize {
        if a + self.w == b {
            2
        } else if b + self.w == a {
            0
        } else if a < b {
            1
        } else {
            3
        }
    }
    fn label_site(&self, p: usize, vertical: bool, w: usize, h: usize) -> Option<(usize, usize)> {
        let (x, y) = ((p % self.w) as isize, (p / self.w) as isize);
        let (lw, lh) = (w as isize, h as isize);
        let beside = [(x + 2, y - lh / 2), (x - lw - 1, y - lh / 2)];
        let above = [(x - lw / 2, y - lh), (x - lw / 2, y + 1)];
        for (left, top) in if vertical { beside } else { above } {
            if left < 0 || top < 0 {
                continue;
            }
            let (left, top) = (left as usize, top as usize);
            if left + w > self.w || top + h > self.h {
                continue;
            }
            let x0 = left.saturating_sub(1);
            let x1 = (left + w + 1).min(self.w);
            if !(top..top + h).all(|y| {
                (x0..x1).all(|x| {
                    let k = y * self.w + x;
                    self.wall[k] == 0
                        && self.clearance[k] == 0
                        && self.ink[k] == 0
                        && !self.symbols.contains_key(&k)
                })
            }) {
                continue;
            }
            return Some((left, top));
        }
        None
    }
    fn add_label(&mut self, path: &[usize], text: &[String], w: usize) -> bool {
        if w == 0 {
            return true;
        }
        let mut order: Vec<_> = (0..path.len()).collect();
        order.sort_by_key(|i| i.abs_diff(path.len() / 2));
        for i in order {
            let p = path[i];
            if self.wall[p] != 0 {
                continue;
            }
            for vertical in [true, false] {
                if self.ink[p] & (if vertical { V } else { H }) == 0 {
                    continue;
                }
                if let Some((left, top)) = self.label_site(p, vertical, w, text.len()) {
                    for (dy, line) in text.iter().enumerate() {
                        self.labels.push((left, top + dy, line.clone()));
                        self.wall[(top + dy) * self.w + left.saturating_sub(1)
                            ..(top + dy) * self.w + (left + w + 1).min(self.w)]
                            .fill(SOLID);
                    }
                    return true;
                }
            }
        }
        false
    }
    pub fn connect(
        &mut self,
        e: &Edge,
        rects: &[Rect],
        nodes: &[Node],
        label: (&[String], usize),
        ascii: bool,
        work: &mut usize,
    ) -> Result<bool, String> {
        let mut starts = self.ports(rects[e.from], matches!(nodes[e.from].kind, Kind::Group(_)));
        let mut ends = self.ports(rects[e.to], matches!(nodes[e.to].kind, Kind::Group(_)));
        if e.from == e.to {
            starts.retain(|s| s.d < 2);
            ends.retain(|s| s.d >= 2);
        }
        let (label, w) = label;
        let Some((a, b, path)) = self.path(&starts, &ends, rects[e.to], (w, label.len()), work)?
        else {
            return Ok(false);
        };
        for (i, &p) in path.iter().enumerate() {
            let before = if i == 0 { a.anchor } else { path[i - 1] };
            let after = path.get(i + 1).copied().unwrap_or(b.anchor);
            self.ink[p] |= 1 << self.direction(p, before) | 1 << self.direction(p, after);
            self.styles[p] = e.line;
        }
        for (s, tip) in [(a, e.start), (b, e.end)] {
            let d = (s.d + 2) % 4;
            let glyph = match tip {
                Tip::None => None,
                Tip::Arrow => Some(
                    if ascii {
                        ['^', '>', 'v', '<']
                    } else {
                        ['▲', '▶', '▼', '◀']
                    }[d],
                ),
                Tip::Circle => Some(if ascii { 'o' } else { '○' }),
                Tip::Cross => Some(if ascii { 'x' } else { '×' }),
            };
            if let Some(ch) = glyph {
                self.symbols.insert(s.p, ch);
                self.symbols.insert(s.anchor, stroke(s.wall, ascii));
            } else {
                self.symbols
                    .insert(s.anchor, stroke(s.wall | 1 << s.d, ascii));
            }
        }
        if !self.add_label(&path, label, w) {
            return Ok(false);
        }
        if let Some(routes) = &mut self.routes {
            let points = [a.anchor]
                .into_iter()
                .chain(path)
                .chain([b.anchor])
                .map(|p| (p % self.w, p / self.w))
                .collect();
            routes.push(points);
        }
        Ok(true)
    }
    pub fn draw(&self, c: &mut Canvas, ascii: bool) -> Result<(), String> {
        for (p, &mask) in self.ink.iter().enumerate() {
            if mask != 0 {
                let mask = mask | self.wall[p];
                let ch = match (self.styles[p], mask, ascii) {
                    (Line::Thick, H, true) => '=',
                    (Line::Thick, V, true) => '!',
                    (Line::Dotted, H, true) => '.',
                    (Line::Dotted, V, true) => ':',
                    (Line::Thick, H, false) => '━',
                    (Line::Thick, V, false) => '┃',
                    (Line::Thick, 3, false) => '┗',
                    (Line::Thick, 6, false) => '┏',
                    (Line::Thick, 9, false) => '┛',
                    (Line::Thick, 12, false) => '┓',
                    (Line::Dotted, H, false) => '┄',
                    (Line::Dotted, V, false) => '┆',
                    _ => stroke(mask, ascii),
                };
                c.put(p % self.w, p / self.w, ch)?;
            }
        }
        for (&p, &ch) in &self.symbols {
            c.put(p % self.w, p / self.w, ch)?;
        }
        for (x, y, s) in &self.labels {
            c.text(*x, *y, s)?;
        }
        Ok(())
    }

    pub(super) fn paint(
        &self,
        line: Paint,
        caption: Paint,
        mut paint: impl FnMut(usize, usize, usize, Paint),
    ) {
        if line.fg.is_some() {
            for (p, &mask) in self.ink.iter().enumerate() {
                if mask != 0 && self.wall[p] == 0 {
                    paint(p % self.w, p / self.w, 1, line);
                }
            }
            for &p in self.symbols.keys() {
                if self.wall[p] == 0 {
                    paint(p % self.w, p / self.w, 1, line);
                }
            }
        }
        for (x, y, text) in &self.labels {
            paint(*x, *y, text.width(), caption);
        }
    }
}

fn stroke(mask: u8, ascii: bool) -> char {
    if ascii {
        return match mask {
            V => '|',
            H => '-',
            _ => '+',
        };
    }
    match mask {
        V => '│',
        H => '─',
        3 => '└',
        6 => '┌',
        9 => '┘',
        12 => '┐',
        7 => '├',
        11 => '┴',
        13 => '┤',
        14 => '┬',
        _ => '┼',
    }
}
