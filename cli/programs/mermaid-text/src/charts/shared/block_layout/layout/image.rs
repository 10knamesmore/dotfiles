use std::fmt::Write;

use crate::{color::Rgb, graphics::Scene};

use super::*;

const DX: f32 = 11.0;
const DY: f32 = 22.0;
const MARGIN: f32 = 16.0;

type Point = (f32, f32);

pub(super) fn columns(scene: &Scene, text: &str) -> usize {
    (scene.measure(text) / DX).ceil() as usize
}

pub(super) fn fit_shape(block: &mut Block, shape: Shape, text: usize) {
    match shape {
        Shape::Diamond => {
            block.w = block.w.max(2 * text + 3);
            block.h = block.h.max(2 * block.header.len() + 2);
        }
        Shape::Hexagon => {
            block.w += text / 2 + 2;
            block.h += 2;
        }
        Shape::Circle | Shape::DoubleCircle => {
            let inner_width = text as f32 * DX + 20.0;
            let inner_height = block.header.len() as f32 * DY + 14.0;
            let diameter = inner_width.hypot(inner_height).max(40.0);
            block.w = block.w.max((diameter / DX).ceil() as usize + 1);
            block.h = block.h.max((diameter / DY).ceil() as usize + 1);
        }
        Shape::Slanted(..) | Shape::Flag | Shape::Arrows(_) => {
            block.w += 2;
            block.h += 1;
        }
        _ => {}
    }
}

pub fn image(
    diagram: &Diagram,
    grid: &Grid,
    options: &Options,
    theme: Option<DiagramColors>,
    scene: &mut Scene,
) -> Result<(), String> {
    if grid.slots.is_empty() {
        return Err("block diagram has no blocks".into());
    }
    if diagram.edges.len() > 4096 {
        return Err("diagram exceeds 4096 connections".into());
    }
    let mut degrees = vec![0; diagram.nodes.len()];
    for edge in &diagram.edges {
        degrees[edge.from] += 1;
        degrees[edge.to] += 1;
    }
    let labels: Vec<_> = diagram
        .edges
        .iter()
        .map(|edge| {
            let lines = wrap(&edge.label, options.width.clamp(18, 36));
            let width = lines.iter().map(|s| columns(scene, s)).max().unwrap_or(0);
            (lines, width)
        })
        .collect();
    let label_width = labels.iter().map(|(_, width)| *width).max().unwrap_or(0);
    let label_height = labels
        .iter()
        .map(|(lines, _)| lines.len())
        .max()
        .unwrap_or(0);
    let captions: Vec<_> = diagram
        .caption
        .iter()
        .flat_map(|text| wrap(text, options.width.clamp(20, 96)))
        .collect();
    let caption_height = captions.len() as f32 * DY;
    let caption_width = captions
        .iter()
        .map(|s| scene.measure(s))
        .fold(0.0, f32::max);
    let theme = theme.unwrap_or_default();
    let mut order: Vec<_> = (0..diagram.edges.len()).collect();
    let connected = !order.is_empty();
    let mut work = 24_000_000;
    for extra in [0, 2, 4, 8, 16, 32] {
        let mut measure = Measure {
            diagram,
            blocks: (0..diagram.nodes.len()).map(|_| Block::default()).collect(),
            degrees: degrees.clone(),
            dx: if connected {
                label_width.max(2) + 2 + extra
            } else {
                2
            },
            dy: if connected {
                label_height.max(1) + 2 + extra
            } else {
                1
            },
            ascii: false,
            scene: Some(scene),
        };
        let layout = measure.grid(grid, options.width.clamp(36, 120))?;
        let blocks = measure.blocks;
        let margin = if connected {
            (degrees.iter().copied().max().unwrap_or(0) + 2).clamp(3, 48) + extra
        } else {
            1
        };
        let width = layout.width() + margin * 2;
        let height = layout.height() + margin * 2;
        let mut rects = vec![Rect::default(); diagram.nodes.len()];
        layout.place(&blocks, &mut rects, margin, margin, layout.width());
        let mut router = if connected {
            let mut router = wires::Router::new(width, height)?;
            router.routes = Some(Vec::new());
            for (i, block) in blocks.iter().enumerate().filter(|(i, _)| rects[*i].w > 0) {
                router.obstacle(rects[i], block.children.is_some(), block.header.len());
            }
            let mut failed = None;
            for (position, &i) in order.iter().enumerate() {
                let (lines, width) = &labels[i];
                if !router.connect(
                    &diagram.edges[i],
                    &rects,
                    &diagram.nodes,
                    (lines, *width),
                    false,
                    &mut work,
                )? {
                    failed = Some(position);
                    break;
                }
            }
            if let Some(position) = failed {
                order[..=position].rotate_right(1);
                continue;
            }
            Some(router)
        } else {
            None
        };

        let (mut left, mut top, mut right, mut bottom) =
            (margin, margin, width - margin, height - margin);
        if let Some(router) = &router {
            if let Some(routes) = &router.routes {
                for &(x, y) in routes.iter().flatten() {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x + 1);
                    bottom = bottom.max(y + 1);
                }
            }
            for (x, y, text) in &router.labels {
                left = left.min(*x);
                top = top.min(*y);
                right = right.max(x + columns(scene, text));
                bottom = bottom.max(y + 1);
            }
        }
        scene.resize(
            ((right - left) as f32 * DX).max(caption_width) + 2.0 * MARGIN,
            (bottom - top) as f32 * DY + caption_height + 2.0 * MARGIN,
        )?;
        let point = |(x, y): (usize, usize)| {
            (
                MARGIN + (x as f32 - left as f32) * DX,
                MARGIN + caption_height + (y as f32 - top as f32) * DY,
            )
        };
        for (i, caption) in captions.iter().enumerate() {
            scene.text(MARGIN, MARGIN + i as f32 * DY, caption, None);
        }
        let frames: Vec<_> = rects
            .iter()
            .zip(&diagram.nodes)
            .map(|(&r, node)| Frame {
                start: point((r.x, r.y)),
                end: if r.w == 0 {
                    (0.0, 0.0)
                } else {
                    point((r.right(), r.bottom()))
                },
                shape: node_shape(node),
            })
            .collect();
        paint_grid(scene, diagram, &layout, &blocks, &frames, theme.nodes);
        if let Some(router) = router.as_mut() {
            let ink = theme.line.fg.unwrap_or_else(|| scene.ink());
            for (&i, path) in order.iter().zip(router.routes.take().into_iter().flatten()) {
                let edge = &diagram.edges[i];
                let mut points: Vec<_> = path.into_iter().map(point).collect();
                let last = points.len() - 1;
                points[0] = frames[edge.from].anchor(points[0], points[1]);
                points[last] = frames[edge.to].anchor(points[last], points[last - 1]);
                draw_link(scene, &points, edge, ink);
            }
            for (x, y, label) in &router.labels {
                let (x, y) = point((*x, *y));
                let y = y - DY / 2.0;
                if let Some(bg) = theme.label.bg {
                    scene.rect(
                        x - 3.0,
                        y,
                        scene.measure(label) + 6.0,
                        DY,
                        2.0,
                        Some(bg),
                        None,
                    );
                }
                scene.text(x, y, label, theme.label.fg);
            }
        }
        return Ok(());
    }
    Err("connections exceed the available PNG routing lanes after six grid expansions".into())
}

fn paint_grid(
    scene: &mut Scene,
    diagram: &Diagram,
    layout: &Layout,
    blocks: &[Block],
    frames: &[Frame],
    theme: Style,
) {
    for slot in &layout.slots {
        let i = slot.node;
        let block = &blocks[i];
        let style = theme.overlay(diagram.nodes[i].style);
        let frame = &frames[i];
        frame.paint(scene, style);
        let (left, top) = frame.start;
        let (right, bottom) = frame.end;
        let mut y = if block.children.is_some() || !block.body.is_empty() {
            top + DY / 2.0
        } else {
            (top + bottom - block.header.len() as f32 * DY) / 2.0
                + if matches!(frame.shape, Shape::Cylinder) {
                    5.0
                } else {
                    0.0
                }
        };
        for line in &block.header {
            let x = (left + right - scene.measure(line)) / 2.0;
            scene.text(x, y, line, style.text);
            y += DY;
        }
        if !block.body.is_empty() {
            let ink = style.stroke.unwrap_or_else(|| scene.ink());
            scene.path(&format!("M{left} {y}H{right}"), None, Some(ink), 1.25);
            for line in &block.body {
                scene.text(left + DX, y, line, style.text);
                y += DY;
            }
        }
        if let Some(children) = &block.children {
            paint_grid(scene, diagram, children, blocks, frames, theme);
        }
    }
}

struct Frame {
    start: Point,
    end: Point,
    shape: Shape,
}

impl Frame {
    fn oval(&self) -> (Point, f32) {
        let (l, t) = self.start;
        let (r, b) = self.end;
        (((l + r) / 2.0, (t + b) / 2.0), (r - l).min(b - t) / 2.0)
    }

    fn vertices(&self) -> Option<Vec<Point>> {
        let (l, t) = self.start;
        let (r, b) = self.end;
        let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
        let inset = (r - l).min(b - t) * 0.22;
        Some(match self.shape {
            Shape::Diamond => vec![(cx, t), (r, cy), (cx, b), (l, cy)],
            Shape::Hexagon => vec![
                (l + inset, t),
                (r - inset, t),
                (r, cy),
                (r - inset, b),
                (l + inset, b),
                (l, cy),
            ],
            Shape::Flag => vec![(l, t), (r, t), (r, b), (l, b), (l + inset, cy)],
            Shape::Slanted(a, z) => {
                let side = |x, ch| match ch {
                    '/' => (x + inset, x - inset),
                    '\\' => (x - inset, x + inset),
                    _ => (x, x),
                };
                let center = |x, slope, sign| {
                    if slope == '|' { x } else { x + sign * inset }
                };
                let (tl, bl) = side(center(l, a, 1.0), a);
                let (tr, br) = side(center(r, z, -1.0), z);
                vec![(tl, t), (tr, t), (br, b), (bl, b)]
            }
            Shape::Arrows(mask) if mask != 0 => {
                let ends = [
                    [(l, t), (r, t)],
                    [(r, t), (r, b)],
                    [(r, b), (l, b)],
                    [(l, b), (l, t)],
                ];
                let sides = [
                    (NORTH, (0.0, inset)),
                    (EAST, (-inset, 0.0)),
                    (SOUTH, (0.0, -inset)),
                    (WEST, (inset, 0.0)),
                ];
                let mut points = Vec::new();
                let inner_l = l + if mask & WEST != 0 { inset } else { 0.0 };
                let inner_r = r - if mask & EAST != 0 { inset } else { 0.0 };
                let inner_t = t + if mask & NORTH != 0 { inset } else { 0.0 };
                let inner_b = b - if mask & SOUTH != 0 { inset } else { 0.0 };
                let corners = [
                    (inner_l, inner_t),
                    (inner_r, inner_t),
                    (inner_r, inner_b),
                    (inner_l, inner_b),
                ];
                for (i, (bit, delta)) in sides.into_iter().enumerate() {
                    points.push(corners[i]);
                    if mask & bit != 0 {
                        let [a, b] = ends[i];
                        let center = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                        let scale = inset / ((b.0 - a.0).abs() + (b.1 - a.1).abs());
                        let spread = ((b.0 - a.0) * scale, (b.1 - a.1) * scale);
                        points.push((center.0 + delta.0 - spread.0, center.1 + delta.1 - spread.1));
                        points.push(center);
                        points.push((center.0 + delta.0 + spread.0, center.1 + delta.1 + spread.1));
                    }
                }
                points
            }
            _ => return None,
        })
    }

    fn paint(&self, scene: &mut Scene, style: Style) {
        let ((l, t), (r, b)) = (self.start, self.end);
        let border = Some(style.stroke.unwrap_or_else(|| scene.ink()));
        let fill = style.fill;
        if let Some(points) = self.vertices() {
            let mut path = polyline(&points);
            path.push('Z');
            scene.path(&path, fill, border, 1.5);
            return;
        }
        match self.shape {
            Shape::Circle | Shape::DoubleCircle => {
                let (center, radius) = self.oval();
                circle(scene, center, radius, fill, border);
                if matches!(self.shape, Shape::DoubleCircle) {
                    circle(scene, center, radius - 4.0, None, border);
                }
            }
            Shape::Cylinder => {
                let rx = (r - l) / 2.0;
                let ry = 9.0;
                let top = t + ry;
                let bottom = b - ry;
                scene.path(
                    &format!("M{l} {top}A{rx} {ry} 0 0 1 {r} {top}V{bottom}A{rx} {ry} 0 0 1 {l} {bottom}Z"),
                    fill, border, 1.5,
                );
                scene.path(
                    &format!("M{l} {top}A{rx} {ry} 0 0 0 {r} {top}"),
                    None,
                    border,
                    1.5,
                );
            }
            shape => {
                let radius = match shape {
                    Shape::Round => 8.0,
                    Shape::Stadium => (b - t) / 2.0,
                    _ => 0.0,
                };
                scene.rect(l, t, r - l, b - t, radius, fill, border);
                if matches!(shape, Shape::Routine) {
                    let (x, z) = (l + DX, r - DX);
                    scene.path(&format!("M{x} {t}V{b}M{z} {t}V{b}"), None, border, 1.5);
                }
            }
        }
    }

    fn anchor(&self, at: Point, outer: Point) -> Point {
        let (x, y) = at;
        let ((l, t), (r, b)) = (self.start, self.end);
        let vertical = x == outer.0;
        let near = if vertical { outer.1 < y } else { outer.0 < x };
        if let Some(vertices) = self.vertices() {
            let cross = |(x, y)| if vertical { (x, y) } else { (y, x) };
            let (along, _) = cross(at);
            let mut boundary = None::<f32>;
            for (a, b) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
                let ((a, u), (b, v)) = (cross(*a), cross(*b));
                if a != b && a.min(b) <= along && along <= a.max(b) {
                    let hit = u + (v - u) * (along - a) / (b - a);
                    boundary = Some(
                        boundary.map_or(hit, |old| if near { old.min(hit) } else { old.max(hit) }),
                    );
                }
            }
            if let Some(boundary) = boundary {
                return cross((along, boundary));
            }
        }
        if matches!(self.shape, Shape::Circle | Shape::DoubleCircle) {
            let ((cx, cy), radius) = self.oval();
            let sign = if near { -1.0 } else { 1.0 };
            let delta = if vertical { x - cx } else { y - cy }.clamp(-radius * 0.95, radius * 0.95);
            let run = (radius * radius - delta * delta).sqrt() * sign;
            return if vertical {
                (cx + delta, cy + run)
            } else {
                (cx + run, cy + delta)
            };
        }
        if matches!(self.shape, Shape::Cylinder) && vertical {
            let dx = ((2.0 * x - l - r) / (r - l)).clamp(-1.0, 1.0);
            let inset = 9.0 * (1.0 - (1.0 - dx * dx).sqrt());
            return (x, if near { t + inset } else { b - inset });
        }
        let radius = match self.shape {
            Shape::Round => 8.0,
            Shape::Stadium => ((b - t).min(r - l)) / 2.0,
            _ => 0.0,
        };
        let distance = if vertical {
            (x - l).min(r - x)
        } else {
            (y - t).min(b - y)
        };
        if distance < radius {
            let inset = radius - (radius * radius - (radius - distance).powi(2)).sqrt();
            return if vertical {
                (x, y + if near { inset } else { -inset })
            } else {
                (x + if near { inset } else { -inset }, y)
            };
        }
        at
    }
}

fn circle(scene: &mut Scene, (x, y): Point, r: f32, fill: Option<Rgb>, stroke: Option<Rgb>) {
    let l = x - r;
    let rt = x + r;
    scene.path(
        &format!("M{l} {y}A{r} {r} 0 1 0 {rt} {y}A{r} {r} 0 1 0 {l} {y}Z"),
        fill,
        stroke,
        1.5,
    );
}

fn polyline(points: &[Point]) -> String {
    let mut path = format!("M{} {}", points[0].0, points[0].1);
    for (x, y) in &points[1..] {
        let _ = write!(path, "L{x} {y}");
    }
    path
}

fn draw_link(scene: &mut Scene, points: &[Point], edge: &Edge, color: Rgb) {
    let mut bends = vec![points[0]];
    for pair in points.windows(3) {
        let [a, b, c] = [pair[0], pair[1], pair[2]];
        if !(a.0 == b.0 && b.0 == c.0 || a.1 == b.1 && b.1 == c.1) {
            bends.push(b);
        }
    }
    bends.push(points[points.len() - 1]);
    let path = if matches!(edge.line, Line::Dotted) {
        let mut dashes = String::new();
        for pair in bends.windows(2) {
            let [(x, y), (z, w)] = [pair[0], pair[1]];
            let len = (z - x).hypot(w - y);
            let mut dist = 0.0_f32;
            while dist < len {
                let end = (dist + 3.0).min(len);
                let (a, b) = (x + (z - x) * dist / len, y + (w - y) * dist / len);
                let (c, d) = (x + (z - x) * end / len, y + (w - y) * end / len);
                let _ = write!(dashes, "M{a} {b}L{c} {d}");
                dist += 7.0;
            }
        }
        dashes
    } else {
        polyline(&bends)
    };
    scene.path(
        &path,
        None,
        Some(color),
        if matches!(edge.line, Line::Thick) {
            3.8
        } else {
            1.6
        },
    );
    for (tip, (x, y), (a, b)) in [
        (edge.start, points[0], points[1]),
        (edge.end, points[points.len() - 1], points[points.len() - 2]),
    ] {
        let length = (a - x).hypot(b - y);
        if length == 0.0 {
            continue;
        }
        let (dx, dy) = ((a - x) / length, (b - y) / length);
        let point = |along, across| (x + dx * along - dy * across, y + dy * along + dx * across);
        match tip {
            Tip::None => {}
            Tip::Arrow => {
                let mut path = polyline(&[(x, y), point(9.0, 4.4), point(9.0, -4.4)]);
                path.push('Z');
                scene.path(&path, Some(color), None, 0.0);
            }
            Tip::Circle => circle(
                scene,
                point(5.0, 0.0),
                3.5,
                Some(scene.paper()),
                Some(color),
            ),
            Tip::Cross => {
                let path = [
                    polyline(&[point(2.0, -4.0), point(10.0, 4.0)]),
                    polyline(&[point(2.0, 4.0), point(10.0, -4.0)]),
                ]
                .concat();
                scene.path(&path, None, Some(color), 1.6);
            }
        }
    }
}
