use std::fmt::Write;

use super::{
    Diagram, Edge, Family, Line, Tip,
    layout::{Block, CELL_HEIGHT, CELL_WIDTH, Plan, Rect},
    routes::{Point as Cell, Routes},
};
use crate::{
    Options,
    charts::shared::plot::image::{self, Point},
    color::{Rgb, Style},
    graphics::Scene,
};

const MARGIN: f32 = 16.0;

struct Page {
    top: f32,
    width: f32,
    height: f32,
}

impl Page {
    fn cell(&self, Cell(x, y): Cell) -> Point {
        (
            MARGIN + (x as f32 + 0.5) * CELL_WIDTH,
            self.top + (y as f32 + 0.5) * CELL_HEIGHT,
        )
    }

    fn bounds(&self, r: Rect) -> [f32; 4] {
        [
            MARGIN + r.x as f32 * CELL_WIDTH,
            self.top + r.y as f32 * CELL_HEIGHT,
            r.w as f32 * CELL_WIDTH,
            r.h as f32 * CELL_HEIGHT,
        ]
    }

    fn port(&self, r: Rect, direction: usize) -> Point {
        let [x, y, w, h] = self.bounds(r);
        match direction {
            0 => (x + w, y + h / 2.0),
            1 => (x + w / 2.0, y + h),
            2 => (x, y + h / 2.0),
            _ => (x + w / 2.0, y),
        }
    }
}

fn item_style(d: &Diagram, id: usize, frame: bool, options: &Options, scene: &Scene) -> Style {
    let mut style = options.theme_style();
    if frame {
        style.fill = image::theme(options, "clusterBkg").or_else(|| {
            let accent = if options.color {
                image::palette(id, true)
            } else {
                scene.ink()
            };
            Some(image::fade(accent, 0.07))
        });
        style.stroke = image::theme(options, "clusterBorder").or(style.stroke);
        style.text = image::theme(options, "titleColor");
    }
    if options.color {
        style = style.overlay(d.items[id].style);
    }
    if style.text.is_none() {
        style.text = style.fill.map(|fill| image::contrast(fill, scene.paper()));
    }
    style
}

fn frame(scene: &mut Scene, page: &Page, b: &Block, style: Style, family: Family) {
    let [x, y, w, h] = page.bounds(b.rect);
    let stroke = style
        .stroke
        .unwrap_or_else(|| image::fade(scene.ink(), 0.55));
    scene.rect(x, y, w, h, 10.0, style.fill, Some(stroke));
    if family == Family::Swimlane {
        let line_y = y + (b.lines.len() as f32 + 1.0) * CELL_HEIGHT;
        image::line(scene, (x, line_y), (x + w, line_y), stroke, 1.0);
    }
    for (i, text) in b.lines.iter().enumerate() {
        scene.text(
            x + 16.0,
            y + 10.0 + i as f32 * CELL_HEIGHT,
            text,
            style.text,
        );
    }
}

fn card(scene: &mut Scene, page: &Page, b: &Block, style: Style, shape: &str) {
    let [x, y, w, h] = page.bounds(b.rect);
    let (right, bottom) = (x + w, y + h);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let fill = Some(style.fill.unwrap_or(scene.paper()));
    let ink = style.stroke.unwrap_or(scene.ink());
    let stroke = Some(ink);
    match shape {
        "decision" | "diamond" => image::polygon(
            scene,
            &[(cx, y), (right, cy), (cx, bottom), (x, cy)],
            fill,
            stroke,
            1.8,
        ),
        "circle" | "double-circle" | "connector" => {
            image::circle(scene, (cx, cy), h / 2.0, fill, stroke, 1.8);
            if shape == "double-circle" {
                image::circle(scene, (cx, cy), h / 2.0 - 5.0, None, stroke, 1.2);
            }
        }
        "refdoc" | "document" => {
            let fold = 13.0;
            let inset = right - fold;
            scene.path(
                &format!(
                    "M {x} {y} H {inset} L {right} {} V {bottom} H {x} Z",
                    y + fold
                ),
                fill,
                stroke,
                1.8,
            );
            scene.path(
                &format!("M {inset} {y} v {fold} h {fold}"),
                None,
                stroke,
                1.2,
            );
        }
        "database" => {
            let rim = 8.0;
            let tuck = y + rim;
            let base = bottom - rim;
            scene.path(
                &format!("M {x} {tuck} A {} {rim} 0 0 1 {right} {tuck} V {base} A {} {rim} 0 0 1 {x} {base} Z M {x} {tuck} A {} {rim} 0 0 0 {right} {tuck}", w / 2.0, w / 2.0, w / 2.0),
                fill,
                stroke,
                1.8,
            );
        }
        "hexagon" => {
            let bevel = 12.0;
            image::polygon(
                scene,
                &[
                    (x + bevel, y),
                    (right - bevel, y),
                    (right, cy),
                    (right - bevel, bottom),
                    (x + bevel, bottom),
                    (x, cy),
                ],
                fill,
                stroke,
                1.8,
            );
        }
        "input" => image::polygon(
            scene,
            &[
                (x + 12.0, y),
                (right + 12.0, y),
                (right - 12.0, bottom),
                (x - 12.0, bottom),
            ],
            fill,
            stroke,
            1.8,
        ),
        _ => {
            let radius = match shape {
                "stadium" | "bpmn-start" | "bpmn-end" => h / 2.0,
                "task" | "rounded" | "tool" => 10.0,
                _ => 4.0,
            };
            scene.rect(x, y, w, h, radius, fill, stroke);
            if shape == "tool" {
                image::line(scene, (x + 8.0, y + 1.0), (x + 8.0, bottom - 1.0), ink, 1.2);
                image::line(
                    scene,
                    (right - 8.0, y + 1.0),
                    (right - 8.0, bottom - 1.0),
                    ink,
                    1.2,
                );
            }
        }
    }
    let top = cy - b.lines.len() as f32 * CELL_HEIGHT / 2.0;
    for (i, text) in b.lines.iter().enumerate() {
        image::center(scene, cx, top + i as f32 * CELL_HEIGHT, text, style.text);
    }
}

fn wire(scene: &mut Scene, points: &[Point], edge: &Edge, color: Rgb) {
    let (x, y) = points[0];
    let mut stroke = format!("M {x} {y}");
    if edge.line == Line::Dotted {
        for segment in points.windows(2) {
            let [(x, y), (u, v)] = [segment[0], segment[1]];
            let distance = (u - x).hypot(v - y);
            let parts = (distance / 7.0).ceil() as usize;
            for n in 0..parts {
                let start = n as f32 / parts as f32;
                let end = (n as f32 + 0.47) / parts as f32;
                write!(
                    stroke,
                    " M {} {} L {} {}",
                    x + (u - x) * start,
                    y + (v - y) * start,
                    x + (u - x) * end,
                    y + (v - y) * end,
                )
                .unwrap();
            }
        }
    } else {
        for &(x, y) in &points[1..] {
            write!(stroke, " L {x} {y}").unwrap();
        }
    }
    scene.path(
        &stroke,
        None,
        Some(color),
        if edge.line == Line::Thick { 3.2 } else { 1.7 },
    );
    let (x, y) = points[points.len() - 1];
    let (u, v) = points[points.len() - 2];
    let length = (x - u).hypot(y - v);
    if length == 0.0 {
        return;
    }
    let (dx, dy) = ((x - u) / length, (y - v) / length);
    let (bx, by) = (x - dx * 9.0, y - dy * 9.0);
    match edge.tip {
        Tip::Arrow => image::polygon(
            scene,
            &[
                (x, y),
                (bx + dy * 4.0, by - dx * 4.0),
                (bx - dy * 4.0, by + dx * 4.0),
            ],
            Some(color),
            None,
            0.0,
        ),
        Tip::Cross => {
            let (x, y) = (x - dx * 5.0, y - dy * 5.0);
            image::line(scene, (x - 4.0, y - 4.0), (x + 4.0, y + 4.0), color, 2.0);
            image::line(scene, (x - 4.0, y + 4.0), (x + 4.0, y - 4.0), color, 2.0);
        }
        Tip::None => {}
    }
}

fn elbow(port: Point, cell: Point, side: usize) -> Point {
    if side.is_multiple_of(2) {
        (cell.0, port.1)
    } else {
        (port.0, cell.1)
    }
}

pub(super) fn draw(
    d: &Diagram,
    plan: &Plan,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    let title = image::wrap(scene, &d.title, 520.0);
    let description = image::wrap(scene, &d.description, 520.0);
    let headings: Vec<_> = title
        .iter()
        .chain(&description)
        .filter(|s| !s.is_empty())
        .collect();
    let Rect { w, h, .. } = plan.blocks[0].rect;
    let top = MARGIN + headings.len() as f32 * CELL_HEIGHT;
    let page = Page {
        top,
        width: 2.0 * MARGIN + w as f32 * CELL_WIDTH,
        height: top + h as f32 * CELL_HEIGHT + MARGIN,
    };
    let min_width = headings
        .iter()
        .map(|s| scene.measure(s) + 2.0 * MARGIN)
        .fold(page.width, f32::max);
    scene.resize(min_width, page.height)?;
    let mut routes = Routes::new(w, h);
    for &id in &plan.order[1..] {
        let b = &plan.blocks[id];
        routes.block(b.rect, b.frame);
        if b.frame {
            for (row, s) in b.lines.iter().enumerate() {
                let width = (scene.measure(s) / CELL_WIDTH).ceil() as usize;
                routes.block_text(Cell(b.rect.x + 1, b.rect.y + row), width + 1);
            }
            frame(
                scene,
                &page,
                b,
                item_style(d, id, true, options, scene),
                d.family,
            );
        }
    }
    let mut labels = Vec::new();
    let mut footnotes = Vec::new();
    for edge in &d.edges {
        let from = plan.blocks[edge.from].rect;
        let to = plan.blocks[edge.to].rect;
        let (path, source, target) = routes
            .route(from, to, d.direction, edge.from == edge.to)
            .ok_or_else(|| {
                format!(
                    "cannot route {} to {}",
                    d.items[edge.from].label, d.items[edge.to].label
                )
            })?;
        routes.trace(&path, source, target, edge);
        let style = if options.color {
            edge.style
        } else {
            Style::default()
        };
        let color = style
            .stroke
            .or_else(|| options.theme_line())
            .unwrap_or(scene.ink());
        let first = page.cell(path[0]);
        let last = page.cell(*path.last().unwrap());
        let start = page.port(from, source);
        let end = page.port(to, target);
        let mut points = vec![start, elbow(start, first, source), first];
        for window in path.windows(3) {
            if window[0].0 != window[2].0 && window[0].1 != window[2].1 {
                points.push(page.cell(window[1]));
            }
        }
        if path.len() > 1 {
            points.push(last);
        }
        points.extend([elbow(end, last, target), end]);
        points.dedup();
        wire(scene, &points, edge, color);
        if !edge.label.is_empty() {
            let label = edge.label.replace('\n', " ");
            let width = (scene.measure(&label) / CELL_WIDTH).ceil() as usize + 1;
            if let Some(cell) = routes.label_position(&path, width) {
                labels.push((cell, label, style.text, style.fill));
            } else {
                footnotes.push((
                    format!(
                        "{} → {}: {label}",
                        d.items[edge.from].label, d.items[edge.to].label
                    )
                    .replace('\n', " "),
                    style.text,
                ));
            }
        }
    }
    for &id in &plan.order[1..] {
        let b = &plan.blocks[id];
        if !b.frame {
            card(
                scene,
                &page,
                b,
                item_style(d, id, false, options, scene),
                &d.items[id].shape,
            );
        }
    }
    for (cell, label, color, fill) in labels {
        let (x, y) = page.cell(cell);
        let width = scene.measure(&label);
        scene.rect(
            x - 5.0,
            y - 10.0,
            width + 10.0,
            CELL_HEIGHT,
            4.0,
            fill.or(Some(scene.paper())),
            None,
        );
        scene.text(x, y - 10.0, &label, color);
    }
    for (i, heading) in headings.into_iter().enumerate() {
        let color = if i < title.iter().filter(|s| !s.is_empty()).count() {
            image::theme(options, "titleColor")
        } else {
            None
        };
        scene.text(MARGIN, MARGIN + i as f32 * CELL_HEIGHT, heading, color);
    }
    let mut bottom = page.height;
    for (text, color) in footnotes {
        for row in image::wrap(scene, &text, min_width - 2.0 * MARGIN) {
            scene.text(MARGIN, bottom, &row, color);
            bottom += CELL_HEIGHT;
        }
    }
    scene.resize(min_width, bottom)
}
