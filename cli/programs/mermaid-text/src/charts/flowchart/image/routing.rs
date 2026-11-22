use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::fmt::Write;

use crate::charts::shared::Direction;
use crate::color::Rgb;
use crate::graphics::Scene;

use super::super::{Edge, EdgeStyle, Link, Tip};
use super::{Bounds, Hierarchy, Point, STEP, Text, Vertex, cells, shapes};

const MAX_GRID_BYTES: usize = 32 * 1024 * 1024;
const MAX_SEARCH_QUEUE: usize = 256 * 1024;
pub(super) const MAX_ROUTING_WORK: usize = 16 * 1024 * 1024;
const WORK_LIMIT: &str = "flowchart connector routing exceeds its work budget";
const NO_ROUTE: &str = "could not route a flowchart connector";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Right,
    Down,
    Left,
    Up,
}

impl Side {
    const ALL: [Self; 4] = [Self::Right, Self::Down, Self::Left, Self::Up];

    fn from_direction(direction: Direction) -> Self {
        match direction {
            Direction::Right => Self::Right,
            Direction::Down => Self::Down,
            Direction::Left => Self::Left,
            Direction::Up => Self::Up,
        }
    }

    fn opposite(self) -> Self {
        Self::ALL[(self as usize + 2) % 4]
    }

    fn delta(self) -> Point {
        match self {
            Self::Right => Point(1, 0),
            Self::Down => Point(0, 1),
            Self::Left => Point(-1, 0),
            Self::Up => Point(0, -1),
        }
    }

    pub(super) fn is_horizontal(self) -> bool {
        matches!(self, Self::Right | Self::Left)
    }

    fn mask(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Clone, Copy)]
struct Port {
    at: Point,
    face: Side,
    heading: Side,
    cost: u32,
}

#[derive(Clone, Copy)]
struct Cell {
    owner: usize,
    blocked: bool,
    // Low bits hold completed routes; high bits hold the route whose label is being placed.
    wires: u8,
    ports: u8,
}

#[derive(Clone, Copy, Default)]
struct Visit {
    search: u32,
    cost: u32,
    previous: u32,
}

struct Route {
    points: Vec<Point>,
    start: Port,
    end: Port,
    label: Option<(Point, Bounds)>,
}

pub(super) enum ConnectError {
    Label,
    Routing(&'static str),
}

impl From<&'static str> for ConnectError {
    fn from(value: &'static str) -> Self {
        Self::Routing(value)
    }
}

impl From<ConnectError> for String {
    fn from(value: ConnectError) -> Self {
        match value {
            ConnectError::Label => "could not place a flowchart connector label",
            ConnectError::Routing(message) => message,
        }
        .into()
    }
}

pub(super) struct Router {
    size: Point,
    grid: Vec<Cell>,
    visits: Vec<Visit>,
    search: u32,
    pub(super) work: usize,
}

impl Router {
    pub(super) fn new(
        size: Point,
        hierarchy: &Hierarchy,
        vertices: &[Vertex<'_>],
        work: usize,
    ) -> Result<Self, String> {
        let cell_bytes = size_of::<Cell>() + Side::ALL.len() * size_of::<Visit>();
        let area = (size.0 as usize)
            .checked_mul(size.1 as usize)
            .filter(|&area| area <= MAX_GRID_BYTES / cell_bytes)
            .ok_or_else(|| {
                format!(
                    "flowchart routing grid {} × {} exceeds its {}-cell limit",
                    size.0,
                    size.1,
                    MAX_GRID_BYTES / cell_bytes
                )
            })?;
        let mut router = Self {
            size,
            grid: vec![
                Cell {
                    owner: hierarchy.root,
                    blocked: false,
                    wires: 0,
                    ports: 0,
                };
                area
            ],
            visits: vec![Visit::default(); area * Side::ALL.len()],
            search: 0,
            work,
        };
        for &group in &hierarchy.scopes[1..] {
            router.fill(vertices[group].bounds, |cell| cell.owner = group)?;
            if !vertices[group].text.lines.is_empty() {
                router.fill(vertices[group].caption(), |cell| cell.blocked = true)?;
            }
        }
        for vertex in vertices.iter().filter(|vertex| !vertex.group) {
            router.fill(vertex.bounds, |cell| cell.blocked = true)?;
        }
        for vertex in vertices {
            for (_, bounds) in vertex.notes() {
                router.fill(bounds, |cell| cell.blocked = true)?;
            }
        }
        let width = size.0 as usize;
        for (y, row) in router.grid.chunks_exact_mut(width).enumerate() {
            row[0].blocked = true;
            row[width - 1].blocked = true;
            if y == 0 || y + 1 == size.1 as usize {
                for cell in row {
                    cell.blocked = true;
                }
            }
        }
        Ok(router)
    }

    fn charge(&mut self, units: usize) -> Result<(), &'static str> {
        self.work = self.work.checked_sub(units).ok_or(WORK_LIMIT)?;
        Ok(())
    }

    fn fill(
        &mut self,
        bounds: Bounds,
        mut update: impl FnMut(&mut Cell),
    ) -> Result<(), &'static str> {
        let end = bounds.end();
        for y in bounds.origin.1..=end.1 {
            if let (Some(start), Some(end)) = (
                self.index(Point(bounds.origin.0, y)),
                self.index(Point(end.0, y)),
            ) {
                self.charge(end - start + 1)?;
                for cell in &mut self.grid[start..=end] {
                    update(cell);
                }
            }
        }
        Ok(())
    }

    fn index(&self, Point(x, y): Point) -> Option<usize> {
        ((0..self.size.0).contains(&x) && (0..self.size.1).contains(&y))
            .then(|| y as usize * self.size.0 as usize + x as usize)
    }

    fn point(&self, index: usize) -> Point {
        let index = index as i32;
        Point(index % self.size.0, index / self.size.0)
    }

    pub(super) fn owner(&self, point: Point) -> usize {
        self.grid[self.index(point).unwrap()].owner
    }

    fn ports(
        &self,
        vertex: &Vertex<'_>,
        preferred: Side,
        inward: bool,
        only: Option<Side>,
        allowed: &BTreeSet<usize>,
    ) -> Vec<Port> {
        let mut ports = Vec::new();
        let center = vertex.bounds.center();
        let origin = vertex.bounds.origin;
        let end = vertex.bounds.end();
        for face in Side::ALL
            .into_iter()
            .filter(|side| only.is_none_or(|only| *side == only))
        {
            let (span, middle, primary) = match face {
                Side::Right => (vertex.bounds.size.1, center.1, end.0),
                Side::Down => (vertex.bounds.size.0, center.0, end.1),
                Side::Left => (vertex.bounds.size.1, center.1, origin.0),
                Side::Up => (vertex.bounds.size.0, center.0, origin.1),
            };
            let extent = ((span - 2) / 2).min(5);
            let heading = if inward { face.opposite() } else { face };
            let side_cost = if face == preferred {
                0
            } else if face == preferred.opposite() {
                30
            } else {
                17
            };
            // The extra off-center choices let a connection enter a named group past its caption.
            let mut offsets: Vec<_> = (-extent..=extent).collect();
            if vertex.group && span > 14 {
                offsets.extend([2 - span / 2, span / 2 - 2]);
            }
            for offset in offsets {
                let lane = middle + offset;
                let on_face = if face.is_horizontal() {
                    Point(primary, lane)
                } else {
                    Point(lane, primary)
                };
                let at = on_face.offset(heading.delta());
                let Some(index) = self.index(at) else {
                    continue;
                };
                let cell = self.grid[index];
                if cell.blocked || !allowed.contains(&cell.owner) {
                    continue;
                }
                let used = if cell.ports & heading.mask() != 0 {
                    30
                } else {
                    0
                };
                ports.push(Port {
                    at,
                    face,
                    heading,
                    cost: side_cost + offset.unsigned_abs() * 3 + used,
                });
            }
        }
        ports
    }

    pub(super) fn connect(
        &mut self,
        edge: Edge<'_>,
        hierarchy: &Hierarchy,
        vertices: &[Vertex<'_>],
        label: &Text<'_>,
        lane: Option<i32>,
    ) -> Result<Trace, ConnectError> {
        let [from, to] = [edge.from, edge.to];
        let ancestors = |mut vertex: usize| {
            std::iter::from_fn(move || {
                if vertex == hierarchy.root {
                    return None;
                }
                vertex = hierarchy.parents[vertex];
                Some(vertex)
            })
        };
        let outgoing_inward = ancestors(to).any(|scope| scope == from);
        let incoming_inward = ancestors(from).any(|scope| scope == to);
        let common = if outgoing_inward {
            from
        } else if incoming_inward {
            to
        } else {
            hierarchy.project(from, to).0
        };
        let within = |endpoint| {
            ancestors(endpoint).take_while(move |&scope| endpoint != common && scope != common)
        };
        let allowed = within(from)
            .chain(within(to))
            .chain(std::iter::once(common))
            .collect();
        let preferred =
            |index| Side::from_direction(hierarchy.directions[hierarchy.parents[index]]);
        let loop_edge = from == to;
        let endpoints = [&vertices[from], &vertices[to]];
        let direct = if let Some(lane) = lane {
            self.parallel_route(endpoints, preferred(from), lane, label, &allowed)?
        } else {
            None
        };
        let Route {
            points,
            start,
            end,
            label: placed,
        } = if let Some(direct) = direct {
            direct
        } else {
            let starts = self.ports(
                endpoints[0],
                preferred(from),
                outgoing_inward,
                loop_edge.then_some(preferred(from)),
                &allowed,
            );
            let ends = self.ports(
                endpoints[1],
                preferred(to).opposite(),
                incoming_inward,
                loop_edge.then_some(if preferred(to).is_horizontal() {
                    Side::Down
                } else {
                    Side::Right
                }),
                &allowed,
            );
            self.route(&starts, &ends, &allowed)?
        };
        for port in [start, end] {
            let index = self.index(port.at).unwrap();
            self.grid[index].ports |= port.heading.mask();
        }
        self.mark_current_wire(&points);
        let label = if label.lines.is_empty() {
            None
        } else {
            let (at, bounds) = match placed {
                Some(label) => label,
                None => self
                    .place_label(&points, label, &allowed)?
                    .ok_or(ConnectError::Label)?,
            };
            self.fill(bounds, |cell| cell.blocked = true)?;
            Some(at)
        };
        for &point in &points {
            let index = self.index(point).unwrap();
            let wires = &mut self.grid[index].wires;
            *wires = (*wires | (*wires >> 4)) & 0xf;
        }

        let mut coordinates = vec![shapes::attachment(&vertices[from], start.face, start.at)];
        coordinates.extend(points.iter().map(|point| point.pixels()));
        coordinates.push(shapes::attachment(&vertices[to], end.face, end.at));
        Ok(Trace {
            coordinates: straighten(coordinates),
            label,
        })
    }

    fn parallel_route(
        &mut self,
        [from, to]: [&Vertex<'_>; 2],
        direction: Side,
        lane: i32,
        text: &Text<'_>,
        allowed: &BTreeSet<usize>,
    ) -> Result<Option<Route>, &'static str> {
        let horizontal = direction.is_horizontal();
        let axes = |Point(x, y)| if horizontal { (x, y) } else { (y, x) };
        let point = |p, c| if horizontal { Point(p, c) } else { Point(c, p) };
        let (a, a_cross) = axes(from.bounds.origin);
        let (b, b_cross) = axes(to.bounds.origin);
        let (a_end, a_cross_end) = axes(from.bounds.end());
        let (b_end, b_cross_end) = axes(to.bounds.end());
        let (first, last, forward) = if a_end + 1 < b {
            (a_end + 1, b - 1, true)
        } else if b_end + 1 < a {
            (b_end + 1, a - 1, false)
        } else {
            return Ok(None);
        };
        let min_cross = a_cross.max(b_cross);
        let max_cross = a_cross_end.min(b_cross_end);
        let cross = (min_cross + max_cross) / 2 + lane;
        if !(min_cross + 1..max_cross).contains(&cross) {
            return Ok(None);
        }
        let middle = point((first + last) / 2, cross);
        let placed = if text.lines.is_empty() {
            None
        } else {
            let at = Point(middle.0, middle.1 - cells(text.height) / 2);
            let bounds = label_bounds(text, at);
            if self.label_room(bounds, allowed, None)? != Some(0) {
                return Ok(None);
            }
            Some((at, bounds))
        };
        let mut points = Vec::new();
        for p in first..=last {
            self.charge(1)?;
            let at = point(p, cross);
            let Some(cell) = self.index(at).map(|index| self.grid[index]) else {
                return Ok(None);
            };
            if cell.blocked || cell.wires != 0 || cell.ports != 0 || !allowed.contains(&cell.owner)
            {
                return Ok(None);
            }
            points.push(at);
        }
        if !forward {
            points.reverse();
        }
        let face = match (horizontal, forward) {
            (true, true) => Side::Right,
            (true, false) => Side::Left,
            (false, true) => Side::Down,
            (false, false) => Side::Up,
        };
        let port = |at, face| Port {
            at,
            face,
            heading: face,
            cost: 0,
        };
        Ok(Some(Route {
            start: port(points[0], face),
            end: port(*points.last().unwrap(), face.opposite()),
            points,
            label: placed,
        }))
    }

    fn route(
        &mut self,
        starts: &[Port],
        ends: &[Port],
        allowed: &BTreeSet<usize>,
    ) -> Result<Route, &'static str> {
        let first = ends.first().ok_or(NO_ROUTE)?;
        let (mut min, mut max) = (first.at, first.at);
        let mut targets = BTreeMap::new();
        for &port in ends {
            min = Point(min.0.min(port.at.0), min.1.min(port.at.1));
            max = Point(max.0.max(port.at.0), max.1.max(port.at.1));
            targets.insert(self.index(port.at).unwrap(), port);
        }
        let remaining = |Point(x, y): Point| {
            10 * (x.abs_diff(x.clamp(min.0, max.0)) + y.abs_diff(y.clamp(min.1, max.1)))
        };
        self.search += 1;
        let mut queue = BinaryHeap::new();
        let mut best = None;
        for &port in starts {
            let state = self.index(port.at).unwrap() * 4 + port.heading as usize;
            self.visits[state] = Visit {
                search: self.search,
                cost: port.cost,
                previous: u32::MAX,
            };
            queue.push(Reverse((
                port.cost + remaining(port.at),
                Reverse(port.cost),
                state,
            )));
        }
        while let Some(Reverse((estimate, Reverse(cost), state))) = queue.pop() {
            self.charge(1)?;
            if best.is_some_and(|(cost, _, _)| estimate >= cost) {
                break;
            }
            if self.visits[state].cost != cost {
                continue;
            }
            let index = state / 4;
            let heading = Side::ALL[state % 4];
            if let Some(&port) = targets.get(&index)
                && heading != port.heading
            {
                let cost = cost
                    + port.cost
                    + if heading == port.heading.opposite() {
                        0
                    } else {
                        9
                    };
                if best.is_none_or(|(existing, _, _)| cost < existing) {
                    best = Some((cost, state, port));
                }
            }
            let point = self.point(index);
            for side in Side::ALL {
                if side == heading.opposite() {
                    continue;
                }
                let next = point.offset(side.delta());
                let Some(next_index) = self.index(next) else {
                    continue;
                };
                let cell = self.grid[next_index];
                if cell.blocked || !allowed.contains(&cell.owner) {
                    continue;
                }
                let existing = self.grid[index].wires;
                let wire_cost = if existing & side.mask() != 0 {
                    43
                } else if cell.wires != 0 {
                    12
                } else {
                    0
                };
                let turn = if side == heading { 0 } else { 9 };
                let cost = cost + 10 + turn + wire_cost;
                let next_state = next_index * 4 + side as usize;
                let visit = &mut self.visits[next_state];
                if visit.search != self.search || cost < visit.cost {
                    *visit = Visit {
                        search: self.search,
                        cost,
                        previous: state as u32,
                    };
                    if queue.len() == MAX_SEARCH_QUEUE {
                        return Err("flowchart connector search queue exceeds its capacity");
                    }
                    queue.push(Reverse((cost + remaining(next), Reverse(cost), next_state)));
                }
            }
        }
        let (_, mut state, end) = best.ok_or(NO_ROUTE)?;
        let mut points = Vec::new();
        loop {
            points.push(self.point(state / 4));
            let previous = self.visits[state].previous;
            if previous == u32::MAX {
                break;
            }
            state = previous as usize;
        }
        points.reverse();
        let start = starts
            .iter()
            .find(|port| port.at == points[0] && port.heading as usize == state % 4)
            .ok_or(NO_ROUTE)?;
        Ok(Route {
            points,
            start: *start,
            end,
            label: None,
        })
    }

    fn mark_current_wire(&mut self, points: &[Point]) {
        for pair in points.windows(2) {
            let from = self.index(pair[0]).unwrap();
            let to = self.index(pair[1]).unwrap();
            let delta = Point(pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
            let side = Side::ALL
                .into_iter()
                .find(|side| side.delta() == delta)
                .unwrap();
            self.grid[from].wires |= side.mask() << 4;
            self.grid[to].wires |= side.opposite().mask() << 4;
        }
    }

    fn place_label(
        &mut self,
        points: &[Point],
        text: &Text<'_>,
        allowed: &BTreeSet<usize>,
    ) -> Result<Option<(Point, Bounds)>, &'static str> {
        let half = label_half_width(text);
        let height = cells(text.height);
        let midpoint = points.len() / 2;
        let mut best = None;
        for (index, &Point(x, y)) in points.iter().enumerate() {
            let horizontal = points
                .get(index + 1)
                .or_else(|| index.checked_sub(1).and_then(|i| points.get(i)))
                .is_some_and(|point| point.1 == y);
            for step in 0..=6 {
                let candidates = if horizontal {
                    [
                        Point(x, y - height / 2),
                        Point(x, y - height - step),
                        Point(x, y + step + 1),
                    ]
                } else {
                    [
                        Point(x + half + step, y - height / 2),
                        Point(x - half - step, y - height / 2),
                        Point(x, y - height / 2),
                    ]
                };
                for (side, at) in candidates.into_iter().enumerate() {
                    let cost = index.abs_diff(midpoint).min(30) + step as usize * 4 + side;
                    if best.is_some_and(|(best, _, _)| cost >= best) {
                        continue;
                    }
                    let bounds = label_bounds(text, at);
                    let inline = (horizontal && side == 0).then_some(y);
                    if let Some(crossings) = self.label_room(bounds, allowed, inline)? {
                        let cost = cost + crossings.min(12) * 12;
                        if best.is_none_or(|(best, _, _)| cost < best) {
                            best = Some((cost, at, bounds));
                        }
                    }
                }
            }
        }
        Ok(best.map(|(_, at, bounds)| (at, bounds)))
    }

    fn label_room(
        &mut self,
        bounds: Bounds,
        allowed: &BTreeSet<usize>,
        inline: Option<i32>,
    ) -> Result<Option<usize>, &'static str> {
        self.charge(1)?;
        let end = bounds.end();
        let Some(origin) = self
            .index(bounds.origin)
            .filter(|_| self.index(end).is_some())
        else {
            return Ok(None);
        };
        let owner = self.grid[origin].owner;
        if !allowed.contains(&owner) {
            return Ok(None);
        }
        let mut crossings = 0;
        for y in bounds.origin.1..=end.1 {
            let start = self.index(Point(bounds.origin.0, y)).unwrap();
            let end = self.index(Point(end.0, y)).unwrap();
            self.charge(end - start + 1)?;
            for cell in &self.grid[start..=end] {
                if cell.blocked || cell.owner != owner {
                    return Ok(None);
                }
                let horizontal = Side::Left.mask() | Side::Right.mask();
                let inline = inline == Some(y) && cell.wires >> 4 == horizontal;
                crossings +=
                    usize::from(cell.wires & 0xf != 0 || (cell.wires >> 4 != 0 && !inline));
            }
        }
        Ok(Some(crossings))
    }
}

fn label_half_width(text: &Text<'_>) -> i32 {
    cells(text.width / 2.0 + 5.0)
}

fn label_bounds(text: &Text<'_>, at: Point) -> Bounds {
    let half = label_half_width(text);
    Bounds {
        origin: Point(at.0 - half, at.1),
        size: Point(2 * half, cells(text.height)),
    }
}

type Pixels = (f32, f32);

pub(super) struct Trace {
    coordinates: Vec<Pixels>,
    pub(super) label: Option<Point>,
}

fn straighten(points: Vec<Pixels>) -> Vec<Pixels> {
    let mut output: Vec<Pixels> = Vec::new();
    for point in points {
        if let Some(&last) = output.last() {
            if point == last {
                continue;
            }
            if let Some(&prior) = output.len().checked_sub(2).and_then(|i| output.get(i))
                && ((prior.0 == last.0 && last.0 == point.0)
                    || (prior.1 == last.1 && last.1 == point.1))
            {
                output.pop();
            }
        }
        output.push(point);
    }
    output
}

impl Trace {
    pub(super) fn annotation(start: Pixels, end: Pixels, scene: &mut Scene, stroke: Rgb) {
        Self {
            coordinates: vec![start, end],
            label: None,
        }
        .draw_line(
            scene,
            Link {
                style: EdgeStyle::Dotted,
                start: Tip::Open,
                end: Tip::Open,
            },
            stroke,
        );
    }

    pub(super) fn draw_line(&self, scene: &mut Scene, link: Link, stroke: Rgb) {
        let mut path = String::new();
        if link.style == EdgeStyle::Dotted {
            let mut offset = 0.0_f32;
            let period = 8.5;
            for segment in self.coordinates.windows(2) {
                let [a, b] = [segment[0], segment[1]];
                let length = (b.0 - a.0).abs() + (b.1 - a.1).abs();
                let mut start = -offset;
                while start < length {
                    let stop = (start + 3.5).min(length);
                    if stop > 0.0 {
                        let (x, y) = along(a, b, start.max(0.0) / length);
                        let (end_x, end_y) = along(a, b, stop / length);
                        write!(path, "M{x} {y}L{end_x} {end_y} ").unwrap();
                    }
                    start += period;
                }
                offset = (offset + length) % period;
            }
        } else if let Some(&(x, y)) = self.coordinates.first() {
            write!(path, "M{x} {y}").unwrap();
            for bend in self.coordinates.windows(3) {
                let [before, at, after] = [bend[0], bend[1], bend[2]];
                let near = |other: Pixels| {
                    let length = (other.0 - at.0).abs() + (other.1 - at.1).abs();
                    along(at, other, (STEP * 0.5 / length).min(0.5))
                };
                let (a, b) = (near(before), near(after));
                write!(path, " L{} {}Q{} {} {} {}", a.0, a.1, at.0, at.1, b.0, b.1).unwrap();
            }
            if let Some(&(x, y)) = self.coordinates.last() {
                write!(path, " L{x} {y}").unwrap();
            }
        }
        let width = if link.style == EdgeStyle::Thick {
            3.0
        } else {
            1.6
        };
        scene.path(&path, None, Some(stroke), width);
    }

    pub(super) fn draw_tips(&self, scene: &mut Scene, link: Link, stroke: Rgb) {
        let points = &self.coordinates;
        if points.len() >= 2 && !stroke.is_transparent() {
            tip(scene, points[0], points[1], link.start, stroke);
            tip(
                scene,
                points[points.len() - 1],
                points[points.len() - 2],
                link.end,
                stroke,
            );
        }
    }
}

fn along(start: Pixels, end: Pixels, fraction: f32) -> Pixels {
    (
        start.0 + (end.0 - start.0) * fraction,
        start.1 + (end.1 - start.1) * fraction,
    )
}

fn tip(scene: &mut Scene, (x, y): Pixels, before: Pixels, tip: Tip, ink: Rgb) {
    let len = (before.0 - x).abs() + (before.1 - y).abs();
    let (dx, dy) = ((before.0 - x) / len, (before.1 - y) / len);
    let point = |advance: f32, spread: f32| {
        (
            x + dx * advance - dy * spread,
            y + dy * advance + dx * spread,
        )
    };
    match tip {
        Tip::Open => {}
        Tip::Arrow => {
            let (a, b) = point(10.0, -4.0);
            let (c, d) = point(10.0, 4.0);
            scene.path(&format!("M{x} {y}L{a} {b}L{c} {d}Z"), Some(ink), None, 1.0);
        }
        Tip::Circle => {
            let (a, b) = point(4.0, 0.0);
            scene.rect(
                a - 4.0,
                b - 4.0,
                8.0,
                8.0,
                4.0,
                Some(scene.paper()),
                Some(ink),
            );
        }
        Tip::Cross => {
            let (a, b) = point(2.0, -3.5);
            let (c, d) = point(9.0, 3.5);
            let (e, f) = point(2.0, 3.5);
            let (g, h) = point(9.0, -3.5);
            scene.path(
                &format!("M{a} {b}L{c} {d} M{e} {f}L{g} {h}"),
                None,
                Some(ink),
                1.7,
            );
        }
    }
}
