use std::fmt::Write;

use super::{Map, parse};
use crate::{
    Options,
    charts::shared::plot::image::{self, Point},
    color::{Rgb, Style},
    graphics::Scene,
};

const LEFT: f32 = 68.0;
const RIGHT: f32 = 28.0;
const ROW: f32 = 20.0;

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn marker((x, y): Point, r: f32) -> Self {
        Self {
            x: x - r,
            y: y - r,
            w: 2.0 * r,
            h: 2.0 * r,
        }
    }

    fn overlaps(self, b: Self) -> bool {
        self.x < b.x + b.w && b.x < self.x + self.w && self.y < b.y + b.h && b.y < self.y + self.h
    }

    fn inside(self, parent: Self) -> bool {
        self.x >= parent.x
            && self.y >= parent.y
            && self.x + self.w <= parent.x + parent.w
            && self.y + self.h <= parent.y + parent.h
    }

    fn project(self, (visibility, evolution): super::Point) -> Point {
        (
            self.x + self.w * evolution as f32,
            self.y + self.h * (1.0 - visibility as f32),
        )
    }
}

struct Colors {
    paper: Rgb,
    axis: Rgb,
    text: Rgb,
    link: Rgb,
    evolution: Rgb,
    component: Style,
    annotation: Style,
}

impl Colors {
    fn new(scene: &Scene, options: &Options) -> Self {
        let theme = |key| image::theme(options, key);
        let paper = theme("wardley.backgroundColor").unwrap_or(scene.paper());
        let ink = theme("textColor").unwrap_or_else(|| {
            if theme("wardley.backgroundColor").is_some() {
                image::contrast(paper, scene.paper())
            } else {
                scene.ink()
            }
        });
        let line = options.theme_line().unwrap_or(ink);
        let text = options.theme_style().text.unwrap_or(ink);
        let component_stroke = theme("wardley.componentStroke").unwrap_or(if options.color {
            image::palette(2, true)
        } else {
            line
        });
        let evolution = theme("wardley.evolutionStroke")
            .or_else(|| theme("wardleyEvolutionColor"))
            .unwrap_or(if options.color {
                image::palette(1, true)
            } else {
                line
            });
        Self {
            paper,
            axis: theme("wardley.axisColor").unwrap_or(line),
            text: theme("wardley.axisTextColor").unwrap_or(text),
            link: theme("wardley.linkStroke").unwrap_or(line),
            evolution,
            component: Style {
                fill: theme("wardley.componentFill").or(Some(paper)),
                stroke: Some(component_stroke),
                text: theme("wardley.componentLabelColor").or(Some(text)),
            },
            annotation: Style {
                fill: theme("wardley.annotationFill").or(Some(paper)),
                stroke: Some(theme("wardley.annotationStroke").unwrap_or(line)),
                text: theme("wardley.annotationTextColor").or(Some(text)),
            },
        }
    }
}

struct Label {
    rows: Vec<String>,
    bounds: Rect,
    style: Style,
}

impl Label {
    fn new(scene: &Scene, text: &str, style: Style, limit: f32) -> Self {
        let rows = image::wrap(scene, text, limit);
        let w = rows
            .iter()
            .map(|line| scene.measure(line))
            .fold(0.0, f32::max)
            + 12.0;
        Self {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w,
                h: ROW * rows.len() as f32 + 6.0,
            },
            rows,
            style,
        }
    }

    fn draw(&self, scene: &mut Scene) {
        let Rect { x, y, w, h } = self.bounds;
        scene.rect(x, y, w, h, 4.0, self.style.fill, self.style.stroke);
        for (i, text) in self.rows.iter().enumerate() {
            scene.text(x + 6.0, y + 3.0 + ROW * i as f32, text, self.style.text);
        }
    }
}

fn place(
    label: &mut Label,
    (x, y): Point,
    (dx, dy): super::Point,
    region: Rect,
    occupied: &mut Vec<Rect>,
) -> bool {
    let Rect { w, h, .. } = label.bounds;
    let offset_x = dx.abs().min(f64::from(region.w)) as f32;
    let offset_y = (dy as f32).clamp(-region.h, region.h);
    for (horizontal, vertical) in [(offset_x, offset_y), (0.0, 0.0)] {
        let gap = 11.0 + horizontal;
        let xs = [x + gap, x - w - gap];
        for step in [0.0, -32.0, 32.0, -64.0, 64.0, -96.0, 96.0] {
            let y = y - h / 2.0 + vertical + step;
            for side in [usize::from(dx < 0.0), usize::from(dx >= 0.0)] {
                let x = xs[side];
                let bounds = Rect { x, y, w, h };
                if bounds.inside(region) && occupied.iter().all(|&r| !bounds.overlaps(r)) {
                    label.bounds = bounds;
                    occupied.push(bounds);
                    return true;
                }
            }
        }
    }
    false
}

fn arrow(scene: &mut Scene, (x, y): Point, (dx, dy): Point, color: Rgb) {
    let (bx, by) = (x - dx * 9.0, y - dy * 9.0);
    image::polygon(
        scene,
        &[
            (x, y),
            (bx + dy * 4.0, by - dx * 4.0),
            (bx - dy * 4.0, by + dx * 4.0),
        ],
        Some(color),
        None,
        0.0,
    );
}

fn connection(scene: &mut Scene, from: Point, to: Point, style: &str, color: Rgb) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy);
    if length <= 15.0 {
        let (x, y) = (from.0 - 3.0, from.1 - 6.0);
        let (u, v) = (to.0 + 3.0, to.1 - 6.0);
        scene.path(
            &format!(
                "M {x} {y} c -19 -25 {} {} {} {}",
                u - x + 19.0,
                v - y - 25.0,
                u - x,
                v - y
            ),
            None,
            Some(color),
            1.4,
        );
        if matches!(style, "+<" | "+<>") {
            arrow(scene, (x, y), (0.6, 0.8), color);
        }
        if !matches!(style, "+<" | "pipeline") {
            arrow(scene, (u, v), (-0.6, 0.8), color);
        }
        return;
    }
    let (dx, dy) = (dx / length, dy / length);
    let (x, y) = (from.0 + dx * 6.5, from.1 + dy * 6.5);
    let (u, v) = (to.0 - dx * 6.5, to.1 - dy * 6.5);
    if matches!(style, "-.->" | "trend" | "pipeline") {
        let distance = length - 13.0;
        let mut path = String::new();
        let mut start = 0.0;
        while start < distance {
            let stop = (start + 4.0).min(distance);
            write!(
                path,
                "M {} {} L {} {} ",
                x + start * dx,
                y + start * dy,
                x + stop * dx,
                y + stop * dy
            )
            .unwrap();
            start += 8.0;
        }
        scene.path(&path, None, Some(color), 1.6);
    } else {
        image::line(
            scene,
            (x, y),
            (u, v),
            color,
            if style.starts_with('+') { 2.6 } else { 1.5 },
        );
    }
    if matches!(style, "+<" | "+<>") {
        arrow(scene, (x, y), (-dx, -dy), color);
    }
    if !matches!(style, "+<" | "pipeline") {
        arrow(scene, (u, v), (dx, dy), color);
    }
}

struct Stages {
    bands: Vec<(f32, f32, Vec<String>)>,
    notes: Vec<String>,
    rows: usize,
}

impl Stages {
    fn new(scene: &Scene, map: &Map, plot: Rect) -> Self {
        let mut bands = Vec::new();
        let mut notes = Vec::new();
        let mut rows = 1;
        let mut start = 0.0;
        for (name, end) in &map.stages {
            let (left, _) = plot.project((0.0, start));
            let (right, _) = plot.project((0.0, *end));
            let label = if right - left < 45.0 {
                notes.extend(image::wrap(
                    scene,
                    &format!("{start:.2}–{end:.2}: {name}"),
                    plot.w,
                ));
                Vec::new()
            } else {
                image::wrap(scene, name, right - left - 10.0)
            };
            rows = rows.max(label.len());
            bands.push((left, right, label));
            start = *end;
        }
        Self { bands, notes, rows }
    }

    fn bottom(&self, plot: Rect) -> f32 {
        plot.y + plot.h + ROW * (self.rows + self.notes.len() + 1) as f32 + 25.0
    }
}

fn draw_axes(scene: &mut Scene, plot: Rect, stages: &Stages, colors: &Colors) {
    let Rect { x, y, w, h } = plot;
    let (right, bottom) = (x + w, y + h);
    let axis = colors.axis;
    let guide = image::fade(axis, 0.27);
    for value in [1.0, 0.5, 0.0] {
        let (_, line_y) = plot.project((value, 0.0));
        image::line(
            scene,
            (x - 6.0, line_y),
            (right, line_y),
            if value == 0.0 { axis } else { guide },
            1.2,
        );
        scene.text(
            x - 36.0,
            line_y - 10.0,
            &format!("{value:.1}"),
            Some(colors.text),
        );
    }
    image::line(scene, (x, bottom), (x, y - 10.0), axis, 1.7);
    image::line(scene, (x, bottom), (right + 12.0, bottom), axis, 1.7);
    arrow(scene, (x, y - 10.0), (0.0, -1.0), axis);
    arrow(scene, (right + 12.0, bottom), (1.0, 0.0), axis);
    scene.text(20.0, y - 34.0, "Visibility", Some(colors.text));
    for (start, stop, rows) in &stages.bands {
        let stop = *stop;
        image::line(scene, (stop, y), (stop, bottom), guide, 1.0);
        image::line(scene, (stop, bottom), (stop, bottom + 5.0), axis, 1.2);
        for (i, text) in rows.iter().enumerate() {
            image::center(
                scene,
                (start + stop) / 2.0,
                bottom + 8.0 + ROW * i as f32,
                text,
                Some(colors.text),
            );
        }
    }
    let mut bottom = bottom + ROW * stages.rows as f32 + 13.0;
    for row in &stages.notes {
        scene.text(x, bottom, row, Some(colors.text));
        bottom += ROW;
    }
    scene.text(x, bottom, "0.0", Some(colors.text));
    scene.text(
        right - scene.measure("1.0"),
        bottom,
        "1.0",
        Some(colors.text),
    );
    image::center(scene, x + w / 2.0, bottom, "Evolution", Some(colors.text));
}

pub(super) fn draw(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let map = parse(source)?;
    let colors = Colors::new(scene, options);
    let width = (map.size.0 as f32).clamp(660.0, 1400.0);
    let title = image::wrap(scene, &map.title, width - 40.0);
    let title_rows = if map.title.is_empty() { 0 } else { title.len() };
    let plot = Rect {
        x: LEFT,
        y: 55.0 + ROW * title_rows as f32,
        w: width - LEFT - RIGHT,
        h: (width as f64 * map.size.1 / map.size.0).clamp(280.0, 970.0) as f32,
    };
    let region = Rect {
        x: LEFT + 3.0,
        y: plot.y - 3.0,
        w: plot.w + RIGHT - 9.0,
        h: plot.h + 2.0,
    };
    let points: Vec<_> = map
        .items
        .iter()
        .map(|item| plot.project(item.point))
        .collect();
    let mut occupied: Vec<_> = points
        .iter()
        .map(|&point| Rect::marker(point, 10.0))
        .collect();
    let stages = Stages::new(scene, &map, plot);
    let mut labels = Vec::new();
    let mut overflow = Vec::new();
    let item_label_style = Style {
        stroke: None,
        fill: Some(colors.paper),
        text: colors.component.text,
    };
    let mut legend = None;
    if let Some(position) = map.annotations
        && map
            .items
            .iter()
            .any(|item| item.kind.starts_with("annotation "))
    {
        let mut entries: Vec<_> = map
            .items
            .iter()
            .filter_map(|item| {
                let number = item
                    .kind
                    .strip_prefix("annotation ")?
                    .parse::<usize>()
                    .ok()?;
                Some((number, &item.name))
            })
            .collect();
        entries.sort_by_key(|(number, _)| *number);
        let text = entries
            .into_iter()
            .map(|(n, name)| format!("{n}. {name}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut label = Label::new(scene, &text, colors.annotation, 220.0);
        let (x, y) = plot.project(position);
        if label.bounds.h <= region.h {
            label.bounds.x = x.clamp(region.x, region.x + region.w - label.bounds.w);
            label.bounds.y = y.clamp(region.y, region.y + region.h - label.bounds.h);
            occupied.push(label.bounds);
            labels.push((label, None));
        } else {
            label.bounds.x = LEFT;
            legend = Some(label);
        }
    }
    for (i, item) in map.items.iter().enumerate() {
        let (kind, qualifier) = item.kind.split_once(' ').unwrap_or((&item.kind, ""));
        if kind == "annotation" && map.annotations.is_some() {
            continue;
        }
        let (title, style) = match kind {
            "annotation" => (format!("{qualifier}. {}", item.name), colors.annotation),
            "note" => (item.name.clone(), colors.annotation),
            "component" if !qualifier.is_empty() => {
                (format!("{} ({qualifier})", item.name), item_label_style)
            }
            "accelerator" | "deaccelerator" => {
                (format!("{} ({})", item.name, item.kind), item_label_style)
            }
            _ => (item.name.clone(), item_label_style),
        };
        let mut label = Label::new(scene, &title, style, 220.0);
        let marker = points[i];
        if place(&mut label, marker, item.offset, region, &mut occupied) {
            labels.push((label, Some(marker)));
        } else {
            overflow.push((marker, title, style));
        }
    }
    let mut links = Vec::new();
    for link in &map.links {
        let a = points[map.lookup(&link.from)?];
        let b = points[map.lookup(&link.to)?];
        links.push((a, b, link.style, colors.link));
        if !link.caption.is_empty() {
            let anchor = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let style = Style {
                text: Some(colors.link),
                ..item_label_style
            };
            let mut label = Label::new(scene, &link.caption, style, 185.0);
            if place(&mut label, anchor, (0.0, 0.0), region, &mut occupied) {
                labels.push((label, Some(anchor)));
            } else {
                overflow.push((anchor, link.caption.clone(), style));
            }
        }
    }
    for (i, item) in map.items.iter().enumerate() {
        if let Some(parent) = item.parent.as_deref() {
            links.push((
                points[map.lookup(parent)?],
                points[i],
                "pipeline",
                colors.link,
            ));
        }
    }
    let mut evolved = Vec::new();
    for (name, target_visibility, target_evolution) in &map.moves {
        let id = map.lookup(name)?;
        let from = points[id];
        let to = plot.project((
            target_visibility.unwrap_or(map.items[id].point.0),
            *target_evolution,
        ));
        links.push((from, to, "trend", colors.evolution));
        evolved.push(to);
    }
    let mut bottom = stages.bottom(plot);
    if let Some(label) = &mut legend {
        label.bounds.y = bottom;
        bottom += label.bounds.h + 10.0;
    }
    let overflow: Vec<_> = overflow
        .into_iter()
        .enumerate()
        .map(|(i, (point, text, style))| {
            let key = format!("#{}", i + 1);
            let rows = image::wrap(scene, &format!("{key} {text}"), width - 2.0 * LEFT);
            (key, point, rows, style.text)
        })
        .collect();
    let height = bottom
        + ROW
            * overflow
                .iter()
                .map(|(_, _, rows, _)| rows.len())
                .sum::<usize>() as f32
        + 10.0;
    scene.resize(width, height)?;
    if colors.paper != scene.paper() {
        scene.rect(
            0.0,
            0.0,
            width,
            height.ceil(),
            0.0,
            Some(colors.paper),
            None,
        );
    }
    for (i, text) in title.iter().take(title_rows).enumerate() {
        scene.text(
            20.0,
            16.0 + ROW * i as f32,
            text,
            Some(image::theme(options, "titleColor").unwrap_or(colors.text)),
        );
    }
    draw_axes(scene, plot, &stages, &colors);
    for (a, b, style, color) in links {
        connection(scene, a, b, style, color);
    }
    for point in evolved {
        image::circle(
            scene,
            point,
            6.0,
            Some(colors.paper),
            Some(colors.evolution),
            1.8,
        );
    }
    for (i, item) in map.items.iter().enumerate() {
        let (x, y) = points[i];
        let (kind, qualifier) = item.kind.split_once(' ').unwrap_or((&item.kind, ""));
        let stroke = colors.component.stroke.unwrap();
        match kind {
            "anchor" => image::polygon(
                scene,
                &[(x, y - 8.0), (x + 8.0, y), (x, y + 8.0), (x - 8.0, y)],
                Some(stroke),
                None,
                0.0,
            ),
            "annotation" => {
                let r = (scene.measure(qualifier) / 2.0 + 4.0).max(11.0);
                image::circle(
                    scene,
                    (x, y),
                    r,
                    colors.annotation.fill,
                    colors.annotation.stroke,
                    1.5,
                );
                image::center(scene, x, y - ROW / 2.0, qualifier, colors.annotation.text);
            }
            "note" => scene.rect(
                x - 5.0,
                y - 5.0,
                10.0,
                10.0,
                2.0,
                colors.annotation.fill,
                colors.annotation.stroke,
            ),
            "accelerator" | "deaccelerator" => {
                let sign = if kind == "accelerator" { 1.0 } else { -1.0 };
                for dx in [-4.0, 4.0] {
                    let u = x + dx + sign * 5.0;
                    scene.path(
                        &format!(
                            "M {} {} L {u} {y} L {} {}",
                            x + dx,
                            y - 6.0,
                            x + dx,
                            y + 6.0
                        ),
                        None,
                        Some(stroke),
                        1.7,
                    );
                }
            }
            _ => image::circle(scene, (x, y), 6.0, colors.component.fill, Some(stroke), 2.0),
        }
    }
    for (label, point) in labels {
        if let Some((x, y)) = point {
            let Rect {
                x: left,
                y: top,
                w,
                h,
            } = label.bounds;
            let mid = top + h / 2.0;
            let side = if x < left { left } else { left + w };
            if (mid - y).abs() > 3.0 || (side - x).abs() > 18.0 {
                image::line(
                    scene,
                    (x, y),
                    (side, mid),
                    image::fade(colors.text, 0.65),
                    1.0,
                );
            }
        }
        label.draw(scene);
    }
    if let Some(legend) = legend {
        legend.draw(scene);
    }
    for (key, (x, y), rows, text) in overflow {
        scene.text(
            (x + 8.0).min(width - scene.measure(&key) - 4.0),
            y - 20.0,
            &key,
            text,
        );
        for row in rows {
            scene.text(LEFT, bottom, &row, text);
            bottom += ROW;
        }
    }
    Ok(())
}
