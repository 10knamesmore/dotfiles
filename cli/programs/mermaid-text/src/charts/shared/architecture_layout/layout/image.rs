use std::fmt::Write;

use crate::color::{Rgb, Style};

use super::*;
use crate::charts::shared::architecture_layout::{Edge, End, LinkPaint, wires::Router};

const DX: f32 = 13.0;
const DY: f32 = 23.0;
const PAD: f32 = 16.0;

type Point = (f32, f32);

pub(super) fn columns(scene: &Scene, text: &str) -> usize {
    (scene.measure(text) / DX).ceil() as usize
}

pub(crate) fn image(
    diagram: &Diagram,
    options: &Options,
    palette: &Palette,
    scene: &mut Scene,
) -> Result<(), String> {
    let frame = arrange(diagram, options, Some(scene))?;
    let headings: Vec<_> = diagram
        .heading
        .iter()
        .flat_map(|text| wrap(text, options.width.clamp(32, 96)))
        .collect();
    let heading_width = measure(scene, &headings);
    let heading_height = headings.len() as f32 * DY;
    let width = (frame.rect.w as f32 * DX).max(heading_width) + 2.0 * PAD;
    let height = frame.rect.h as f32 * DY + heading_height + 2.0 * PAD;
    scene.resize(width, height)?;

    let point =
        |(x, y): (usize, usize)| (PAD + x as f32 * DX, PAD + heading_height + y as f32 * DY);
    let mut boxes = vec![Rect::default(); diagram.nodes.len()];
    let mut router = Router::new(frame.rect.w, frame.rect.h);
    let mut image = DiagramImage {
        scene,
        diagram,
        palette,
        point,
        router: &mut router,
        boxes: &mut boxes,
    };
    image.frame(&frame, (0, 0));
    for (i, text) in headings.iter().enumerate() {
        image.scene.text(PAD, PAD + i as f32 * DY, text, None);
    }
    let mut remaining = Vec::new();
    for (i, edge) in diagram.edges.iter().enumerate() {
        if !image.relationship(edge, i + 1)? {
            remaining.push((i + 1, edge));
        }
    }
    let mut size = (width, height);
    for (n, edge) in remaining {
        let panel = RelationshipPanel::new(diagram, n, edge, palette, image.scene)?;
        size.0 = size.0.max(panel.width + 2.0 * PAD);
        size.1 += panel.height;
        image.scene.resize(size.0, size.1)?;
        panel.paint(image.scene, size.1 - panel.height);
    }
    Ok(())
}

struct DiagramImage<'a, F> {
    scene: &'a mut Scene,
    diagram: &'a Diagram,
    palette: &'a Palette,
    point: F,
    router: &'a mut Router,
    boxes: &'a mut [Rect],
}

impl<F: Fn((usize, usize)) -> Point> DiagramImage<'_, F> {
    fn frame(&mut self, frame: &Frame, (x, y): (usize, usize)) {
        let rect = Rect {
            x: x + frame.rect.x,
            y: y + frame.rect.y,
            ..frame.rect
        };
        if let Some(i) = frame.node {
            self.boxes[i] = rect;
            let node = &self.diagram.nodes[i];
            let style = self.palette.element(node);
            let align_right = node.kind == "Node_R";
            self.router.frame(
                rect,
                node.boundary,
                align_right,
                frame.text.iter().map(|s| columns(self.scene, s)),
            );
            let (l, t) = (self.point)((rect.x, rect.y));
            let (r, b) = (self.point)((rect.x + rect.w - 1, rect.y + rect.h - 1));
            let ink = style.stroke.unwrap_or_else(|| self.scene.ink());
            if node.boundary {
                self.scene.rect(l, t, r - l, b - t, 0.0, style.fill, None);
                draw_border(self.scene, [(l, t), (r, t), (r, b), (l, b), (l, t)], ink);
            } else {
                self.scene
                    .rect(l, t, r - l, b - t, 5.0, style.fill, Some(ink));
            }
            if !align_right {
                draw_icon(self.scene, node, (l + DX, t + DY), ink);
            }
            for (i, text) in frame.text.iter().enumerate() {
                let x = if align_right {
                    r - DX - self.scene.measure(text)
                } else {
                    l + 2.0 * DX
                };
                self.scene
                    .text(x, t + (i as f32 + 0.5) * DY, text, style.text);
            }
        }
        for child in &frame.children {
            self.frame(child, (rect.x, rect.y));
        }
    }

    fn relationship(&mut self, edge: &Edge, n: usize) -> Result<bool, String> {
        let Some(route) = self.router.route(self.diagram, edge, self.boxes)? else {
            return Ok(false);
        };
        let arrows = [edge.from.arrow, edge.to.arrow];
        self.router.reserve(&route, arrows);
        let pixel = |p| (self.point)(self.router.coordinates(p));
        let points: Vec<_> = [route.pins[0]]
            .into_iter()
            .chain(route.path.iter().copied())
            .chain([route.pins[1]])
            .map(pixel)
            .collect();
        let paint = self.palette.relationship(edge);
        let ink = paint.line.fg.unwrap_or_else(|| self.scene.ink());
        let arrow_color = paint.arrow.fg.unwrap_or(ink);
        draw_wire(self.scene, &points, ink);
        for ((pin, side), arrow) in route.pins.into_iter().zip(route.sides).zip(arrows) {
            if arrow {
                draw_arrow(self.scene, pixel(pin), side, arrow_color);
            }
        }
        let member = match (edge.from.group, edge.to.group) {
            (true, true) => format!("from {}, to {}", edge.from.id, edge.to.id),
            (true, false) => format!("via {}", edge.from.id),
            (false, true) => format!("via {}", edge.to.id),
            (false, false) => String::new(),
        };
        let caption = match (edge.label.trim().is_empty(), member.is_empty()) {
            (true, true) => return Ok(true),
            (true, false) => member,
            (false, true) => edge.label.clone(),
            (false, false) => format!("{}\n{member}", edge.label),
        };
        if self.caption(&route.path, &caption, paint.text.fg) {
            return Ok(true);
        }
        self.caption(&route.path, &format!("[{n}]"), paint.text.fg);
        Ok(false)
    }

    fn caption(&mut self, path: &[usize], label: &str, color: Option<Rgb>) -> bool {
        for limit in [44, 24, 16, 10] {
            let lines = wrap(label, limit);
            let w = lines
                .iter()
                .map(|text| columns(self.scene, text))
                .max()
                .unwrap_or(0);
            if let Some(coords) = self.router.label(path, w, lines.len()) {
                let (x, y) = (self.point)(coords);
                for (i, line) in lines.iter().enumerate() {
                    self.scene.text(x, y + (i as f32 - 0.5) * DY, line, color);
                }
                return true;
            }
        }
        false
    }
}

fn draw_icon(scene: &mut Scene, node: &Node, (x, y): Point, ink: Rgb) {
    let kind = node.kind.strip_suffix("_Ext").unwrap_or(&node.kind);
    let icon = node.icon.to_ascii_lowercase();
    let path = match (icon.as_str(), kind) {
        (_, "junction") => "m0 -7 7 7 -7 7 -7 -7z",
        ("cloud", _) => "m-6 5a4 4 0 0 1 -0.5 -8 6 6 0 0 1 11.5 1.5 3.5 3.5 0 0 1 -0.5 7z",
        ("server", _) => "m-7 -6h14v12h-14zm0 6h14m-11 -3h1m-1 6h1",
        ("database", _) | (_, "ContainerDb" | "SystemDb" | "ComponentDb") => {
            "m-7 -4a7 3 0 1 1 14 0v8a7 3 0 1 1 -14 0zm0 0a7 3 0 0 0 14 0m-14 4a7 3 0 0 0 14 0"
        }
        ("disk", _) => "m-7 0a7 7 0 1 0 14 0 7 7 0 1 0 -14 0m5 0a2 2 0 1 0 4 0 2 2 0 1 0 -4 0",
        ("internet", _) => {
            "m-7 0a7 7 0 1 0 14 0 7 7 0 1 0 -14 0h14m-7 -7c-5 4 -5 10 0 14 5 -4 5 -10 0 -14"
        }
        (_, "Person") => "m-3 -4a3 3 0 1 0 6 0 3 3 0 1 0 -6 0m-3 11v-3q6 -7 12 0v3z",
        (_, "ContainerQueue" | "SystemQueue" | "ComponentQueue") => {
            "m-7 -4h14v8h-14zm4 0v8m4 -8v8m4 -8v8"
        }
        _ => return,
    };
    scene.path(&format!("M{x} {y}{path}"), None, Some(ink), 1.3);
}

fn draw_border(scene: &mut Scene, points: [Point; 5], color: Rgb) {
    let mut path = String::new();
    for pair in points.windows(2) {
        let [(x, y), (x1, y1)] = [pair[0], pair[1]];
        let len = (x - x1).abs() + (y - y1).abs();
        let mut offset = 0.0_f32;
        while offset < len {
            let a = offset / len;
            let b = (offset + 7.0).min(len) / len;
            let (u, v) = (x + (x1 - x) * a, y + (y1 - y) * a);
            let (w, z) = (x + (x1 - x) * b, y + (y1 - y) * b);
            let _ = write!(path, "M{u} {v}L{w} {z}");
            offset += 12.0;
        }
    }
    scene.path(&path, None, Some(color), 1.4);
}

fn draw_wire(scene: &mut Scene, points: &[Point], color: Rgb) {
    let mut path = format!("M{} {}", points[0].0, points[0].1);
    for pair in points.windows(3) {
        let [a, (x, y), b] = [pair[0], pair[1], pair[2]];
        if !(a.0 == x && x == b.0 || a.1 == y && y == b.1) {
            let _ = write!(path, "L{x} {y}");
        }
    }
    let (x, y) = points[points.len() - 1];
    let _ = write!(path, "L{x} {y}");
    scene.path(&path, None, Some(color), 1.65);
}

fn draw_arrow(scene: &mut Scene, (x, y): Point, outside: Side, color: Rgb) {
    let (dx, dy) = match outside {
        Side::Top => (0.0, -1.0),
        Side::Right => (1.0, 0.0),
        Side::Bottom => (0.0, 1.0),
        Side::Left => (-1.0, 0.0),
    };
    let (a, b) = (x + dx * 9.0, y + dy * 9.0);
    let (u, v) = (-dy * 4.5, dx * 4.5);
    scene.path(
        &format!("M{x} {y}L{} {}L{} {}Z", a + u, b + v, a - u, b - v),
        Some(color),
        None,
        0.0,
    );
}

struct RelationshipPanel {
    width: f32,
    height: f32,
    left: Reference,
    right: Reference,
    label: Vec<String>,
    paint: LinkPaint,
}

struct Reference {
    text: Vec<String>,
    style: Style,
    width: f32,
    arrow: bool,
}

impl RelationshipPanel {
    fn new(
        diagram: &Diagram,
        n: usize,
        edge: &Edge,
        palette: &Palette,
        scene: &Scene,
    ) -> Result<Self, String> {
        let endpoint = |end: &End| {
            let node = &diagram.nodes[diagram.endpoint(end)?];
            let mut label = node.id.clone();
            if end.group {
                let _ = write!(label, " / {}", end.id);
            }
            if let Some(side) = end.side {
                let _ = write!(label, " :{}", side.letter());
            }
            let text = wrap(&label, 34);
            Ok::<_, String>(Reference {
                width: measure(scene, &text) + 2.0 * PAD,
                text,
                style: palette.element(node),
                arrow: end.arrow,
            })
        };
        let left = endpoint(&edge.from)?;
        let right = endpoint(&edge.to)?;
        let label = wrap(&format!("[{n}] {}", edge.label), 70);
        Ok(Self {
            width: (left.width + 4.0 * PAD + right.width).max(measure(scene, &label)),
            height: ((left.text.len().max(right.text.len()) + label.len()) as f32 * DY) + 2.0 * PAD,
            left,
            right,
            label,
            paint: palette.relationship(edge),
        })
    }

    fn paint(&self, scene: &mut Scene, y: f32) {
        for (i, text) in self.label.iter().enumerate() {
            scene.text(PAD, y + i as f32 * DY, text, self.paint.text.fg);
        }
        let top = y + self.label.len() as f32 * DY + PAD / 2.0;
        let height = self.left.text.len().max(self.right.text.len()) as f32 * DY + PAD;
        let center = top + height / 2.0;
        let a = PAD + self.left.width;
        let b = PAD + self.width - self.right.width;
        let color = self.paint.line.fg.unwrap_or_else(|| scene.ink());
        for (x, reference) in [(PAD, &self.left), (b, &self.right)] {
            let stroke = Some(reference.style.stroke.unwrap_or_else(|| scene.ink()));
            scene.rect(
                x,
                top,
                reference.width,
                height,
                5.0,
                reference.style.fill,
                stroke,
            );
            for (i, text) in reference.text.iter().enumerate() {
                scene.text(
                    x + PAD,
                    center + (i as f32 - reference.text.len() as f32 / 2.0) * DY,
                    text,
                    reference.style.text,
                );
            }
        }
        draw_wire(scene, &[(a, center), (b, center)], color);
        let arrow = self.paint.arrow.fg.unwrap_or(color);
        if self.left.arrow {
            draw_arrow(scene, (a, center), Side::Right, arrow);
        }
        if self.right.arrow {
            draw_arrow(scene, (b, center), Side::Left, arrow);
        }
    }
}

fn measure(scene: &Scene, lines: &[String]) -> f32 {
    lines
        .iter()
        .map(|s| scene.measure(s))
        .fold(0.0_f32, f32::max)
}
