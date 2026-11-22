use std::collections::BTreeSet;

use unicode_width::UnicodeWidthStr;

use crate::{canvas::Canvas, color::Paint};

use super::{
    End, Link, Mark,
    card::{columns, wrap},
};

#[derive(Clone, Copy)]
pub(super) enum Axis {
    Vertical,
    Horizontal,
}

impl Axis {
    pub(super) fn xy(self, along: usize, across: usize) -> (usize, usize) {
        match self {
            Self::Vertical => (across, along),
            Self::Horizontal => (along, across),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Port {
    pub(super) along: usize,
    pub(super) across: usize,
}

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl Rect {
    fn contains(self, x: usize, y: usize) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }
}

pub(super) struct Router<'a> {
    axis: Axis,
    size: (usize, usize),
    cards: &'a [Rect],
    occupied: BTreeSet<(usize, usize)>,
    ascii: bool,
}

pub(super) struct Wire {
    cells: Vec<(usize, usize, char)>,
    ports: [(usize, usize, char); 2],
    text: Vec<(usize, usize, String, TextRole)>,
    axis: Axis,
    ends: [Port; 2],
    bend: usize,
}

enum TextRole {
    Marker,
    Caption,
}

pub(super) struct Captions<'a> {
    pub(super) ends: [End<'a>; 2],
    pub(super) label: &'a str,
}

impl Router<'_> {
    pub(super) fn new(axis: Axis, size: (usize, usize), cards: &[Rect], ascii: bool) -> Router<'_> {
        Router {
            axis,
            size,
            cards,
            occupied: BTreeSet::new(),
            ascii,
        }
    }

    pub(super) fn route(
        &mut self,
        from: Port,
        to: Port,
        bend: usize,
        link: Link,
        captions: &Captions<'_>,
    ) -> Option<Wire> {
        let axis = self.axis;
        let (horizontal, vertical) = match (self.ascii, link) {
            (true, Link::Solid) => ('-', '|'),
            (true, Link::Dashed) => ('.', ':'),
            (_, Link::Solid) => ('─', '│'),
            (_, Link::Dashed) => ('┄', '┊'),
        };
        let (along_stroke, cross_stroke) = match axis {
            Axis::Vertical => (vertical, horizontal),
            Axis::Horizontal => (horizontal, vertical),
        };
        let corner = |before: bool, after: bool| {
            if self.ascii {
                return '+';
            }
            match (before, after) {
                (false, false) => '┌',
                (false, true) => '┐',
                (true, false) => '└',
                (true, true) => '┘',
            }
        };
        let [start, end] = if self.ascii {
            ['+', '+']
        } else {
            match axis {
                Axis::Vertical => ['┬', '┴'],
                Axis::Horizontal => ['├', '┤'],
            }
        };
        let at_card = |boundary: usize, across: usize, ch: char| {
            let (x, y) = axis.xy(boundary, across);
            (x, y, ch)
        };
        let start_cell = at_card(from.along, from.across, start);
        let end_cell = at_card(to.along, to.across, end);
        let mut wire = Wire {
            cells: Vec::new(),
            ports: [start_cell, end_cell],
            text: Vec::new(),
            axis,
            ends: [from, to],
            bend,
        };
        let mut add = |along, across, stroke| {
            let (x, y) = axis.xy(along, across);
            wire.cells.push((x, y, stroke));
        };
        if from.across == to.across {
            for along in from.along + 1..to.along {
                add(along, from.across, along_stroke);
            }
        } else {
            let stem = |end: End<'_>, start| {
                let marker = marker(end.mark, axis, start, self.ascii);
                match axis {
                    Axis::Horizontal => marker.width(),
                    Axis::Vertical => usize::from(!marker.is_empty()),
                }
            };
            if bend <= from.along + stem(captions.ends[0], true)
                || bend >= to.along - stem(captions.ends[1], false)
            {
                return None;
            }
            let increasing = from.across < to.across;
            let (first, last) = match axis {
                Axis::Vertical => (corner(true, !increasing), corner(false, increasing)),
                Axis::Horizontal => (corner(!increasing, true), corner(increasing, false)),
            };
            for along in from.along + 1..bend {
                add(along, from.across, along_stroke);
            }
            add(bend, from.across, first);
            for across in from.across.min(to.across) + 1..from.across.max(to.across) {
                add(bend, across, cross_stroke);
            }
            add(bend, to.across, last);
            for along in bend + 1..to.along {
                add(along, to.across, along_stroke);
            }
        }
        if wire.cells.iter().any(|&(x, y, _)| !self.free(x, y)) {
            return None;
        }
        if wire
            .ports
            .iter()
            .any(|&(x, y, _)| self.occupied.contains(&(x, y)))
        {
            return None;
        }
        for (index, port) in [from, to].into_iter().enumerate() {
            let end = captions.ends[index];
            let mark = marker(end.mark, axis, index == 0, self.ascii);
            if !mark.is_empty() {
                let along = if index == 0 {
                    port.along + 1
                } else if matches!(axis, Axis::Horizontal) {
                    port.along.checked_sub(mark.width())?
                } else {
                    port.along.checked_sub(1)?
                };
                let (x, y) = axis.xy(along, port.across);
                if !(x..x + mark.width()).all(|x| self.free(x, y))
                    || wire.text.iter().any(|&(other, row, ref text, _)| {
                        row == y && other < x + mark.width() && x < other + columns(text)
                    })
                {
                    return None;
                }
                wire.text.push((x, y, mark.to_owned(), TextRole::Marker));
            }
            if let Some(count) = end.count {
                self.count(
                    &mut wire,
                    port,
                    index == 0,
                    count,
                    to.along - from.along - 1,
                )?;
            }
        }
        if !captions.label.trim().is_empty() {
            self.label(&mut wire, from, to, bend, captions.label)?;
        }
        for &(x, y, _) in wire.cells.iter().chain(&wire.ports) {
            self.occupied.insert((x, y));
        }
        for (x, y, text, _) in &wire.text {
            self.occupied
                .extend((*x..*x + columns(text)).map(|x| (x, *y)));
        }
        Some(wire)
    }

    fn free(&self, x: usize, y: usize) -> bool {
        x < self.size.0
            && y < self.size.1
            && !self.occupied.contains(&(x, y))
            && !self.cards.iter().any(|card| card.contains(x, y))
    }

    fn caption(&self, wire: &mut Wire, x: usize, y: usize, lines: &[String]) -> bool {
        let fits = lines.iter().enumerate().all(|(offset, line)| {
            let y = y + offset;
            let width = columns(line);
            (x..x + width).all(|x| self.free(x, y))
                && !wire
                    .cells
                    .iter()
                    .any(|&(cell, row, _)| row == y && (x..x + width).contains(&cell))
                && !wire.text.iter().any(|&(other, row, ref text, _)| {
                    row == y && other <= x + width && x <= other + columns(text)
                })
        });
        if fits {
            wire.text.extend(
                lines
                    .iter()
                    .enumerate()
                    .map(|(offset, line)| (x, y + offset, line.clone(), TextRole::Caption)),
            );
        }
        fits
    }

    fn count(
        &self,
        wire: &mut Wire,
        port: Port,
        start: bool,
        text: &str,
        gap: usize,
    ) -> Option<()> {
        let axis = self.axis;
        let (x, y) = axis.xy(port.along, port.across);
        if matches!(axis, Axis::Vertical) {
            let limit = (self.size.0 - x - 1).max(x).saturating_sub(3).max(4);
            let lines = wrap(text, limit);
            let width = lines.iter().map(|line| columns(line)).max().unwrap_or(0);
            let y = if start {
                y + 1
            } else {
                y.checked_sub(lines.len())?
            };
            if self.caption(wire, x + 3, y, &lines)
                || x.checked_sub(width + 2)
                    .is_some_and(|x| self.caption(wire, x, y, &lines))
            {
                return Some(());
            }
        } else {
            let lines = wrap(text, gap.saturating_sub(2).max(4));
            let width = lines.iter().map(|line| columns(line)).max().unwrap_or(0);
            let x = if start {
                x + 2
            } else {
                x.checked_sub(width + 1)?
            };
            if y.checked_sub(lines.len())
                .is_some_and(|y| self.caption(wire, x, y, &lines))
                || self.caption(wire, x, y + 1, &lines)
            {
                return Some(());
            }
        }
        None
    }

    fn label(&self, wire: &mut Wire, a: Port, b: Port, bend: usize, text: &str) -> Option<()> {
        let lo = a.along + 1;
        let hi = b.along;
        if matches!(self.axis, Axis::Vertical) {
            let mut rows: Vec<_> = (lo..hi).collect();
            rows.sort_by_key(|&y| y.abs_diff((lo + hi) / 2));
            for y in rows {
                let (left_edge, right_edge) = if y == bend {
                    (a.across.min(b.across), a.across.max(b.across))
                } else {
                    let x = if y < bend { a.across } else { b.across };
                    (x, x)
                };
                let left = left_edge.saturating_sub(2);
                let right = self.size.0.saturating_sub(right_edge + 2);
                let lines = wrap(text, left.max(right).max(4));
                let width = lines.iter().map(|line| columns(line)).max().unwrap_or(0);
                if y + lines.len() <= hi
                    && ((width <= right && self.caption(wire, right_edge + 2, y, &lines))
                        || (width <= left && self.caption(wire, left - width, y, &lines)))
                {
                    return Some(());
                }
            }
        } else {
            let (lo, hi) = (lo + 1, hi - 1);
            let lines = wrap(text, hi - lo);
            let width = lines.iter().map(|line| columns(line)).max().unwrap_or(0);
            let x = lo + (hi - lo).saturating_sub(width) / 2;
            for y in [
                a.across.max(b.across) + 1,
                a.across.min(b.across).saturating_sub(lines.len()),
                a.across + 1,
                b.across + 1,
            ] {
                if self.caption(wire, x, y, &lines) {
                    return Some(());
                }
            }
        }
        None
    }
}

impl Wire {
    pub(super) fn points(&self) -> [(usize, usize); 4] {
        let [from, to] = self.ends;
        [
            self.axis.xy(from.along, from.across),
            self.axis.xy(self.bend, from.across),
            self.axis.xy(self.bend, to.across),
            self.axis.xy(to.along, to.across),
        ]
    }

    pub(super) fn captions(&self) -> impl Iterator<Item = (usize, usize, &str)> {
        self.text.iter().filter_map(|(x, y, text, role)| {
            matches!(role, TextRole::Caption).then_some((*x, *y, text.as_str()))
        })
    }

    pub(super) fn paint(
        &self,
        canvas: &mut Canvas,
        line: Paint,
        caption: Paint,
    ) -> Result<(), String> {
        for &(x, y, ch) in &self.cells {
            canvas.put(x, y, ch)?;
            canvas.paint(x, y, 1, line);
        }
        for (x, y, ch) in self.ports {
            canvas.put(x, y, ch)?;
        }
        for (x, y, text, role) in &self.text {
            canvas.text(*x, *y, text)?;
            canvas.paint(
                *x,
                *y,
                columns(text),
                match role {
                    TextRole::Marker => line,
                    TextRole::Caption => caption,
                },
            );
        }
        Ok(())
    }
}

pub(super) fn marker(mark: Mark, axis: Axis, start: bool, ascii: bool) -> &'static str {
    match (mark, axis, start, ascii) {
        (Mark::None, ..) => "",
        (Mark::Diamond, _, _, true) => "*",
        (Mark::Diamond, ..) => "◆",
        (Mark::HollowDiamond, _, _, true) => "o",
        (Mark::HollowDiamond, ..) => "◇",
        (Mark::Circle, _, _, true) => "()",
        (Mark::Circle, ..) => "○",
        (Mark::Triangle, Axis::Vertical, true, true) => "^|",
        (Mark::Triangle, Axis::Vertical, false, true) => "v|",
        (Mark::Triangle, Axis::Vertical, true, false) => "△",
        (Mark::Triangle, Axis::Vertical, false, false) => "▽",
        (Mark::Triangle, Axis::Horizontal, true, true) => "<|",
        (Mark::Triangle, Axis::Horizontal, false, true) => "|>",
        (Mark::Triangle, Axis::Horizontal, true, false) => "◁",
        (Mark::Triangle, Axis::Horizontal, false, false) => "▷",
        (Mark::Arrow, Axis::Vertical, true, true) => "^",
        (Mark::Arrow, Axis::Vertical, false, true) => "v",
        (Mark::Arrow, Axis::Vertical, true, false) => "↑",
        (Mark::Arrow, Axis::Vertical, false, false) => "↓",
        (Mark::Arrow, Axis::Horizontal, true, true) => "<",
        (Mark::Arrow, Axis::Horizontal, false, true) => ">",
        (Mark::Arrow, Axis::Horizontal, true, false) => "←",
        (Mark::Arrow, Axis::Horizontal, false, false) => "→",
    }
}
