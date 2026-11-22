use super::{Direction, Edge, Line, Tip, layout::Rect};
use rustc_hash::FxHashMap;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(super) struct Point(pub usize, pub usize);

impl Point {
    fn step(self, d: usize) -> Option<Self> {
        match d {
            0 => Some(Self(self.0 + 1, self.1)),
            1 => Some(Self(self.0, self.1 + 1)),
            2 => Some(Self(self.0.checked_sub(1)?, self.1)),
            _ => Some(Self(self.0, self.1.checked_sub(1)?)),
        }
    }
    fn distance(self, other: Self) -> usize {
        self.0.abs_diff(other.0) + self.1.abs_diff(other.1)
    }
    fn direction(self, to: Self) -> usize {
        if self.0 < to.0 {
            0
        } else if self.1 < to.1 {
            1
        } else if self.0 > to.0 {
            2
        } else {
            3
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct Cell {
    blocked: bool,
    frame: bool,
    pub strokes: u8,
    pub line: Option<Line>,
    pub tip: Option<(Tip, usize)>,
}

pub(super) struct Routes {
    cells: Vec<Cell>,
    w: usize,
    h: usize,
}

impl Routes {
    pub(super) fn new(w: usize, h: usize) -> Self {
        Self {
            cells: vec![Cell::default(); w * h],
            w,
            h,
        }
    }

    fn cell(&self, p: Point) -> Cell {
        self.cells[p.1 * self.w + p.0]
    }
    fn mark(&mut self, p: Point) -> &mut Cell {
        &mut self.cells[p.1 * self.w + p.0]
    }
    fn inside(&self, p: Point) -> bool {
        p.0 < self.w && p.1 < self.h
    }

    pub(super) fn strokes(&self) -> impl Iterator<Item = (Point, Cell)> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.strokes != 0)
            .map(|(i, &cell)| (Point(i % self.w, i / self.w), cell))
    }

    pub(super) fn block_text(&mut self, p: Point, width: usize) {
        for x in p.0..p.0 + width {
            self.mark(Point(x, p.1)).blocked = true;
        }
    }

    pub(super) fn block(&mut self, Rect { x, y, w, h }: Rect, frame: bool) {
        for dy in 0..h {
            for dx in 0..w {
                let p = Point(x + dx, y + dy);
                if !frame {
                    self.mark(p).blocked = true;
                }
                if dx == 0 || dy == 0 || dx + 1 == w || dy + 1 == h {
                    self.mark(p).frame = frame;
                }
            }
        }
    }

    fn ports(&self, r: Rect) -> Vec<(Point, usize)> {
        let boundary = [
            Point(r.x + r.w - 1, r.y + r.h / 2),
            Point(r.x + r.w / 2, r.y + r.h - 1),
            Point(r.x, r.y + r.h / 2),
            Point(r.x + r.w / 2, r.y),
        ];
        boundary
            .into_iter()
            .enumerate()
            .filter_map(|(dir, p)| {
                let next = p.step(dir)?;
                (self.inside(next) && !self.cell(next).blocked).then_some((next, dir))
            })
            .collect()
    }

    pub(super) fn route(
        &self,
        from: Rect,
        to: Rect,
        dir: Direction,
        loopback: bool,
    ) -> Option<(Vec<Point>, usize, usize)> {
        let preferred = usize::from(!dir.horizontal) + 2 * usize::from(dir.reverse);
        let mut starts = self.ports(from);
        let mut ends = self.ports(to);
        if loopback {
            starts.retain(|&(_, d)| d == preferred);
            ends.retain(|&(_, d)| d == (preferred + 3) % 4);
        }
        if ends.is_empty() {
            return None;
        }
        let estimate = |p: Point| {
            ends.iter()
                .map(|&(end, _)| p.distance(end) * 4)
                .min()
                .unwrap()
        };
        let mut queue = BinaryHeap::new();
        let mut visits = FxHashMap::default();
        for (p, d) in starts {
            let cost = if d == preferred { 0 } else { 5 };
            visits.insert((p, d), (cost, None));
            queue.push(Reverse((cost + estimate(p), cost, p, d)));
        }
        let mut best = None;
        while let Some(Reverse((bound, cost, p, d))) = queue.pop() {
            if best.is_some_and(|(total, _, _, _)| bound >= total) {
                break;
            }
            if visits[&(p, d)].0 != cost {
                continue;
            }
            for &(end, port) in &ends {
                if end == p {
                    let total = cost
                        + if (port + 2) % 4 == d { 0 } else { 22 }
                        + if port == (preferred + 2) % 4 { 0 } else { 5 };
                    if best.is_none_or(|(old, _, _, _)| total < old) {
                        best = Some((total, p, d, port));
                    }
                }
            }
            if visits.len() > 120_000 {
                break;
            }
            for n in 0..4 {
                if n == (d + 2) % 4 {
                    continue;
                }
                let Some(q) = p.step(n) else {
                    continue;
                };
                if !self.inside(q) {
                    continue;
                }
                let cell = self.cell(q);
                if cell.blocked {
                    continue;
                }
                let extra = if cell.tip.is_some() {
                    60
                } else if cell.strokes & (1 << n | 1 << ((n + 2) % 4)) != 0 {
                    22
                } else if cell.strokes != 0 {
                    8
                } else {
                    0
                };
                let close_to_box = [(n + 1) % 4, (n + 3) % 4].into_iter().any(|side| {
                    q.step(side)
                        .is_some_and(|pt| self.inside(pt) && self.cell(pt).blocked)
                });
                let score = cost
                    + 4
                    + extra
                    + usize::from(n != d) * 8
                    + usize::from(close_to_box) * 4
                    + usize::from(cell.frame) * 3;
                if visits.get(&(q, n)).is_none_or(|&(v, _)| score < v) {
                    visits.insert((q, n), (score, Some((p, d))));
                    queue.push(Reverse((score + estimate(q), score, q, n)));
                }
            }
        }
        let (_, end, last, port) = best?;
        let mut state = (end, last);
        let mut path = vec![end];
        while let Some(prev) = visits[&state].1 {
            path.push(prev.0);
            state = prev;
        }
        path.reverse();
        Some((path, state.1, port))
    }

    pub(super) fn trace(&mut self, path: &[Point], source: usize, target: usize, edge: &Edge) {
        for (i, &p) in path.iter().enumerate() {
            let before = if i == 0 {
                (source + 2) % 4
            } else {
                p.direction(path[i - 1])
            };
            let after = if i + 1 == path.len() {
                (target + 2) % 4
            } else {
                p.direction(path[i + 1])
            };
            let c = self.mark(p);
            c.strokes |= 1 << before | 1 << after;
            c.line = Some(edge.line);
        }
        if edge.tip != Tip::None {
            self.mark(*path.last().unwrap()).tip = Some((edge.tip, (target + 2) % 4));
        }
    }

    pub(super) fn label_position(&mut self, path: &[Point], w: usize) -> Option<Point> {
        let mut order: Vec<_> = (0..path.len()).collect();
        order.sort_by_key(|&i| (2 * i).abs_diff(path.len() - 1));
        for i in order {
            let Point(x, y) = path[i];
            let candidates = [
                (x.checked_sub(w / 2), y.checked_sub(1)),
                (x.checked_sub(w / 2), Some(y + 1)),
                (x.checked_add(2), Some(y)),
                (x.checked_sub(w + 1), Some(y)),
                (x.checked_sub(w / 2), y.checked_sub(2)),
                (x.checked_sub(w / 2), Some(y + 2)),
            ];
            for (x, y) in candidates {
                let (Some(x), Some(y)) = (x, y) else {
                    continue;
                };
                if x + w > self.w || y >= self.h {
                    continue;
                }
                if (x..x + w).all(|cx| {
                    let cell = self.cell(Point(cx, y));
                    !cell.blocked && !cell.frame && cell.strokes == 0
                }) {
                    let position = Point(x, y);
                    self.block_text(position, w);
                    return Some(position);
                }
            }
        }
        None
    }
}
