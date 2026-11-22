use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::{Options, color::Rgb, graphics::Scene};

use super::{parse, percentage};

const PAD: f32 = 24.0;
const ROW: f32 = 23.0;
const RADIUS: f64 = 132.0;
const DIAMETER: f32 = (RADIUS * 2.0) as f32;

const PALETTE: &[&str] = &[
    "#568bd1", "#efa55e", "#59b4a9", "#c17eb0", "#e3c357", "#8696b0", "#df837f", "#8f9c6e",
];
const MONOCHROME: &[&str] = &[
    "#676767", "#b2b2b2", "#828282", "#dbdbdb", "#969696", "#4d4d4d",
];

struct Entry {
    percent: String,
    label: Vec<String>,
    positive: bool,
    weight: f64,
    fill: Option<Rgb>,
}

pub(crate) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let pie = parse(source)?;
    let theme = |key: &str| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let stroke = theme("pieStrokeColor").or(Some(scene.paper()));
    let outline = theme("pieOuterStrokeColor").or(Some(scene.ink()));
    let legend_color = theme("pieLegendTextColor");
    let section_color = theme("pieSectionTextColor").or(legend_color);
    let palette = if options.color { PALETTE } else { MONOCHROME };

    let max = pie.max_value();
    let weight = |value| if max > 0.0 { value / max } else { 0.0 };
    let total: f64 = pie.slices.iter().map(|slice| weight(slice.value)).sum();
    let entries: Vec<_> = pie
        .slices
        .iter()
        .enumerate()
        .map(|(index, slice)| {
            let mut label = slice
                .label
                .split('\n')
                .map(|line| line.replace('\t', " "))
                .collect::<Vec<_>>();
            if pie.show_data {
                let last = label.last_mut().unwrap();
                last.push_str(&format!(" [{}]", slice.raw_value));
            }
            Entry {
                percent: percentage(slice.value, max, total),
                label,
                positive: slice.value > 0.0,
                weight: weight(slice.value),
                fill: theme(&format!("pie{}", index + 1))
                    .or_else(|| Rgb::parse(palette[index % palette.len()])),
            }
        })
        .collect();
    let headings: Vec<_> = [
        pie.title,
        pie.accessible_title
            .filter(|&title| Some(title) != pie.title),
    ]
    .into_iter()
    .flatten()
    .filter(|title| !title.is_empty())
    .map(|title| title.replace('\t', " "))
    .collect();
    let description: Vec<_> = pie
        .description
        .unwrap_or("")
        .trim()
        .lines()
        .map(|line| line.trim().replace('\t', " "))
        .collect();

    let measured = |text: &str| scene.measure(text);
    let percent_width = entries
        .iter()
        .map(|entry| measured(&entry.percent))
        .fold(0.0, f32::max);
    let label_width = entries
        .iter()
        .flat_map(|entry| &entry.label)
        .map(|line| measured(line))
        .fold(0.0, f32::max);
    let legend_width = 36.0 + percent_width + label_width;
    let legend_height = entries
        .iter()
        .map(|entry| entry.label.len() as f32 * ROW + 4.0)
        .sum::<f32>();
    let body_width = DIAMETER
        + if entries.is_empty() {
            0.0
        } else {
            30.0 + legend_width
        };
    let body_height = DIAMETER.max(legend_height);
    let heading_height = if headings.is_empty() {
        0.0
    } else {
        headings.len() as f32 * ROW + 12.0
    };
    let description_height = if description.is_empty() {
        0.0
    } else {
        description.len() as f32 * ROW + 12.0
    };
    let width = headings
        .iter()
        .chain(&description)
        .map(|line| measured(line))
        .fold(body_width, f32::max)
        + 2.0 * PAD;
    let height = 2.0 * PAD + heading_height + body_height + description_height;
    scene.resize(width, height)?;

    for (row, title) in headings.iter().enumerate() {
        scene.text(
            (width - scene.measure(title)) / 2.0,
            PAD + row as f32 * ROW,
            title,
            theme("pieTitleTextColor"),
        );
    }
    let body_top = PAD + heading_height;
    let body_left = (width - body_width) / 2.0;
    let x = f64::from(body_left) + RADIUS;
    let y = f64::from(body_top + body_height / 2.0);
    let circle = format!(
        "M {x} {} A {RADIUS} {RADIUS} 0 0 1 {x} {} A {RADIUS} {RADIUS} 0 0 1 {x} {} Z",
        y - RADIUS,
        y + RADIUS,
        y - RADIUS,
    );
    let point = |angle: f64, radius: f64| {
        let (sin, cos) = (angle - FRAC_PI_2).sin_cos();
        (x + radius * cos, y + radius * sin)
    };
    let mut labels = Vec::new();
    let mut cumulative = 0.0;
    for entry in entries.iter().filter(|entry| entry.weight > 0.0) {
        let start = cumulative / total * TAU;
        cumulative += entry.weight;
        let end = cumulative / total * TAU;
        let angle = end - start;
        if angle <= 0.0 {
            continue;
        }
        if angle == TAU {
            scene.path(&circle, entry.fill, None, 0.0);
            labels.push((x as f32, y as f32, entry.percent.as_str()));
            continue;
        }
        let middle = (start + end) / 2.0;
        let (sx, sy) = point(start, RADIUS);
        let (ex, ey) = point(end, RADIUS);
        let mut sector = format!("M {x} {y} L {sx} {sy}");
        if angle > PI {
            let (mx, my) = point(middle, RADIUS);
            sector.push_str(&format!(" A {RADIUS} {RADIUS} 0 0 1 {mx} {my}"));
        }
        sector.push_str(&format!(" A {RADIUS} {RADIUS} 0 0 1 {ex} {ey} Z"));
        scene.path(&sector, entry.fill, stroke, 1.0);

        let distance = RADIUS * 0.68;
        let label_width = scene.measure(&entry.percent) + 8.0;
        let clearance = (angle / 2.0).min(FRAC_PI_2).sin() * distance;
        if f64::from(label_width.hypot(22.0)) / 2.0 < clearance.min(RADIUS - distance) {
            let (lx, ly) = point(middle, distance);
            labels.push((lx as f32, ly as f32, entry.percent.as_str()));
        }
    }
    scene.path(&circle, None, outline, 1.5);
    for (lx, ly, text) in labels {
        let label_width = scene.measure(text);
        scene.rect(
            lx - label_width / 2.0 - 4.0,
            ly - 11.0,
            label_width + 8.0,
            22.0,
            3.0,
            Some(scene.paper()),
            None,
        );
        scene.text(lx - label_width / 2.0, ly - 10.0, text, section_color);
    }
    if entries.is_empty() {
        let text = "No slices";
        scene.text(
            x as f32 - scene.measure(text) / 2.0,
            y as f32 - 10.0,
            text,
            None,
        );
    }

    let legend_x = body_left + DIAMETER + 30.0;
    let percent_right = legend_x + 24.0 + percent_width;
    let mut row_top = body_top + (body_height - legend_height) / 2.0;
    for entry in entries {
        let (swatch_y, swatch_height) = if entry.positive {
            (row_top + 3.0, 14.0)
        } else {
            (row_top + 9.0, 2.0)
        };
        scene.rect(
            legend_x,
            swatch_y,
            16.0,
            swatch_height,
            1.0,
            entry.fill,
            outline,
        );
        scene.text(
            percent_right - scene.measure(&entry.percent),
            row_top,
            &entry.percent,
            legend_color,
        );
        for line in entry.label {
            scene.text(percent_right + 12.0, row_top, &line, legend_color);
            row_top += ROW;
        }
        row_top += 4.0;
    }
    for (row, line) in description.iter().enumerate() {
        scene.text(
            PAD,
            body_top + body_height + 12.0 + row as f32 * ROW,
            line,
            None,
        );
    }
    Ok(())
}
