use std::fmt::Write;

use super::{
    Diagram, Edge, Kind, Node, Tip,
    layout::{groups, labels},
    parse,
};
use crate::{
    Options,
    charts::shared::plot::image::{self, Point},
    color::{Rgb, Style},
    graphics::Scene,
};

const MARGIN: f32 = 20.0;
const GAP: f32 = 52.0;
const MEMBER_GAP: f32 = 30.0;

#[derive(Clone, Copy, Default)]
struct Bounds {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Bounds {
    fn anchor(self, axis: Axis, sign: f32) -> Point {
        let (x, y) = (self.x + self.w / 2.0, self.y + self.h / 2.0);
        if axis.horizontal {
            (x + sign * self.w / 2.0, y)
        } else {
            (x, y + sign * self.h / 2.0)
        }
    }
}

#[derive(Clone, Copy)]
struct Axis {
    horizontal: bool,
    top: f32,
}

impl Axis {
    fn swap(self, (x, y): Point) -> Point {
        if self.horizontal { (x, y) } else { (y, x) }
    }

    fn xy(self, along: f32, across: f32) -> Point {
        let (x, y) = self.swap((along, across));
        (x + MARGIN, y + self.top)
    }

    fn ab(self, (x, y): Point) -> Point {
        self.swap((x - MARGIN, y - self.top))
    }

    fn bounds(self, along: f32, across: f32, size: Point) -> Bounds {
        let (x, y) = self.xy(along, across);
        let (w, h) = self.swap(size);
        Bounds { x, y, w, h }
    }

    fn cross(self) -> Self {
        Self {
            horizontal: !self.horizontal,
            ..self
        }
    }
}

struct Visual {
    rows: Vec<String>,
    size: Point,
    bounds: Bounds,
}

impl Visual {
    fn new(scene: &Scene, node: &Node) -> Self {
        let mut text = labels(node, false);
        if let Some(icon) = node.attr.get("icon") {
            text.insert(0, icon.clone());
        }
        let rows: Vec<_> = text
            .iter()
            .flat_map(|s| image::wrap(scene, s, 235.0))
            .collect();
        let widest = rows.iter().map(|s| scene.measure(s)).fold(0.0, f32::max);
        let height = 20.0 * rows.len() as f32;
        let size = match node.kind {
            Kind::Actor => ((widest + 16.0).max(68.0), height + 66.0),
            Kind::Ellipse => ((widest * 1.30 + 30.0).max(120.0), height * 1.45 + 35.0),
            Kind::Boundary => (widest + 36.0, height + 26.0),
            _ => ((widest + 30.0).max(110.0), height + 24.0),
        };
        Self {
            rows,
            size,
            bounds: Bounds::default(),
        }
    }

    fn port(&self, node: &Node, axis: Axis, sign: f32, offset: f32) -> Point {
        let (x, y) = self.bounds.anchor(axis, sign);
        if node.kind == Kind::Actor {
            let cx = self.bounds.x + self.bounds.w / 2.0;
            return if axis.horizontal {
                (cx + sign * 15.0, self.bounds.y + 31.0)
            } else if sign < 0.0 {
                (cx, self.bounds.y + 6.0)
            } else {
                (cx + 15.0, self.bounds.y + 31.0)
            };
        }
        if node.kind != Kind::Ellipse {
            return (x, y);
        }
        let (w, h) = axis.swap((self.bounds.w, self.bounds.h));
        let inset = w / 2.0 * (1.0 - (1.0 - offset * offset).sqrt());
        let (dx, dy) = axis.swap((-sign * inset, offset * h / 2.0));
        (x + dx, y + dy)
    }

    fn exit(&self, node: &Node, axis: Axis, sign: f32, port: Point) -> Point {
        if node.kind == Kind::Actor && !axis.horizontal && sign > 0.0 {
            (self.bounds.x + self.bounds.w + 8.0, port.1)
        } else {
            port
        }
    }
}

struct Panel {
    boundary: Option<usize>,
    members: Vec<usize>,
    leading: f32,
    trailing: f32,
    depth: f32,
}

struct Relation<'a> {
    edge: &'a Edge,
    from: usize,
    to: usize,
    label: Vec<String>,
    width: f32,
}

fn actor(scene: &mut Scene, center: f32, top: f32, node: &Node, style: Style) {
    let stroke = style.stroke.unwrap_or(scene.ink());
    let kind = node.attr.get("type").map(String::as_str);
    let fill = style.fill.or(Some(scene.paper()));
    image::circle(scene, (center, top + 12.0), 8.0, fill, Some(stroke), 2.0);
    if kind == Some("hollow") {
        image::circle(scene, (center, top + 12.0), 4.0, None, Some(stroke), 1.2);
    }
    let arm = top + 29.0;
    let leg = top + 37.0;
    let base = top + 52.0;
    let (left, right) = (center - 15.0, center + 15.0);
    scene.path(
        &format!("M {center} {} V {leg} M {left} {arm} H {right} M {left} {base} L {center} {leg} L {right} {base}", top + 20.0),
        None, Some(stroke), if kind == Some("awesome") { 4.0 } else { 2.0 },
    );
    if node.attr.get("business").is_some_and(|v| v == "true") {
        image::line(
            scene,
            (center - 9.0, top + 20.0),
            (center + 11.0, top + 41.0),
            stroke,
            1.5,
        );
    }
}

fn render_node(scene: &mut Scene, node: &Node, visual: &Visual, style: Style) {
    let Bounds { x, y, w, h } = visual.bounds;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let stroke = Some(style.stroke.unwrap_or(scene.ink()));
    let fill = Some(style.fill.unwrap_or(scene.paper()));
    match node.kind {
        Kind::Actor => actor(scene, cx, y + 2.0, node, style),
        Kind::Ellipse => scene.path(
            &format!(
                "M {x} {cy} A {} {} 0 1 1 {} {cy} A {} {} 0 1 1 {x} {cy} Z",
                w / 2.0,
                h / 2.0,
                x + w,
                w / 2.0,
                h / 2.0
            ),
            fill,
            stroke,
            1.7,
        ),
        _ => scene.rect(x, y, w, h, 5.0, fill, stroke),
    }
    let ink = style
        .text
        .or_else(|| style.fill.map(|f| image::contrast(f, scene.paper())));
    let top = if node.kind == Kind::Actor {
        y + 58.0
    } else {
        cy - 10.0 * visual.rows.len() as f32
    };
    for (i, text) in visual.rows.iter().enumerate() {
        let y = top + 20.0 * i as f32;
        if node.kind == Kind::Json {
            scene.text(x + 15.0, y, text, ink);
        } else {
            image::center(scene, cx, y, text, ink);
        }
    }
}

fn boundary(scene: &mut Scene, node: &Node, visual: &Visual, style: Style) {
    let Bounds { x, y, w, h } = visual.bounds;
    let stroke = style
        .stroke
        .unwrap_or_else(|| image::fade(scene.ink(), 0.65));
    let fill = style.fill.or_else(|| Some(image::fade(scene.ink(), 0.055)));
    let tab = node.attr.get("type").is_some_and(|kind| kind == "package");
    if tab {
        let tab_width = visual.size.0.min(w - 12.0);
        let right = x + w;
        let bottom = y + h;
        scene.path(
            &format!("M {x} {bottom} V {y} h {tab_width} l 12 12 H {right} V {bottom} Z"),
            fill,
            Some(stroke),
            1.7,
        );
    } else {
        scene.rect(x, y, w, h, 7.0, fill, Some(stroke));
    }
    for (i, text) in visual.rows.iter().enumerate() {
        scene.text(x + 14.0, y + 8.0 + 20.0 * i as f32, text, style.text);
    }
}

fn arrow(scene: &mut Scene, (x, y): Point, (u, v): Point, tip: Tip, color: Rgb) {
    let distance = (x - u).hypot(y - v);
    if tip == Tip::None || distance == 0.0 {
        return;
    }
    let (dx, dy) = ((x - u) / distance, (y - v) / distance);
    let (bx, by) = (x - dx * 10.0, y - dy * 10.0);
    match tip {
        Tip::None => {}
        Tip::Arrow | Tip::Triangle => {
            let hollow = tip == Tip::Triangle;
            image::polygon(
                scene,
                &[
                    (x, y),
                    (bx - dy * 5.0, by + dx * 5.0),
                    (bx + dy * 5.0, by - dx * 5.0),
                ],
                Some(if hollow { scene.paper() } else { color }),
                Some(color),
                1.2,
            );
        }
        Tip::Circle => {
            image::circle(
                scene,
                (x - dx * 4.5, y - dy * 4.5),
                4.5,
                Some(scene.paper()),
                Some(color),
                1.5,
            );
        }
        Tip::Cross => {
            let (x, y) = (x - dx * 5.0, y - dy * 5.0);
            image::line(scene, (x - 4.0, y - 4.0), (x + 4.0, y + 4.0), color, 1.7);
            image::line(scene, (x + 4.0, y - 4.0), (x - 4.0, y + 4.0), color, 1.7);
        }
    }
}

fn route(scene: &mut Scene, points: &[Point], edge: &Edge, color: Rgb) {
    let mut path = format!("M {} {}", points[0].0, points[0].1);
    for pair in points.windows(2) {
        let [(x, y), (u, v)] = [pair[0], pair[1]];
        if !edge.dotted {
            write!(path, " L {u} {v}").unwrap();
            continue;
        }
        let length = (u - x).hypot(v - y);
        if length == 0.0 {
            continue;
        }
        let (dx, dy) = ((u - x) / length, (v - y) / length);
        let mut start = 0.0;
        while start < length {
            let stop = (start + 4.0).min(length);
            write!(
                path,
                " M {} {} L {} {}",
                x + start * dx,
                y + start * dy,
                x + stop * dx,
                y + stop * dy
            )
            .unwrap();
            start += 8.0;
        }
    }
    scene.path(&path, None, Some(color), 1.5);
    let [head, tail] = [points[0], *points.last().unwrap()];
    if let Some(&near) = points.iter().find(|&&p| p != head) {
        arrow(scene, head, near, edge.ends.0, color);
    }
    if let Some(&near) = points.iter().rev().find(|&&p| p != tail) {
        arrow(scene, tail, near, edge.ends.1, color);
    }
}

pub(super) fn draw(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let diagram = parse(source)?;
    render(&diagram, options, scene)
}

fn render(d: &Diagram, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let headings: Vec<_> = d
        .headings
        .iter()
        .flat_map(|s| image::wrap(scene, s, 630.0))
        .collect();
    let axis = Axis {
        horizontal: !matches!(d.direction.as_str(), "TD" | "TB" | "BT"),
        top: MARGIN + 22.0 * headings.len() as f32,
    };
    let mut visuals: Vec<_> = d.nodes.iter().map(|n| Visual::new(scene, n)).collect();
    let relations: Vec<_> = d
        .edges
        .iter()
        .map(|edge| {
            let label = if edge.dotted {
                format!("«{}»", edge.label)
            } else {
                edge.label.clone()
            };
            let label = if label.is_empty() {
                Vec::new()
            } else {
                image::wrap(scene, &label, 180.0)
            };
            let width = label.iter().map(|s| scene.measure(s)).fold(0.0, f32::max);
            Relation {
                edge,
                from: d.ids[&edge.from],
                to: d.ids[&edge.to],
                label,
                width,
            }
        })
        .collect();
    let mut ownership = vec![0; d.nodes.len()];
    let groups = groups(d);
    for (i, (_, members)) in groups.iter().enumerate() {
        for &member in members {
            ownership[member] = i;
        }
    }
    let mut internal = vec![0.0; groups.len()];
    let mut rail_offsets = Vec::new();
    for relation in &relations {
        let (a, b) = (ownership[relation.from], ownership[relation.to]);
        rail_offsets.push(internal[a]);
        if a == b && groups[a].0.is_some() {
            internal[a] += if axis.horizontal {
                relation.width + 22.0
            } else {
                22.0 + 20.0 * relation.label.len() as f32
            };
        }
    }
    let mut panels = Vec::new();
    let mut main = 0.0;
    let mut across: f32 = 0.0;
    for ((boundary, members), rail_space) in groups.into_iter().zip(internal) {
        let mut depth = 0.0;
        let mut length: f32 = 0.0;
        for &i in &members {
            let (along, cross) = axis.swap(visuals[i].size);
            length = length.max(along);
            depth += cross + MEMBER_GAP;
        }
        if let Some(i) = boundary {
            let (title_along, title_across) = axis.swap(visuals[i].size);
            let title_pad = if axis.horizontal { title_across } else { 0.0 };
            depth = (depth - MEMBER_GAP).max(0.0) + title_pad + 30.0;
            length = (length + rail_space + 34.0).max(title_along);
            if !axis.horizontal {
                depth = depth.max(title_across);
                length += title_along;
            }
        }
        across = across.max(depth);
        panels.push(Panel {
            boundary,
            members,
            leading: main,
            trailing: main + length,
            depth,
        });
        main += length + GAP;
    }
    for panel in &panels {
        let start_across = (across - panel.depth) / 2.0;
        let mut ab = (panel.leading, start_across);
        if let Some(id) = panel.boundary {
            let title_height = visuals[id].size.1;
            visuals[id].bounds =
                axis.bounds(ab.0, ab.1, (panel.trailing - panel.leading, panel.depth));
            if axis.horizontal {
                ab.0 += 17.0;
                ab.1 += title_height + 6.0;
            } else {
                ab.0 += title_height + 6.0;
                ab.1 += 15.0;
            }
        }
        for &i in &panel.members {
            let size = axis.swap(visuals[i].size);
            visuals[i].bounds = axis.bounds(ab.0, ab.1, size);
            ab.1 += size.1 + MEMBER_GAP;
        }
    }

    let style = |id: &str| {
        if options.color {
            d.styles.resolve(id, options.theme_style())
        } else {
            Style::default()
        }
    };
    for panel in &panels {
        if let Some(id) = panel.boundary {
            boundary(scene, &d.nodes[id], &visuals[id], style(&d.nodes[id].id));
        }
    }

    let mut outer = across + 26.0;
    let mut captions = Vec::new();
    for (rel, offset) in relations.iter().zip(rail_offsets) {
        let (a, b) = (ownership[rel.from], ownership[rel.to]);
        let from = &visuals[rel.from];
        let to = &visuals[rel.to];
        let (from_node, to_node) = (&d.nodes[rel.from], &d.nodes[rel.to]);
        let start_port = |axis, side, offset| from.port(from_node, axis, side, offset);
        let end_port = |axis, side, offset| to.port(to_node, axis, side, offset);
        let edge_style = rel
            .edge
            .id
            .as_ref()
            .and_then(|id| {
                options
                    .color
                    .then(|| d.styles.resolve(id, Style::default()))
            })
            .unwrap_or_default();
        let stroke = edge_style
            .stroke
            .or_else(|| options.theme_line())
            .unwrap_or(scene.ink());
        let points = if a == b && panels[a].boundary.is_some() {
            let (start, mut end) = (start_port(axis, 1.0, 0.28), end_port(axis, 1.0, -0.28));
            if start == end {
                let (along, across) = axis.ab(end);
                end = axis.xy(along, across + 14.0);
            }
            let (u, v) = (axis.ab(start), axis.ab(end));
            let rail = u.0.max(v.0) + 12.0 + offset;
            let mut label_at = axis.xy(rail + 5.0, (u.1 + v.1) / 2.0);
            if !axis.horizontal {
                label_at.0 -= rel.width / 2.0;
            }
            captions.push((label_at, rel, edge_style));
            vec![start, axis.xy(rail, u.1), axis.xy(rail, v.1), end]
        } else if a.abs_diff(b) == 1 && rel.label.is_empty() {
            let sign = if a < b { 1.0 } else { -1.0 };
            let (start, end) = (start_port(axis, sign, 0.0), end_port(axis, -sign, 0.0));
            let from_exit = from.exit(from_node, axis, sign, start);
            let to_exit = to.exit(to_node, axis, -sign, end);
            let (_, q) = axis.ab(from_exit);
            let (_, s) = axis.ab(to_exit);
            let partition = &panels[a.min(b)];
            let next = &panels[a.max(b)];
            let mid = (partition.trailing + next.leading) / 2.0;
            vec![
                start,
                from_exit,
                axis.xy(mid, q),
                axis.xy(mid, s),
                to_exit,
                end,
            ]
        } else {
            let cross = axis.cross();
            let (start, mut end) = (start_port(cross, 1.0, 0.0), end_port(cross, 1.0, 0.0));
            if rel.from == rel.to {
                let (along, across) = axis.ab(end);
                end = axis.xy(along + 18.0, across - 4.0);
            }
            let from_exit = from.exit(from_node, cross, 1.0, start);
            let to_exit = to.exit(to_node, cross, 1.0, end);
            let (p, _) = axis.ab(from_exit);
            let (q, _) = axis.ab(to_exit);
            let mut label_at = axis.xy((p + q) / 2.0, outer);
            if axis.horizontal {
                label_at.0 -= rel.width / 2.0;
            } else {
                label_at.0 += 5.0;
            }
            if !rel.label.is_empty() {
                captions.push((label_at, rel, edge_style));
            }
            let points = vec![
                start,
                from_exit,
                axis.xy(p, outer),
                axis.xy(q, outer),
                to_exit,
                end,
            ];
            outer += if axis.horizontal {
                22.0 + 20.0 * rel.label.len() as f32
            } else {
                22.0 + rel.width
            };
            points
        };
        route(scene, &points, rel.edge, stroke);
    }
    for (node, visual) in d.nodes.iter().zip(&visuals) {
        if node.kind != Kind::Boundary {
            render_node(scene, node, visual, style(&node.id));
        }
    }
    let (w, h) = axis.xy(main - GAP, outer);
    let mut width = headings
        .iter()
        .map(|s| scene.measure(s) + 2.0 * MARGIN)
        .fold(w + MARGIN, f32::max);
    let mut height = h + MARGIN;
    for ((x, y), rel, style) in captions {
        if rel.label.is_empty() {
            continue;
        }
        let x = x.max(MARGIN);
        let top = y - 10.0 * rel.label.len() as f32;
        scene.rect(
            x - 4.0,
            top,
            rel.width + 8.0,
            20.0 * rel.label.len() as f32,
            3.0,
            style.fill.or(Some(scene.paper())),
            None,
        );
        for (i, line) in rel.label.iter().enumerate() {
            scene.text(x, top + 20.0 * i as f32, line, style.text);
        }
        width = width.max(x + rel.width + MARGIN);
        height = height.max(top + 20.0 * rel.label.len() as f32 + MARGIN);
    }
    for (i, row) in headings.iter().enumerate() {
        scene.text(MARGIN, MARGIN + 22.0 * i as f32, row, None);
    }
    scene.resize(width, height)
}
