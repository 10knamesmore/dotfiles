use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use unicode_width::UnicodeWidthStr;

use crate::charts::shared::Direction;
use crate::color::{Layer as ColorLayer, Paint, Style};
use crate::{BorderCorners, Options};

use super::{EdgeStyle, Graph, Label, Link, Node, Outline, Tip, Topology};

pub(super) const MAX_CELLS: usize = 2 * 1024 * 1024;
pub(super) const CANVAS_LIMIT: &str = "flowchart layout exceeds 2,097,152 canvas cells";

#[derive(Default)]
pub(super) struct Drawing {
    pub(super) plain: String,
    pub(super) branch_slots: usize,
    // Runs use terminal columns so panels can translate paint with their text origin.
    paints: Vec<PaintRun>,
}

struct PaintRun {
    x: usize,
    y: usize,
    width: usize,
    paint: Paint,
}

impl Drawing {
    pub(super) fn render(self) -> String {
        if self.paints.is_empty() {
            return self.plain;
        }
        let mut layer = ColorLayer::default();
        for run in self.paints {
            layer.paint(run.x, run.y, run.width, run.paint);
        }
        layer.apply(&self.plain)
    }
}

pub(super) fn render(
    graph: &Graph<'_>,
    topology: &Topology,
    direction: Direction,
    options: &Options,
    max_branch_gap: usize,
) -> Result<String, String> {
    let draw = |gap| {
        let mut remaining = MAX_CELLS;
        render_components(
            graph,
            topology,
            topology.components.iter().map(Vec::as_slice),
            direction,
            options,
            &mut remaining,
            gap,
        )
    };
    let drawing = match draw(max_branch_gap) {
        Err(error) if error == CANVAS_LIMIT && max_branch_gap != 0 => draw(0)?,
        result => result?,
    };
    Ok(drawing.render())
}

pub(super) fn render_scoped(
    graph: &Graph<'_>,
    topology: &Topology,
    direction: Direction,
    options: &Options,
    remaining: &mut usize,
    max_branch_gap: usize,
) -> Result<Drawing, String> {
    let members: Vec<_> = (0..graph.nodes.len()).collect();
    render_components(
        graph,
        topology,
        std::iter::once(members.as_slice()),
        direction,
        options,
        remaining,
        max_branch_gap,
    )
}

fn render_components<'a>(
    graph: &Graph<'_>,
    topology: &Topology,
    components: impl Iterator<Item = &'a [usize]>,
    direction: Direction,
    options: &Options,
    remaining: &mut usize,
    max_branch_gap: usize,
) -> Result<Drawing, String> {
    let mut output = Drawing::default();
    let mut top = 0;
    let horizontal = direction.is_horizontal();
    let columns = |(primary, cross)| if horizontal { primary } else { cross };
    let node_theme = options.theme_style();
    let edge_theme = Style {
        stroke: options.theme_line(),
        ..Style::default()
    };
    for component in components {
        let mut layout = Layout::new(graph, topology, component, edge_theme)?;
        let mut size = layout.measure(horizontal, 2, 0);
        if columns(size) > options.width || !fits_canvas(size, *remaining) {
            let compact = layout.measure(horizontal, 1, 0);
            size = if fits_canvas(size, *remaining)
                && (columns(size) <= columns(compact) || !fits_canvas(compact, *remaining))
            {
                layout.measure(horizontal, 2, 0)
            } else {
                compact
            };
        } else {
            let slots = layout.branch_slots(horizontal);
            output.branch_slots += slots;
            let gap = (options.width - columns(size))
                .checked_div(slots)
                .unwrap_or(0)
                .min(max_branch_gap);
            if gap > 0 {
                let expanded = layout.measure(horizontal, 2, gap);
                size = if columns(expanded) <= options.width && fits_canvas(expanded, *remaining) {
                    expanded
                } else {
                    layout.measure(horizontal, 2, 0)
                };
            }
        }
        let axes = Axes {
            direction,
            primary: size.0,
        };
        if !fits_canvas(size, *remaining) {
            return Err(CANVAS_LIMIT.into());
        }
        let rows = if horizontal { size.1 } else { size.0 };
        let columns = columns(size);
        *remaining -= columns * rows;
        let mut canvas = Canvas::new(columns, rows, options, node_theme);
        layout.draw(axes, &mut canvas)?;
        if !output.plain.is_empty() {
            output.plain.push_str("\n\n");
        }
        canvas.write(&mut output, top);
        top += rows + 1;
    }
    Ok(output)
}

fn fits_canvas((primary, cross): (usize, usize), budget: usize) -> bool {
    primary
        .checked_mul(cross)
        .is_some_and(|cells| cells <= budget)
}

impl Direction {
    fn is_horizontal(self) -> bool {
        matches!(self, Self::Right | Self::Left)
    }

    fn arrow(self, ascii: bool) -> char {
        match (self, ascii) {
            (Self::Right, false) => '▶',
            (Self::Left, false) => '◀',
            (Self::Down, false) => '▼',
            (Self::Up, false) => '▲',
            (Self::Right, true) => '>',
            (Self::Left, true) => '<',
            (Self::Down, true) => 'v',
            (Self::Up, true) => '^',
        }
    }

    fn reversed(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Down => Self::Up,
            Self::Up => Self::Down,
        }
    }
}

impl Tip {
    fn glyph(self, direction: Direction, ascii: bool) -> Option<char> {
        match (self, ascii) {
            (Self::Open, _) => None,
            (Self::Arrow, _) => Some(direction.arrow(ascii)),
            (Self::Circle, false) => Some('○'),
            (Self::Circle, true) => Some('o'),
            (Self::Cross, false) => Some('×'),
            (Self::Cross, true) => Some('x'),
        }
    }
}

#[derive(Clone, Copy)]
struct Axes {
    direction: Direction,
    primary: usize,
}

impl Axes {
    fn point(self, primary: usize, lane: usize) -> (usize, usize) {
        match self.direction {
            Direction::Right => (primary, lane),
            Direction::Left => (self.primary - 1 - primary, lane),
            Direction::Down => (lane, primary),
            Direction::Up => (lane, self.primary - 1 - primary),
        }
    }
}

#[derive(Default)]
struct Layer {
    vertices: Vec<usize>,
    start: usize,
    span: usize,
}

fn node_label_lines(node: Node<'_>) -> impl Iterator<Item = Cow<'_, str>> {
    node.label
        .lines()
        .chain(node.port_name.into_iter().flat_map(Label::lines))
        .chain(node.shape.note.map(Cow::Borrowed))
}

fn node_label_rows(node: Node<'_>) -> usize {
    node.label.rows
        + node.port_name.map_or(0, |name| name.rows)
        + usize::from(node.shape.note.is_some())
}

struct Vertex<'a> {
    node: Node<'a>,
    primary_span: usize,
    slant: usize,
    incoming: Vec<usize>,
    outgoing: Vec<usize>,
    lane: usize,
    box_lanes: usize,
    used_lanes: usize,
}

impl Vertex<'_> {
    fn size(&mut self, segments: &mut [Segment<'_>], horizontal: bool, spacing: usize) {
        let port_step = |segment: &Segment<'_>| {
            if horizontal {
                segment.label.map_or(spacing, |label| {
                    if label.rows == 1 {
                        spacing
                    } else {
                        label.rows + 1
                    }
                })
            } else {
                segment.label.map_or(spacing, |label| 3 + label.columns)
            }
        };
        let departure_span = self.outgoing.split_last().map_or(0, |(_, preceding)| {
            1 + preceding
                .iter()
                .map(|&index| port_step(&segments[index]))
                .sum::<usize>()
        });
        let arrival_span = self
            .incoming
            .split_last()
            .map_or(0, |(_, preceding)| 1 + preceding.len() * spacing);
        let port_lanes = 2 + arrival_span.max(departure_span);
        let (row_width, mut height) = self.node.dimensions();
        if matches!(self.node.shape.outline, Outline::Decision) && self.node.body.is_none() {
            height = node_label_rows(self.node) + 4;
            if horizontal {
                height = height.max(port_lanes);
            }
            self.slant = height - 1;
        }
        let width = row_width + self.slant;
        self.primary_span = if horizontal { width } else { height };
        self.box_lanes = port_lanes.max(if horizontal { height } else { width });
        self.used_lanes = self.box_lanes;
        let center = self.box_lanes / 2;
        for (offset, &index) in self.incoming.iter().enumerate() {
            segments[index].arrival = center - arrival_span / 2 + offset * spacing;
        }
        let mut departure = center - departure_span / 2;
        for &index in &self.outgoing {
            let segment = &mut segments[index];
            segment.departure = departure;
            departure += port_step(segment);
        }
        self.used_lanes = self.used_lanes.max(departure);
    }

    fn primary_bounds(&self, layer: &Layer) -> (usize, usize) {
        let span = self.primary_span;
        let start = layer.start + (layer.span - span) / 2;
        (start, start + span - 1)
    }

    fn port_inset(&self, lane: usize, from: Direction) -> usize {
        match from {
            Direction::Down | Direction::Right => self.slant.saturating_sub(lane),
            Direction::Up | Direction::Left => lane.saturating_sub(self.box_lanes - 1 - self.slant),
        }
    }

    fn arrival(&self, layer: &Layer, lane: usize, direction: Direction) -> usize {
        self.primary_bounds(layer).0 + self.port_inset(lane, direction)
    }

    fn departure(&self, layer: &Layer, lane: usize, direction: Direction) -> usize {
        self.primary_bounds(layer).1 - self.port_inset(lane, direction.reversed())
    }
}

struct Segment<'a> {
    from: usize,
    to: usize,
    label: Option<Label<'a>>,
    link: Link,
    style: Style,
    side_lane: Option<usize>,
    departure: usize,
    arrival: usize,
}

struct ExteriorEdge {
    arrival: usize,
    departure: usize,
    edge: usize,
}

impl ExteriorEdge {
    fn bounds(&self) -> (usize, usize) {
        (
            self.arrival.min(self.departure),
            self.arrival.max(self.departure),
        )
    }
}

fn assign_side_lanes(arcs: &mut [ExteriorEdge], segments: &mut [Segment<'_>]) -> usize {
    arcs.sort_by_key(ExteriorEdge::bounds);
    let mut active = BinaryHeap::new();
    let mut free = Vec::new();
    let mut count = 0;
    for arc in arcs {
        let (start, end) = arc.bounds();
        while let Some(&Reverse((end, lane))) = active.peek() {
            if end >= start {
                break;
            }
            active.pop();
            free.push(lane);
        }
        let lane = free.pop().unwrap_or_else(|| {
            count += 1;
            count - 1
        });
        segments[arc.edge].side_lane = Some(lane);
        active.push(Reverse((end, lane)));
    }
    count
}

struct Bend {
    segment: usize,
    lane: usize,
}

#[derive(Default)]
struct Gap {
    start: usize,
    segments: Vec<usize>,
    departures: Vec<usize>,
    arrivals: Vec<usize>,
    label_span: usize,
    bends: Vec<Bend>,
    branches: bool,
    labels_at_targets: bool,
}

impl Gap {
    fn can_label_targets(
        &self,
        segments: &[Segment<'_>],
        vertices: &[Vertex<'_>],
        columns: usize,
    ) -> bool {
        if !self.branches
            || self.label_span == 0
            || self.segments.len() < 2
            || !self.arrivals.is_empty()
            || !self.departures.is_empty()
        {
            return false;
        }
        let mut occupied = Vec::with_capacity(self.segments.len());
        for &index in &self.segments {
            let edge = &segments[index];
            if !matches!(edge.link.start, Tip::Open) {
                return false;
            }
            let lane = vertices[edge.to].lane + edge.arrival;
            let width = edge.label.map_or(1, |label| label.columns.max(1));
            let Some(start) = lane.checked_sub(width / 2) else {
                return false;
            };
            if start + width > columns {
                return false;
            }
            occupied.push((start, start + width));
        }
        occupied.sort_unstable();
        occupied.windows(2).all(|pair| pair[0].1 < pair[1].0)
    }
}

struct Layout<'a> {
    layers: Vec<Layer>,
    vertices: Vec<Vertex<'a>>,
    segments: Vec<Segment<'a>>,
    gaps: Vec<Gap>,
    side_lanes: usize,
    side_start: usize,
}

impl<'a> Layout<'a> {
    fn new(
        graph: &Graph<'a>,
        topology: &Topology,
        component: &[usize],
        edge_theme: Style,
    ) -> Result<Self, &'static str> {
        let last_layer = component
            .iter()
            .map(|&node| topology.ranks[node])
            .max()
            .unwrap_or_default();
        let mut layers: Vec<Layer> = (0..=last_layer).map(|_| Layer::default()).collect();
        let mut gaps: Vec<Gap> = (0..=layers.len()).map(|_| Gap::default()).collect();
        let mut vertices = Vec::with_capacity(component.len());
        let mut segments = Vec::new();
        let mut exterior = Vec::new();
        for &node in component {
            layers[topology.ranks[node]].vertices.push(vertices.len());
            vertices.push(Vertex {
                node: graph.nodes[node],
                primary_span: 0,
                slant: 0,
                incoming: Vec::new(),
                outgoing: Vec::new(),
                lane: 0,
                box_lanes: 0,
                used_lanes: 0,
            });
        }
        for (from, &node) in component.iter().enumerate() {
            let departure = topology.ranks[node] + 1;
            for &index in &topology.outgoing[node] {
                let edge = &graph.edges[index];
                if edge.link.style == EdgeStyle::Invisible {
                    continue;
                }
                let arrival = topology.ranks[edge.to];
                let to = component
                    .binary_search(&edge.to)
                    .map_err(|_| "missing flowchart node")?;
                let index = segments.len();
                if departure == arrival {
                    gaps[departure].segments.push(index);
                } else {
                    gaps[departure].departures.push(index);
                    gaps[arrival].arrivals.push(index);
                    exterior.push(ExteriorEdge {
                        departure,
                        arrival,
                        edge: index,
                    });
                }
                vertices[from].outgoing.push(index);
                vertices[to].incoming.push(index);
                segments.push(Segment {
                    from,
                    to,
                    label: edge.label,
                    link: edge.link,
                    style: edge_theme.overlay(edge.style),
                    side_lane: None,
                    departure: 0,
                    arrival: 0,
                });
            }
        }
        let side_lanes = assign_side_lanes(&mut exterior, &mut segments);
        for layer in &mut layers {
            for &index in &layer.vertices {
                let vertex = &mut vertices[index];
                vertex.incoming.sort_by_key(|&index| {
                    let segment = &segments[index];
                    (segment.side_lane, segment.from)
                });
                vertex.outgoing.sort_by_key(|&index| {
                    let segment = &segments[index];
                    (segment.side_lane, segment.to)
                });
            }
        }
        for gap in &mut gaps {
            gap.branches = gap
                .segments
                .iter()
                .chain(&gap.departures)
                .chain(&gap.arrivals)
                .any(|&index| {
                    let segment = &segments[index];
                    vertices[segment.from].outgoing.len() > 1
                        || vertices[segment.to].incoming.len() > 1
                });
        }
        Ok(Self {
            layers,
            vertices,
            segments,
            gaps,
            side_lanes,
            side_start: 0,
        })
    }

    fn branch_slots(&self, horizontal: bool) -> usize {
        let branching_gaps = self.gaps.iter().filter(|gap| gap.branches).count();
        if branching_gaps == 0 {
            return 0;
        }
        if horizontal {
            // Share spare columns between fork/join gaps and the two outside margins.
            branching_gaps + 1
        } else {
            self.layers
                .iter()
                .map(|layer| layer.vertices.len())
                .max()
                .filter(|&n| n > 1)
                .unwrap_or(0)
        }
    }

    fn measure(&mut self, horizontal: bool, spacing: usize, branch_gap: usize) -> (usize, usize) {
        let mut spans = Vec::with_capacity(self.layers.len());
        for layer in &mut self.layers {
            layer.span = 3;
            let mut next_lane = 0;
            let mut span = 0;
            for &index in &layer.vertices {
                let vertex = &mut self.vertices[index];
                vertex.size(&mut self.segments, horizontal, spacing);
                layer.span = layer.span.max(vertex.primary_span);
                vertex.lane = next_lane;
                span = next_lane + vertex.box_lanes;
                next_lane += vertex.used_lanes + spacing + if horizontal { 0 } else { branch_gap };
            }
            spans.push(span);
        }
        let center = spans.iter().copied().max().unwrap_or_default() / 2;
        let mut cross = 0;
        for (layer, span) in self.layers.iter().zip(spans) {
            for &index in &layer.vertices {
                let vertex = &mut self.vertices[index];
                vertex.lane += center - span / 2;
                cross = cross.max(vertex.lane + vertex.used_lanes);
            }
        }

        let mut primary = 0;
        let spare_lane = cross;
        for (rank, gap) in self.gaps.iter_mut().enumerate() {
            // Turn outer departures first and inner arrivals first to keep nearby ports clear.
            gap.departures.sort_by_key(|&index| {
                let segment = &self.segments[index];
                Reverse(self.vertices[segment.from].lane + segment.departure)
            });
            gap.arrivals.sort_by_key(|&index| {
                let segment = &self.segments[index];
                self.vertices[segment.to].lane + segment.arrival
            });
            gap.start = primary;
            gap.label_span = gap
                .segments
                .iter()
                .chain(&gap.departures)
                .map(|&index| {
                    let segment = &self.segments[index];
                    segment.label.map_or(
                        usize::from(!matches!(segment.link.start, Tip::Open)),
                        |label| {
                            if horizontal {
                                label.columns + 3
                            } else {
                                label.rows
                            }
                        },
                    )
                })
                .max()
                .unwrap_or(0);
            let wires = gap.segments.iter().map(|&index| {
                let segment = &self.segments[index];
                Wire {
                    segment: index,
                    lane: self.vertices[segment.from].lane + segment.departure,
                    target: self.vertices[segment.to].lane + segment.arrival,
                }
            });
            gap.bends = route(wires, spare_lane);
            gap.labels_at_targets = branch_gap > 0
                && !horizontal
                && gap.can_label_targets(&self.segments, &self.vertices, cross);
            if gap.bends.iter().any(|bend| bend.lane == spare_lane) {
                cross = spare_lane + 1;
            }
            if (rank > 0 && rank < self.layers.len())
                || !gap.segments.is_empty()
                || !gap.departures.is_empty()
                || !gap.arrivals.is_empty()
            {
                let tail = if rank == self.layers.len() {
                    0
                } else if gap
                    .segments
                    .iter()
                    .chain(&gap.arrivals)
                    .any(|&index| !matches!(self.segments[index].link.style, EdgeStyle::Solid))
                {
                    2
                } else {
                    spacing
                };
                primary += gap.label_span
                    + gap.departures.len()
                    + gap.bends.len()
                    + gap.arrivals.len()
                    + tail
                    + if horizontal && gap.branches {
                        branch_gap
                    } else {
                        0
                    };
            }
            if let Some(layer) = self.layers.get_mut(rank) {
                layer.start = primary;
                primary += layer.span;
            }
        }
        self.side_start = cross + 1;
        if self.side_lanes > 0 {
            cross = self.side_start + self.side_lanes;
        }
        (primary, cross)
    }

    fn draw(&self, axes: Axes, canvas: &mut Canvas<'a>) -> Result<(), &'static str> {
        let horizontal = axes.direction.is_horizontal();
        for layer in &self.layers {
            for &index in &layer.vertices {
                let vertex = &self.vertices[index];
                let (start, end) = vertex.primary_bounds(layer);
                let first = axes.point(start, vertex.lane);
                let last = axes.point(end, vertex.lane + vertex.box_lanes - 1);
                if vertex.slant > 0 {
                    canvas.slanted_node(first, last, vertex.node)?;
                } else {
                    canvas.node(first, last, vertex.node)?;
                }
            }
        }

        let mut ends = vec![(0, 0, false); self.segments.len()];
        let mut side_tracks = vec![(0, 0); self.segments.len()];
        for (rank, gap) in self.gaps.iter().enumerate() {
            let mut track = if gap.labels_at_targets {
                gap.start
            } else {
                gap.start + gap.label_span
            };
            for &index in &gap.departures {
                let segment = &self.segments[index];
                let vertex = &self.vertices[segment.from];
                let primary =
                    vertex.departure(&self.layers[rank - 1], segment.departure, axes.direction);
                let lane = vertex.lane + segment.departure;
                let corner = axes.point(track, lane);
                let side = self.side_start + segment.side_lane.ok_or("missing exterior lane")?;
                canvas.connect(axes.point(primary, lane), corner, segment, (true, false))?;
                canvas.connect(corner, axes.point(track, side), segment, (false, false))?;
                side_tracks[index].0 = track;
                track += 1;
            }
            for &index in &gap.segments {
                let segment = &self.segments[index];
                let vertex = &self.vertices[segment.from];
                let primary =
                    vertex.departure(&self.layers[rank - 1], segment.departure, axes.direction);
                ends[index] = (primary, vertex.lane + segment.departure, true);
            }
            for bend in &gap.bends {
                let (last_primary, lane, at_port) = ends[bend.segment];
                let segment = &self.segments[bend.segment];
                let corner = axes.point(track, lane);
                canvas.connect(
                    axes.point(last_primary, lane),
                    corner,
                    segment,
                    (at_port, false),
                )?;
                canvas.connect(
                    corner,
                    axes.point(track, bend.lane),
                    segment,
                    (false, false),
                )?;
                ends[bend.segment] = (track, bend.lane, false);
                track += 1;
            }
            for &index in &gap.arrivals {
                let segment = &self.segments[index];
                let vertex = &self.vertices[segment.to];
                let lane = vertex.lane + segment.arrival;
                let side = self.side_start + segment.side_lane.ok_or("missing exterior lane")?;
                canvas.connect(
                    axes.point(track, side),
                    axes.point(track, lane),
                    segment,
                    (false, false),
                )?;
                side_tracks[index].1 = track;
                track += 1;
            }
            let first_arrival = track - gap.arrivals.len();
            for (offset, &index) in gap.arrivals.iter().enumerate() {
                let segment = &self.segments[index];
                let vertex = &self.vertices[segment.to];
                let end = vertex.arrival(&self.layers[rank], segment.arrival, axes.direction);
                let lane = vertex.lane + segment.arrival;
                canvas.connect(
                    axes.point(first_arrival + offset, lane),
                    axes.point(end, lane),
                    segment,
                    (false, true),
                )?;
                if let Some(mark) = segment.link.end.glyph(axes.direction, canvas.ascii) {
                    canvas.glyph(axes.point(end - 1, lane), mark, segment.style.border())?;
                }
            }
            for &index in &gap.segments {
                let segment = &self.segments[index];
                let vertex = &self.vertices[segment.to];
                let end = vertex.arrival(&self.layers[rank], segment.arrival, axes.direction);
                let (primary, lane, at_port) = ends[index];
                canvas.connect(
                    axes.point(primary, lane),
                    axes.point(end, lane),
                    segment,
                    (at_port, true),
                )?;
                if let Some(mark) = segment.link.end.glyph(axes.direction, canvas.ascii) {
                    canvas.glyph(axes.point(end - 1, lane), mark, segment.style.border())?;
                }
            }
            for &index in gap.segments.iter().chain(&gap.departures) {
                let segment = &self.segments[index];
                let vertex = &self.vertices[segment.from];
                let lane = vertex.lane + segment.departure;
                if let Some(mark) = segment
                    .link
                    .start
                    .glyph(axes.direction.reversed(), canvas.ascii)
                {
                    let primary =
                        vertex.departure(&self.layers[rank - 1], segment.departure, axes.direction);
                    canvas.glyph(axes.point(primary + 1, lane), mark, segment.style.border())?;
                }
                if let Some(label) = segment.label {
                    let paint = Paint {
                        bg: segment.style.fill.or(canvas.label_background),
                        fg: segment.style.text,
                    };
                    if horizontal {
                        let (x1, y) = axes.point(gap.start + 1, lane);
                        let (x2, _) = axes.point(gap.start + label.columns + 2, lane);
                        for x in x1.min(x2)..=x1.max(x2) {
                            canvas.glyph(
                                (x, y),
                                ' ',
                                Paint {
                                    fg: None,
                                    bg: paint.bg,
                                },
                            )?;
                        }
                        canvas.text((x1.min(x2) + 1, y), label, paint)?;
                    } else if gap.labels_at_targets {
                        let target = self.vertices[segment.to].lane + segment.arrival;
                        let (x, first) = axes.point(track, target - label.columns / 2);
                        let (_, last) = axes.point(track + label.rows - 1, target);
                        canvas.text((x, first.min(last)), label, paint)?;
                    } else {
                        let (x, first) = axes.point(gap.start, lane + 2);
                        let (_, last) = axes.point(gap.start + label.rows - 1, lane + 2);
                        canvas.text((x, first.min(last)), label, paint)?;
                    }
                }
            }
        }
        for (segment, (departure, arrival)) in self.segments.iter().zip(side_tracks) {
            if let Some(side) = segment.side_lane {
                let lane = self.side_start + side;
                canvas.connect(
                    axes.point(departure, lane),
                    axes.point(arrival, lane),
                    segment,
                    (false, false),
                )?;
            }
        }
        Ok(())
    }
}

struct Wire {
    segment: usize,
    lane: usize,
    target: usize,
}

fn route(wires: impl Iterator<Item = Wire>, spare_lane: usize) -> Vec<Bend> {
    let mut wires: Vec<_> = wires.collect();
    let occupied: BTreeSet<_> = wires.iter().map(|wire| wire.lane).collect();
    wires.retain(|wire| wire.lane != wire.target);
    wires.sort_by_key(|wire| {
        if wire.target < wire.lane {
            (false, wire.lane)
        } else {
            (true, usize::MAX - wire.lane)
        }
    });
    let mut ready = BinaryHeap::new();
    let mut waiting = BTreeMap::new();
    for (index, wire) in wires.iter().enumerate() {
        if occupied.contains(&wire.target) {
            waiting.insert(wire.target, index);
        } else {
            ready.push(Reverse(index));
        }
    }
    let mut bends = Vec::with_capacity(wires.len());
    let mut finished = vec![false; wires.len()];
    let mut remaining = wires.len();
    let mut first_pending = 0;
    // Move one wire at a time into a free lane so parallel runs never overlap.
    while remaining > 0 {
        let (index, lane) = if let Some(Reverse(index)) = ready.pop() {
            finished[index] = true;
            remaining -= 1;
            (index, wires[index].target)
        } else {
            // The remaining moves are permutations of occupied lanes. Free one lane
            // to complete a permutation before the spare is needed again.
            while finished[first_pending] {
                first_pending += 1;
            }
            (first_pending, spare_lane)
        };
        let wire = &mut wires[index];
        if let Some(index) = waiting.remove(&wire.lane) {
            ready.push(Reverse(index));
        }
        wire.lane = lane;
        bends.push(Bend {
            segment: wire.segment,
            lane,
        });
    }
    bends
}

#[derive(Clone, Copy)]
enum Cell {
    Stroke(u16),
    // An orthogonal wire can terminate here without changing the diagonal glyph.
    SlantedEdge(char, u16),
    Glyph(char),
    Text(u32),
    Covered,
}

struct Canvas<'a> {
    columns: usize,
    ascii: bool,
    corners: BorderCorners,
    color: bool,
    node_theme: Style,
    label_background: Option<crate::color::Rgb>,
    cells: Vec<Cell>,
    paints: Vec<Paint>,
    labels: Vec<Cow<'a, str>>,
}

impl<'a> Canvas<'a> {
    fn new(columns: usize, rows: usize, options: &Options, node_theme: Style) -> Self {
        Self {
            columns,
            ascii: options.ascii,
            corners: options.corners,
            color: options.color,
            label_background: options.config.theme_color("edgeLabelBackground"),
            node_theme,
            cells: vec![Cell::Stroke(0); columns * rows],
            paints: Vec::new(),
            labels: Vec::new(),
        }
    }

    fn cell(&mut self, (x, y): (usize, usize)) -> Result<&mut Cell, &'static str> {
        if x >= self.columns {
            return Err("flowchart layout exceeded its canvas");
        }
        self.cells
            .get_mut(y * self.columns + x)
            .ok_or("flowchart layout exceeded its canvas")
    }

    fn paint(&mut self, (x, y): (usize, usize), width: usize, paint: Paint) {
        if !self.color || width == 0 {
            return;
        }
        if self.paints.is_empty() {
            if paint == Paint::default() {
                return;
            }
            self.paints.resize(self.cells.len(), Paint::default());
        }
        let start = y * self.columns + x;
        self.paints[start..start + width].fill(paint);
    }

    fn glyph(&mut self, at: (usize, usize), ch: char, paint: Paint) -> Result<(), &'static str> {
        *self.cell(at)? = Cell::Glyph(ch);
        self.paint(at, 1, paint);
        Ok(())
    }

    fn text(
        &mut self,
        (x, y): (usize, usize),
        label: Label<'a>,
        paint: Paint,
    ) -> Result<(), &'static str> {
        for (row, line) in label.lines().enumerate() {
            self.text_line((x, y + row), line, paint)?;
        }
        Ok(())
    }

    fn text_line(
        &mut self,
        (x, y): (usize, usize),
        text: impl Into<Cow<'a, str>>,
        paint: Paint,
    ) -> Result<(), &'static str> {
        let text = text.into();
        let width = text.width();
        if width == 0 {
            return Ok(());
        }
        let index = u32::try_from(self.labels.len()).map_err(|_| CANVAS_LIMIT)?;
        self.labels.push(text);
        *self.cell((x, y))? = Cell::Text(index);
        for offset in 1..width {
            *self.cell((x + offset, y))? = Cell::Covered;
        }
        self.paint((x, y), width, paint);
        Ok(())
    }

    fn stroke(&mut self, at: (usize, usize), direction: u16) -> Result<(), &'static str> {
        let bits = match self.cell(at)? {
            Cell::Stroke(bits) | Cell::SlantedEdge(_, bits) => bits,
            _ => return Err("flowchart layout overlaps a label or glyph"),
        };
        if stroke_mask(*bits) & stroke_mask(direction) != 0 {
            return Err("flowchart connectors share a canvas lane");
        }
        *bits |= direction;
        Ok(())
    }

    fn contour(&mut self, at: (usize, usize), contour: Contour) -> Result<(), &'static str> {
        let Cell::Stroke(bits) = self.cell(at)? else {
            return Err("invalid node contour placement");
        };
        *bits = (*bits & 0x0fff) | (contour as u16) << 12;
        Ok(())
    }

    fn wire(
        &mut self,
        (x1, y1): (usize, usize),
        (x2, y2): (usize, usize),
        style: EdgeStyle,
    ) -> Result<(), &'static str> {
        let shift = match style {
            EdgeStyle::Solid => 0,
            EdgeStyle::Dotted => 4,
            EdgeStyle::Thick => 8,
            EdgeStyle::Invisible => return Ok(()),
        };
        let (up, right, down, left) = (1 << shift, 2 << shift, 4 << shift, 8 << shift);
        if y1 == y2 {
            for x in x1.min(x2)..x1.max(x2) {
                self.stroke((x, y1), right)?;
                self.stroke((x + 1, y1), left)?;
            }
        } else if x1 == x2 {
            for y in y1.min(y2)..y1.max(y2) {
                self.stroke((x1, y), down)?;
                self.stroke((x1, y + 1), up)?;
            }
        } else {
            return Err("flowchart connector is not orthogonal");
        }
        Ok(())
    }

    fn connect(
        &mut self,
        from: (usize, usize),
        to: (usize, usize),
        segment: &Segment<'_>,
        ports: (bool, bool),
    ) -> Result<(), &'static str> {
        self.wire(from, to, segment.link.style)?;
        let paint = segment.style.border();
        if !self.color || paint == Paint::default() || from == to {
            return Ok(());
        }

        // Junction cells at node ports retain the node's frame paint.
        let ((x1, y1), (x2, y2), (start_port, end_port)) = if from < to {
            (from, to, ports)
        } else {
            (to, from, (ports.1, ports.0))
        };
        if y1 == y2 {
            let start = x1 + usize::from(start_port);
            let end = x2 + usize::from(!end_port);
            self.paint((start, y1), end - start, paint);
        } else {
            for y in y1 + usize::from(start_port)..y2 + usize::from(!end_port) {
                self.paint((x1, y), 1, paint);
            }
        }
        Ok(())
    }

    fn slanted_node(
        &mut self,
        (x1, y1): (usize, usize),
        (x2, y2): (usize, usize),
        node: Node<'a>,
    ) -> Result<(), &'static str> {
        let style = self.node_theme.overlay(node.style);
        let fill = Paint {
            fg: None,
            bg: style.fill,
        };
        let frame = style.border();
        let ink = style.ink();
        let top = y1.min(y2);
        let bottom = y1.max(y2);
        let row_width = x1.abs_diff(x2) + 1 - (bottom - top);
        let slash = if self.ascii { '/' } else { '╱' };
        let label_top = top + (bottom - top + 1 - node_label_rows(node)) / 2;
        let mut labels = node_label_lines(node);
        for y in top..=bottom {
            let left = x1.min(x2) + (bottom - y);
            let right = left + row_width - 1;
            for x in [left, right] {
                *self.cell((x, y))? = Cell::SlantedEdge(slash, 0);
                self.paint((x, y), 1, frame);
            }
            if y == top || y == bottom {
                self.paint((left + 1, y), row_width - 2, frame);
                for x in left + 1..right {
                    self.stroke((x, y), 0b1010)?;
                }
            } else {
                self.paint((left + 1, y), row_width - 2, fill);
                if y >= label_top
                    && let Some(line) = labels.next()
                {
                    self.text_line((left + (row_width - line.width()) / 2, y), line, ink)?;
                }
            }
        }
        Ok(())
    }

    fn node(
        &mut self,
        (x1, y1): (usize, usize),
        (x2, y2): (usize, usize),
        node: Node<'a>,
    ) -> Result<(), &'static str> {
        use Outline::*;
        let (left, right) = (x1.min(x2), x1.max(x2));
        let (top, bottom) = (y1.min(y2), y1.max(y2));
        let outline = node.shape.outline;
        let style = self.node_theme.overlay(node.style);
        let fill = Paint {
            fg: None,
            bg: style.fill,
        };
        let frame = style.border();
        let ink = style.ink();
        let border = if node.body.is_some() || matches!(outline, Fork) {
            EdgeStyle::Thick
        } else {
            EdgeStyle::Solid
        };
        self.wire((left, top), (right, top), border)?;
        self.wire((left, bottom), (right, bottom), border)?;
        self.wire((left, top), (left, bottom), border)?;
        self.wire((right, top), (right, bottom), border)?;
        self.paint((left, top), right - left + 1, frame);
        self.paint((left, bottom), right - left + 1, frame);
        for y in top + 1..bottom {
            self.paint((left, y), 1, frame);
            self.paint((left + 1, y), right - left - 1, fill);
            self.paint((right, y), 1, frame);
        }

        let group = node.body.is_some();
        if group || (matches!(outline, Rectangle) && node.shape.note.is_none()) {
            for at in [(left, top), (right, top), (left, bottom), (right, bottom)] {
                self.contour(at, Contour::RectangleCorner)?;
            }
        }
        let corners = match (outline, self.ascii, group) {
            (_, _, true)
            | (Rectangle | FixedRectangle | Subroutine | Fork | Text | Brace { .. }, _, _) => None,
            (Hexagon | Trapezoid, false, _) => Some(['╱', '╲', '╲', '╱']),
            (Hexagon | Trapezoid, true, _) => Some(['/', '\\', '\\', '/']),
            (TrapezoidInverted, false, _) => Some(['╲', '╱', '╱', '╲']),
            (TrapezoidInverted, true, _) => Some(['\\', '/', '/', '\\']),
            (SlantedRight, false, _) => Some(['╱'; 4]),
            (SlantedRight, true, _) => Some(['/'; 4]),
            (SlantedLeft, false, _) => Some(['╲'; 4]),
            (SlantedLeft, true, _) => Some(['\\'; 4]),
            (Flag, false, _) => Some(['╲', '┐', '╱', '┘']),
            (Flag, true, _) => Some(['\\', '+', '/', '+']),
            (Circle | DoubleCircle, false, _) => Some(['◜', '◝', '◟', '◞']),
            (Circle | DoubleCircle | Stadium, true, _) => Some(['/', '\\', '\\', '/']),
            (Triangle { down: false }, false, _) => Some(['╱', '╲', '└', '┘']),
            (Triangle { down: false }, true, _) => Some(['/', '\\', '+', '+']),
            (Triangle { down: true }, false, _) => Some(['┌', '┐', '╲', '╱']),
            (Triangle { down: true }, true, _) => Some(['+', '+', '\\', '/']),
            (Document { .. }, false, _) => Some(['┌', '┐', '╰', '╯']),
            (Document { .. }, true, _) => Some(['+', '+', '\\', '/']),
            (_, false, _) => Some(['╭', '╮', '╰', '╯']),
            (_, true, _) => Some(['.', '.', '\'', '\'']),
        };
        if let Some([ul, ur, dl, dr]) = corners {
            self.glyph((left, top), ul, frame)?;
            self.glyph((right, top), ur, frame)?;
            self.glyph((left, bottom), dl, frame)?;
            self.glyph((right, bottom), dr, frame)?;
        }

        if let Some(body) = node.body {
            self.text((left + 2, top + 1), node.label, ink)?;
            let mut separator = top + 1 + node.label.rows;
            if let Some(port) = node.port_name {
                self.text((left + 2, separator), port, ink)?;
                separator += 1;
            }
            if body.rows > 0 {
                for x in left + 2..right - 1 {
                    self.glyph((x, separator), if self.ascii { '.' } else { '┄' }, frame)?;
                }
                let (body_x, body_y) = (left + 2, separator + 1);
                for (row, line) in body.drawing.plain.lines().enumerate() {
                    self.text_line((body_x, body_y + row), line, fill)?;
                }
                for run in &body.drawing.paints {
                    self.paint(
                        (body_x + run.x, body_y + run.y),
                        run.width,
                        Paint {
                            bg: run.paint.bg.or(style.fill),
                            ..run.paint
                        },
                    );
                }
            }
            return Ok(());
        }

        let mut label_top = top + 1;
        match outline {
            Database => {
                let (start, stroke, end) = if self.ascii {
                    ('\\', '_', '/')
                } else {
                    ('╰', '─', '╯')
                };
                self.glyph((left + 1, label_top), start, frame)?;
                for x in left + 2..right - 1 {
                    self.glyph((x, label_top), stroke, frame)?;
                }
                self.glyph((right - 1, label_top), end, frame)?;
                label_top += 1;
            }
            Subroutine => {
                for y in top + 1..bottom {
                    let glyph = if self.ascii { '|' } else { '│' };
                    self.glyph((left + 1, y), glyph, frame)?;
                    self.glyph((right - 1, y), glyph, frame)?;
                }
            }
            Circle | DoubleCircle | Stadium | Hexagon => {
                let (left_hint, right_hint) = if matches!(outline, Hexagon) {
                    (Contour::LeftPoint, Contour::RightPoint)
                } else {
                    (Contour::LeftRound, Contour::RightRound)
                };
                for y in top + 1..bottom {
                    self.contour((left, y), left_hint)?;
                    self.contour((right, y), right_hint)?;
                }
                if matches!(outline, DoubleCircle) {
                    for y in [top + 1, bottom - 1] {
                        let cap = if self.ascii { '-' } else { '─' };
                        for x in left + 2..right - 1 {
                            self.glyph((x, y), cap, frame)?;
                        }
                    }
                }
            }
            Text | Brace { .. } => {
                for x in left..=right {
                    self.contour((x, top), Contour::Hidden)?;
                    self.contour((x, bottom), Contour::Hidden)?;
                }
                for y in top + 1..bottom {
                    let l = if matches!(outline, Brace { left: true, .. }) {
                        Contour::LeftBrace
                    } else {
                        Contour::Hidden
                    };
                    let r = if matches!(outline, Brace { right: true, .. }) {
                        Contour::RightBrace
                    } else {
                        Contour::Hidden
                    };
                    self.contour((left, y), l)?;
                    self.contour((right, y), r)?;
                }
            }
            Triangle { down } => {
                let (row, tip) = if down {
                    (bottom, Contour::TipDown)
                } else {
                    (top, Contour::TipUp)
                };
                self.contour((left + (right - left) / 2, row), tip)?;
            }
            Document { lined } => {
                for x in left + 1..right {
                    self.contour((x, bottom), Contour::Wave)?;
                }
                if lined {
                    for x in left + 2..right - 1 {
                        self.glyph((x, label_top), if self.ascii { '-' } else { '─' }, frame)?;
                    }
                    label_top += 1;
                }
            }
            _ => {}
        }
        let rows = node_label_rows(node);
        let label_top = label_top + (bottom - label_top - rows).div_ceil(2);
        for (row, line) in node_label_lines(node).enumerate() {
            let start = left + (right - left + 1 - line.width()) / 2;
            self.text_line((start, label_top + row), line, ink)?;
        }
        Ok(())
    }

    fn write(&self, drawing: &mut Drawing, top: usize) {
        for (index, row) in self.cells.chunks_exact(self.columns).enumerate() {
            if index != 0 {
                drawing.plain.push('\n');
            }
            let end = row.iter().rposition(|cell| match *cell {
                Cell::Stroke(bits) => stroke_glyph(bits, self.ascii, self.corners) != ' ',
                Cell::Glyph(ch) | Cell::SlantedEdge(ch, _) => ch != ' ',
                _ => true,
            });
            if let Some(end) = end {
                for cell in &row[..=end] {
                    match *cell {
                        Cell::Stroke(bits) => {
                            drawing
                                .plain
                                .push(stroke_glyph(bits, self.ascii, self.corners))
                        }
                        Cell::Glyph(ch) | Cell::SlantedEdge(ch, _) => drawing.plain.push(ch),
                        Cell::Text(index) => drawing.plain.push_str(&self.labels[index as usize]),
                        Cell::Covered => {}
                    }
                }
                if !self.paints.is_empty() {
                    let start = index * self.columns;
                    let mut x = 0;
                    for run in self.paints[start..=start + end].chunk_by(|a, b| a == b) {
                        if run[0] != Paint::default() {
                            drawing.paints.push(PaintRun {
                                x,
                                y: top + index,
                                width: run.len(),
                                paint: run[0],
                            });
                        }
                        x += run.len();
                    }
                }
            }
        }
    }
}

// The upper four bits decorate node contours. The lower bits reserve wire directions.
#[derive(Clone, Copy)]
#[repr(u16)]
enum Contour {
    LeftRound = 1,
    RightRound = 2,
    LeftPoint = 3,
    RightPoint = 4,
    Hidden = 5,
    LeftBrace = 6,
    RightBrace = 7,
    Wave = 8,
    TipUp = 9,
    TipDown = 10,
    RectangleCorner = 11,
}

fn stroke_mask(bits: u16) -> u16 {
    (bits | (bits >> 4) | (bits >> 8)) & 15
}

fn stroke_glyph(bits: u16, ascii: bool, corners: BorderCorners) -> char {
    // Four-way crossings belong to separate wires. Box ports are T-junctions.
    const UNICODE: [char; 16] = [
        ' ', '│', '─', '└', '│', '│', '┌', '├', '─', '┘', '─', '┴', '┐', '┤', '┬', '╪',
    ];
    const HEAVY: [char; 16] = [
        ' ', '┃', '━', '┗', '┃', '┃', '┏', '┣', '━', '┛', '━', '┻', '┓', '┫', '┳', '╪',
    ];
    const ASCII: [char; 16] = [
        ' ', '|', '-', '+', '|', '|', '+', '+', '-', '+', '-', '+', '+', '+', '+', 'x',
    ];
    let (solid, dotted, thick) = (bits & 15, (bits >> 4) & 15, (bits >> 8) & 15);
    let mask = solid | dotted | thick;
    let glyphs = match (ascii, solid, dotted) {
        (true, _, _) => &ASCII,
        (false, 0, 0) => &HEAVY,
        _ => &UNICODE,
    };
    match (bits >> 12, mask) {
        (1, 5) => return '(',
        (2, 5) => return ')',
        (3, 5) => return '<',
        (4, 5) => return '>',
        (6, 5) => return '{',
        (7, 5) => return '}',
        (5..=7, 7 | 13) => return if ascii { '-' } else { '─' },
        (5..=7, 11 | 14) => return if ascii { '|' } else { '│' },
        (5..=7, _) => return ' ',
        (8, 10) => return if ascii { '~' } else { '∼' },
        (9, 10) => return if ascii { '^' } else { '△' },
        (10, 10) => return if ascii { 'v' } else { '▽' },
        (contour, _) if contour == Contour::RectangleCorner as u16 => {
            let masks = [6, 12, 3, 9];
            if let Some(index) = masks.iter().position(|&corner| corner == mask) {
                return corners.resolve(ascii, masks.map(|mask| glyphs[usize::from(mask)]))[index];
            }
        }
        _ => {}
    }
    match (ascii, solid, dotted, thick, mask) {
        (false, 0, _, 0, 5) => '┆',
        (false, 0, _, 0, 10) => '┄',
        (true, 0, 0, _, 5) => '#',
        (true, 0, 0, _, 10) => '=',
        (true, 0, _, 0, 5) => ':',
        (true, 0, _, 0, 10) => '.',
        _ => glyphs[usize::from(mask)],
    }
}
