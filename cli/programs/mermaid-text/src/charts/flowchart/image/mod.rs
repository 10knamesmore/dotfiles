use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::Options;
use crate::charts::shared::Direction;
use crate::color::{Rgb, Style};
use crate::graphics::Scene;

use super::groups::{Entry, Hierarchy};
use super::{Edge, EdgeStyle, Link, Outline, Topology};

mod routing;
mod shapes;

const STEP: f32 = 10.0;
const LINE_HEIGHT: f32 = 22.0;
const MARGIN: i32 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Point(i32, i32);

impl Point {
    fn offset(self, delta: Self) -> Self {
        Self(self.0 + delta.0, self.1 + delta.1)
    }

    fn pixels(self) -> (f32, f32) {
        (self.0 as f32 * STEP, self.1 as f32 * STEP)
    }
}

#[derive(Clone, Copy, Default)]
struct Bounds {
    origin: Point,
    size: Point,
}

impl Bounds {
    fn end(self) -> Point {
        self.origin.offset(self.size)
    }

    fn center(self) -> Point {
        self.origin.offset(Point(self.size.0 / 2, self.size.1 / 2))
    }

    fn pixels(self) -> (f32, f32, f32, f32) {
        let (x, y) = self.origin.pixels();
        let (w, h) = self.size.pixels();
        (x, y, w, h)
    }
}

#[derive(Default)]
struct Text<'a> {
    lines: Vec<(Cow<'a, str>, f32)>,
    width: f32,
    height: f32,
}

impl<'a> Text<'a> {
    fn measure(scene: &Scene, lines: impl Iterator<Item = Cow<'a, str>>) -> Self {
        let lines: Vec<_> = lines
            .map(|line| {
                let width = scene.measure(&line);
                (line, width)
            })
            .collect();
        let width = lines.iter().map(|(_, width)| *width).fold(0.0, f32::max);
        let height = lines.len() as f32 * LINE_HEIGHT;
        Self {
            lines,
            width,
            height,
        }
    }

    fn draw(&self, scene: &mut Scene, center_x: f32, top: f32, ink: Option<Rgb>) {
        for (row, (line, width)) in self.lines.iter().enumerate() {
            scene.text(
                center_x - width / 2.0,
                top + row as f32 * LINE_HEIGHT,
                line,
                ink,
            );
        }
    }
}

fn cells(px: f32) -> i32 {
    (px / STEP).ceil() as i32
}

struct Vertex<'a> {
    bounds: Bounds,
    text: Text<'a>,
    outline: Outline,
    group: bool,
    style: Style,
    notes: Vec<Note<'a>>,
}

pub(in crate::charts) struct Node<'a> {
    pub lines: Vec<Cow<'a, str>>,
    pub outline: Outline,
    pub parent: Option<usize>,
    pub direction: Option<Direction>,
    pub style: Style,
    pub notes: Vec<(bool, Vec<Cow<'a, str>>)>,
}

struct Note<'a> {
    left: bool,
    text: Text<'a>,
}

impl Vertex<'_> {
    fn caption(&self) -> Bounds {
        let size = Point(cells(self.text.width + 16.0), cells(self.text.height + 8.0));
        let center = self.bounds.center();
        Bounds {
            origin: Point(center.0 - size.0 / 2, self.bounds.origin.1 + 1),
            size,
        }
    }

    fn note_layout(&self, left: bool) -> (i32, i32) {
        let mut width = 0;
        let mut height = 0;
        for note in self.notes.iter().filter(|note| note.left == left) {
            width = width.max(cells(note.text.width + 32.0) + 3);
            height += cells(note.text.height + 18.0) + i32::from(height > 0);
        }
        (width, height)
    }

    fn footprint(&self) -> Bounds {
        let (left, left_height) = self.note_layout(true);
        let (right, right_height) = self.note_layout(false);
        let height = self.bounds.size.1.max(left_height).max(right_height);
        Bounds {
            origin: Point(-left, (self.bounds.size.1 - height) / 2),
            size: Point(self.bounds.size.0 + left + right, height),
        }
    }

    fn notes(&self) -> impl Iterator<Item = (&Text<'_>, Bounds)> {
        [true, false].into_iter().flat_map(move |left| {
            let (_, height) = self.note_layout(left);
            let mut top = self.bounds.center().1 - height / 2;
            self.notes
                .iter()
                .filter(move |note| note.left == left)
                .map(move |note| {
                    let size = Point(
                        cells(note.text.width + 32.0),
                        cells(note.text.height + 18.0),
                    );
                    let x = if left {
                        self.bounds.origin.0 - size.0 - 3
                    } else {
                        self.bounds.end().0 + 3
                    };
                    let origin = Point(x, top);
                    top += size.1 + 1;
                    (&note.text, Bounds { origin, size })
                })
        })
    }

    fn draw(&self, scene: &mut Scene, corners: f32) {
        let (x, y, w, h) = self.bounds.pixels();
        let fill = self.style.fill.or(Some(scene.paper()));
        let stroke = self.style.stroke.or(Some(scene.ink()));
        if self.group {
            scene.rect(x, y, w, h, corners, fill, stroke);
            let (_, top) = self.caption().origin.pixels();
            self.text
                .draw(scene, x + w / 2.0, top + 3.0, self.style.text);
        } else {
            shapes::draw(scene, self, fill, stroke, corners);
            let offset = match self.outline {
                Outline::Triangle { down: false } => h * 0.14,
                Outline::Triangle { down: true } => -h * 0.14,
                Outline::Database => 6.0,
                Outline::Document { .. } => -4.0,
                Outline::Synchronization { vertical: true } => -h / 2.0 + self.text.height / 2.0,
                _ => 0.0,
            };
            self.text.draw(
                scene,
                x + w / 2.0,
                y + (h - self.text.height) / 2.0 + offset,
                if matches!(self.outline, Outline::Synchronization { vertical: false }) {
                    self.style.text.or(Some(scene.paper()))
                } else {
                    self.style.text
                },
            );
        }
    }

    fn draw_notes(&self, scene: &mut Scene, style: Style) {
        let stroke = style.stroke.unwrap_or(scene.ink());
        let fill = style.fill.or(Some(scene.paper()));
        let (x, y, w, h) = self.bounds.pixels();
        for (text, note) in self.notes() {
            let (left, top, width, height) = note.pixels();
            let (start, end) = if left < x {
                ((x, y + h / 2.0), (left + width, top + height / 2.0))
            } else {
                ((x + w, y + h / 2.0), (left, top + height / 2.0))
            };
            routing::Trace::annotation(start, end, scene, stroke);
            shapes::note(scene, (left, top, width, height), fill, Some(stroke));
            text.draw(
                scene,
                left + width / 2.0,
                top + (height - text.height) / 2.0,
                style.text,
            );
        }
    }
}

impl<'a> Vertex<'a> {
    fn new(text: Text<'a>, outline: Outline, group: bool, style: Style) -> Self {
        let size = if group {
            Point(cells(text.width + 64.0), cells(text.height + 65.0))
        } else {
            shapes::size(outline, &text)
        };
        Self {
            bounds: Bounds {
                origin: Point::default(),
                size,
            },
            text,
            outline,
            group,
            style,
            notes: Vec::new(),
        }
    }
}

pub(in crate::charts) fn render_graph<'a>(
    nodes: Vec<Node<'a>>,
    links: impl IntoIterator<Item = (usize, usize, Option<Cow<'a, str>>)>,
    direction: Direction,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    let hierarchy = Hierarchy::from_parts(
        nodes.len(),
        nodes
            .iter()
            .map(|node| (node.parent, node.direction.is_some(), node.direction)),
        direction,
    )?;
    let node_theme = options.theme_style();
    let vertices = nodes
        .into_iter()
        .map(|node| {
            let mut vertex = Vertex::new(
                Text::measure(scene, node.lines.into_iter()),
                node.outline,
                node.direction.is_some(),
                node_theme.overlay(node.style),
            );
            vertex.notes = node
                .notes
                .into_iter()
                .map(|(left, lines)| Note {
                    left,
                    text: Text::measure(scene, lines.into_iter()),
                })
                .collect();
            vertex
        })
        .collect();
    let (edges, labels): (Vec<_>, Vec<_>) = links
        .into_iter()
        .map(|(from, to, label)| {
            (
                Edge {
                    from,
                    to,
                    label: None,
                    link: Link::arrow(EdgeStyle::Solid),
                    style: Style::default(),
                },
                Text::measure(scene, label.into_iter()),
            )
        })
        .unzip();
    draw_graph(
        vertices,
        &edges,
        &hierarchy,
        labels,
        options,
        scene,
        Layout::State,
    )
}

pub(super) fn render(
    entries: &[Entry<'_>],
    edges: &[Edge<'_>],
    hierarchy: &Hierarchy,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    let node_theme = options.theme_style();
    let vertices = entries
        .iter()
        .map(|entry| {
            let node = &entry.node;
            let text = Text::measure(
                scene,
                node.label.lines().chain(node.shape.note.map(Cow::Borrowed)),
            );
            Vertex::new(
                text,
                node.shape.outline,
                entry.is_group(),
                node_theme.overlay(node.style),
            )
        })
        .collect();
    let edge_labels: Vec<_> = edges
        .iter()
        .map(|edge| {
            edge.label
                .map(|label| Text::measure(scene, label.lines()))
                .unwrap_or_default()
        })
        .collect();

    draw_graph(
        vertices,
        edges,
        hierarchy,
        edge_labels,
        options,
        scene,
        Layout::Flowchart,
    )
}

#[derive(Clone, Copy)]
enum Layout {
    Flowchart,
    State,
}

fn draw_graph(
    mut vertices: Vec<Vertex<'_>>,
    edges: &[Edge<'_>],
    hierarchy: &Hierarchy,
    edge_labels: Vec<Text<'_>>,
    options: &Options,
    scene: &mut Scene,
    mode: Layout,
) -> Result<(), String> {
    if vertices.is_empty() {
        return Err("graph has no nodes".into());
    }
    let mut order: Vec<_> = (0..edges.len())
        .filter(|&index| edges[index].link.style != EdgeStyle::Invisible)
        .collect();
    let mut parallel = None::<Vec<Option<i32>>>;
    let mut work = routing::MAX_ROUTING_WORK;
    let (router, connections) = loop {
        let size = layout(hierarchy, edges, &edge_labels, &mut vertices, mode)?;
        let (width, height) = size.pixels();
        scene.resize(width, height)?;

        let mut router = routing::Router::new(size, hierarchy, &vertices, work)?;
        let lane = |index: usize| parallel.as_ref().and_then(|lanes| lanes[index]);
        // Reserve parallel tracks before other edges, then prioritize the widest labels.
        order.sort_by(|&a, &b| {
            lane(b)
                .is_some()
                .cmp(&lane(a).is_some())
                .then_with(|| edge_labels[b].width.total_cmp(&edge_labels[a].width))
        });
        let connections = order
            .iter()
            .map(|&index| {
                router
                    .connect(
                        edges[index],
                        hierarchy,
                        &vertices,
                        &edge_labels[index],
                        lane(index),
                    )
                    .map(|trace| (index, trace))
                    .map_err(|error| (index, error))
            })
            .collect::<Result<Vec<_>, _>>();
        match connections {
            Ok(connections) => break (router, connections),
            Err((index, routing::ConnectError::Label))
                if matches!(mode, Layout::Flowchart) && parallel.is_none() =>
            {
                parallel = parallel_lanes(edges, &edge_labels, hierarchy, &mut vertices, index);
                if parallel.is_none() {
                    return Err(routing::ConnectError::Label.into());
                }
                work = router.work;
            }
            Err((_, error)) => return Err(error.into()),
        }
    };

    let corners = if options.corners.resolve(false, ['╭', '╮', '╰', '╯']) == ['╭', '╮', '╰', '╯']
    {
        7.0
    } else {
        0.0
    };
    for &group in &hierarchy.scopes[1..] {
        vertices[group].draw(scene, corners);
    }
    let default_stroke = options.theme_line().unwrap_or_else(|| scene.ink());
    let label_background = options
        .color
        .then(|| options.config.theme_color("edgeLabelBackground"))
        .flatten();
    for (index, trace) in &connections {
        let edge = edges[*index];
        let stroke = edge.style.stroke.unwrap_or(default_stroke);
        trace.draw_line(scene, edge.link, stroke);
    }
    for vertex in vertices.iter().filter(|vertex| !vertex.group) {
        vertex.draw(scene, corners);
    }
    let note_style = if options.color {
        let color = |key| options.config.theme_color(key);
        Style {
            fill: color("noteBkgColor"),
            stroke: color("noteBorderColor"),
            text: color("noteTextColor"),
        }
    } else {
        Style::default()
    };
    for vertex in &vertices {
        vertex.draw_notes(scene, note_style);
    }
    for (index, trace) in connections {
        let edge = edges[index];
        let stroke = edge.style.stroke.unwrap_or(default_stroke);
        trace.draw_tips(scene, edge.link, stroke);
        if let Some(position) = trace.label {
            let text = &edge_labels[index];
            let (x, y) = position.pixels();
            let base = router.owner(position);
            let base = vertices.get(base).and_then(|group| group.style.fill);
            let paper = edge
                .style
                .fill
                .or(label_background)
                .or(base)
                .or(Some(scene.paper()));
            scene.rect(
                x - text.width / 2.0 - 5.0,
                y - 3.0,
                text.width + 10.0,
                text.height + 2.0,
                3.0,
                paper,
                None,
            );
            text.draw(scene, x, y - 3.0, edge.style.text);
        }
    }
    Ok(())
}

fn parallel_lanes(
    edges: &[Edge<'_>],
    labels: &[Text<'_>],
    hierarchy: &Hierarchy,
    vertices: &mut [Vertex<'_>],
    failed: usize,
) -> Option<Vec<Option<i32>>> {
    let mut pairs = BTreeMap::<_, Vec<_>>::new();
    for (index, edge) in edges.iter().enumerate() {
        if edge.link.style != EdgeStyle::Invisible
            && edge.from != edge.to
            && hierarchy.parents[edge.from] == hierarchy.parents[edge.to]
        {
            pairs.entry((edge.from, edge.to)).or_default().push(index);
        }
    }
    pairs.retain(|_, edges| {
        edges
            .iter()
            .filter(|&&index| !labels[index].lines.is_empty())
            .take(2)
            .count()
            == 2
    });
    let failed = edges[failed];
    if !pairs.contains_key(&(failed.from, failed.to)) {
        return None;
    }
    let mut lanes = vec![None; edges.len()];
    for ((from, to), edges) in pairs {
        let horizontal = Axes::new(hierarchy.directions[hierarchy.parents[from]]).horizontal;
        let spans = edges.iter().map(|&index| {
            let label = &labels[index];
            if horizontal {
                cells(label.height) + 2
            } else {
                2 * cells(label.width / 2.0 + 5.0) + 2
            }
        });
        let width = spans.clone().sum::<i32>();
        let mut lane = -width / 2;
        for (index, span) in edges.iter().copied().zip(spans) {
            lanes[index] = Some(lane + span / 2);
            lane += span;
        }
        // Keep the outer lanes away from the corners of either endpoint.
        for id in [from, to] {
            let vertex = &mut vertices[id];
            let size = &mut vertex.bounds.size;
            let cross = if horizontal { &mut size.1 } else { &mut size.0 };
            *cross = (*cross).max(width + 2);
            if matches!(vertex.outline, Outline::Circle | Outline::DoubleCircle) {
                let diameter = size.0.max(size.1);
                *size = Point(diameter, diameter);
            }
        }
    }
    Some(lanes)
}

#[derive(Clone, Copy)]
struct Axes {
    horizontal: bool,
    reversed: bool,
}

impl Axes {
    fn new(direction: Direction) -> Self {
        Self {
            horizontal: matches!(direction, Direction::Right | Direction::Left),
            reversed: matches!(direction, Direction::Left | Direction::Up),
        }
    }

    fn point(self, primary: i32, cross: i32) -> Point {
        if self.horizontal {
            Point(primary, cross)
        } else {
            Point(cross, primary)
        }
    }

    fn spans(self, point: Point) -> (i32, i32) {
        if self.horizontal {
            (point.0, point.1)
        } else {
            (point.1, point.0)
        }
    }
}

fn layout(
    hierarchy: &Hierarchy,
    edges: &[Edge<'_>],
    labels: &[Text<'_>],
    vertices: &mut [Vertex<'_>],
    mode: Layout,
) -> Result<Point, String> {
    let mut scoped = vec![Vec::new(); hierarchy.root + 1];
    let mut padding = vec![Point(3, 3); hierarchy.root + 1];
    padding[hierarchy.root] = Point(MARGIN, MARGIN);
    for (index, edge) in edges.iter().enumerate() {
        let (scope, from, to) = hierarchy.project(edge.from, edge.to);
        if from == to && !labels[index].lines.is_empty() {
            let height = cells(labels[index].height) + 4;
            if edge.from == edge.to {
                padding[scope].1 = padding[scope].1.max(height);
            } else {
                // A connector between a group and its own member runs inside the frame.
                let width = cells(labels[index].width + 20.0);
                padding[from].0 = padding[from].0.max(width);
                padding[from].1 = padding[from].1.max(height);
            }
        }
        scoped[scope].push((hierarchy.positions[from], hierarchy.positions[to], index));
    }
    let mut diagram = Point::default();
    for &scope in hierarchy.scopes.iter().rev() {
        let axes = Axes::new(hierarchy.directions[scope]);
        let members = &hierarchy.members[scope];
        let links = &scoped[scope];
        let size = layout_scope(vertices, members, links, labels, axes);
        let label_span = links
            .iter()
            .map(|&(_, _, edge)| labels[edge].width)
            .fold(0.0, f32::max);
        let label_allowance = if axes.horizontal || label_span == 0.0 {
            10.0
        } else {
            50.0
        };
        let body_width = if matches!(mode, Layout::State) && !axes.horizontal && label_span > 0.0 {
            size.0 + 2 * cells(label_span + 20.0)
        } else {
            size.0.max(cells(label_span + label_allowance))
        };
        let Point(pad_x, pad_y) = padding[scope];
        if scope == hierarchy.root {
            diagram = Point(body_width + 2 * pad_x, size.1 + 2 * pad_y);
            let left = pad_x + (body_width - size.0) / 2;
            for &member in members {
                vertices[member].bounds.origin =
                    vertices[member].bounds.origin.offset(Point(left, pad_y));
            }
        } else {
            let top = cells(vertices[scope].text.height) + 1 + pad_y;
            let bounds = &mut vertices[scope].bounds;
            bounds.size.0 = bounds.size.0.max(body_width + 2 * pad_x);
            bounds.size.1 = bounds.size.1.max(size.1 + top + pad_y);
            let left = (bounds.size.0 - size.0) / 2;
            for &member in members {
                vertices[member].bounds.origin =
                    vertices[member].bounds.origin.offset(Point(left, top));
            }
        }
    }
    for &scope in &hierarchy.scopes[1..] {
        let origin = vertices[scope].bounds.origin;
        for &member in &hierarchy.members[scope] {
            vertices[member].bounds.origin = vertices[member].bounds.origin.offset(origin);
        }
    }
    if diagram.0 <= 0 || diagram.1 <= 0 {
        return Err("invalid flowchart image dimensions".into());
    }
    Ok(diagram)
}

fn layout_scope(
    vertices: &mut [Vertex<'_>],
    members: &[usize],
    links: &[(usize, usize, usize)],
    labels: &[Text<'_>],
    axes: Axes,
) -> Point {
    let rank_links: Vec<_> = links.iter().filter(|&&(from, to, _)| from != to).collect();
    let topology = Topology::from_endpoints(members.len(), rank_links.len(), |i| {
        (rank_links[i].0, rank_links[i].1)
    });
    let mut gaps = vec![7; members.len()];
    let mut label_spans = vec![3; members.len()];
    let mut cross_gaps = vec![4; members.len()];
    let mut neighbors = vec![Vec::new(); members.len()];
    let mut port_labels = BTreeMap::new();
    let mut cross_margin = 0;
    for &(from, to, index) in links {
        let a = topology.ranks[from];
        let b = topology.ranks[to];
        if a == b {
            continue;
        }
        neighbors[from].push(to);
        neighbors[to].push(from);
        let label = &labels[index];
        if !label.lines.is_empty() {
            let (span, cross) = if axes.horizontal {
                (label.width, label.height)
            } else {
                (label.height, label.width)
            };
            for rank in [a, b] {
                cross_gaps[rank] = cross_gaps[rank].max(cells(cross) + 2);
            }
            let gap = a.min(b);
            label_spans[gap] += cells(span) + 2;
            gaps[gap] = gaps[gap].max(label_spans[gap]);
            for endpoint in [from, to] {
                let count = port_labels.entry((gap, endpoint)).or_insert(0);
                *count += 1;
                cross_margin = cross_margin.max((*count - 2).clamp(0, 4));
            }
        }
    }

    let mut max_primary = 0;
    let mut next_cross = cross_margin;
    let mut order = vec![0; members.len()];
    for component in topology.components {
        let count = component
            .iter()
            .map(|&index| topology.ranks[index])
            .max()
            .unwrap_or(0)
            + 1;
        let mut layers = vec![Vec::new(); count];
        for index in component {
            let layer = &mut layers[topology.ranks[index]];
            order[index] = layer.len();
            layer.push(index);
        }
        // Order adjacent ranks by the positions of their neighbors.
        for forward in [true, false, true] {
            for i in 0..layers.len() {
                let rank = if forward { i } else { layers.len() - 1 - i };
                layers[rank].sort_by_cached_key(|&member| {
                    let (sum, count) = neighbors[member]
                        .iter()
                        .filter(|&&other| {
                            topology.ranks[other].abs_diff(rank) == 1
                                && (topology.ranks[other] < rank) == forward
                        })
                        .fold((0, 0), |(sum, count), &other| {
                            (sum + order[other], count + 1)
                        });
                    (sum * 1024)
                        .checked_div(count)
                        .unwrap_or(order[member] * 1024)
                });
                for (position, &index) in layers[rank].iter().enumerate() {
                    order[index] = position;
                }
            }
        }

        let layer_sizes: Vec<_> = layers
            .iter()
            .zip(&cross_gaps)
            .map(|(layer, &gap)| {
                let (primary, cross) = layer
                    .iter()
                    .map(|&index| axes.spans(vertices[members[index]].footprint().size))
                    .fold((0, 0), |(primary, cross), (p, c)| {
                        (primary.max(p), cross + c + gap)
                    });
                (primary, (cross - gap).max(0))
            })
            .collect();
        let cross = layer_sizes
            .iter()
            .map(|&(_, cross)| cross)
            .max()
            .unwrap_or(0);
        let primary = layer_sizes
            .iter()
            .enumerate()
            .map(|(rank, &(primary, _))| primary + if rank + 1 < count { gaps[rank] } else { 0 })
            .sum();
        let mut layer_start = 0;
        for (rank, (layer, &(span, cross_span))) in layers.iter().zip(&layer_sizes).enumerate() {
            let mut lane = next_cross + (cross - cross_span) / 2;
            for &index in layer {
                let vertex = &mut vertices[members[index]];
                let footprint = vertex.footprint();
                let (p, c) = axes.spans(footprint.size);
                let start = layer_start + (span - p) / 2;
                vertex.bounds.origin = axes
                    .point(
                        if axes.reversed {
                            primary - start - p
                        } else {
                            start
                        },
                        lane,
                    )
                    .offset(Point(-footprint.origin.0, -footprint.origin.1));
                lane += c + cross_gaps[rank];
            }
            layer_start += span + gaps[rank];
        }
        max_primary = max_primary.max(primary);
        next_cross += cross + 5;
    }
    axes.point(max_primary, (next_cross - 5 + cross_margin).max(0))
}
