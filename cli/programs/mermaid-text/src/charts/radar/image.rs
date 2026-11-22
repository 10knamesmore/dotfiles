use super::{Diagram, exact, parse, scale_ticks};
use crate::{
    Options,
    charts::shared::plot::image::{
        MARGIN, Point, ROW, center, circle, fade, line, palette, polygon, theme, wrap,
    },
    color::Rgb,
    graphics::Scene,
};
use std::f32::consts::TAU;

const LABEL_GAP: f32 = 12.0;

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let Diagram {
        title,
        axes,
        series,
        low,
        high,
        ticks,
        polygon: angular,
        legend,
        clipped,
        metadata,
    } = parse(source)?;
    let axis = theme(options, "radar.axisColor")
        .or(options.theme_line())
        .unwrap_or_else(|| fade(scene.ink(), 0.55));
    let grid = theme(options, "radar.graticuleColor").unwrap_or_else(|| fade(scene.ink(), 0.28));
    let colors: Vec<_> = (0..series.len())
        .map(|i| theme(options, &format!("cScale{i}")).unwrap_or_else(|| palette(i, options.color)))
        .collect();
    let max_axis_width = axes
        .iter()
        .map(|axis| scene.measure(&axis.name))
        .fold(0.0, f32::max);
    let keyed = axes.len() > 20 || max_axis_width > 200.0;
    let labels: Vec<_> = axes
        .iter()
        .enumerate()
        .map(|(i, axis)| {
            if keyed {
                (i + 1).to_string()
            } else {
                axis.name.clone()
            }
        })
        .collect();
    let label_width = if keyed {
        scene.measure(&axes.len().to_string())
    } else {
        max_axis_width
    };
    let radius = 180.0;
    let width = 600.0_f32.max(2.0 * (radius + label_width + LABEL_GAP + MARGIN));
    let title_rows = if title.is_empty() {
        Vec::new()
    } else {
        wrap(scene, &title, width - 2.0 * MARGIN)
    };
    let top = MARGIN + title_rows.len() as f32 * ROW;
    let origin = (width / 2.0, top + radius + ROW + LABEL_GAP);
    let point = |i: usize, unit: f32| {
        let angle = TAU * i as f32 / axes.len() as f32;
        (
            origin.0 + angle.sin() * radius * unit,
            origin.1 - angle.cos() * radius * unit,
        )
    };
    for (i, row) in title_rows.iter().enumerate() {
        center(
            scene,
            origin.0,
            MARGIN + i as f32 * ROW,
            row,
            theme(options, "titleColor"),
        );
    }
    for ring in 1..=ticks {
        let unit = ring as f32 / ticks as f32;
        if angular && axes.len() >= 3 {
            let vertices: Vec<_> = (0..axes.len()).map(|i| point(i, unit)).collect();
            polygon(scene, &vertices, None, Some(grid), 1.0);
        } else {
            circle(scene, origin, radius * unit, None, Some(grid), 1.0);
        }
    }
    for i in 0..axes.len() {
        line(scene, origin, point(i, 1.0), axis, 1.0);
    }
    if ticks > 0 {
        let label_every = ticks.div_ceil(7);
        for (i, (_, label)) in scale_ticks(low, high, ticks + 1).into_iter().enumerate() {
            if i == ticks || i.is_multiple_of(label_every) {
                let y = origin.1 - radius * i as f32 / ticks as f32;
                line(scene, (origin.0 - 4.0, y), (origin.0 + 4.0, y), axis, 1.0);
                scene.text(origin.0 + 6.0, y - 10.0, &label, None);
            }
        }
    }
    for (index, ((_, values), &color)) in series.iter().zip(&colors).enumerate() {
        let points: Vec<_> = values
            .iter()
            .enumerate()
            .map(|(i, value)| point(i, ((value - low) / (high - low)).clamp(0.0, 1.0) as f32))
            .collect();
        let fill = (points.len() >= 3).then_some(fade(color, 0.16));
        polygon(scene, &points, fill, Some(color), 2.4);
        for at in points {
            marker(scene, at, index, color);
        }
    }
    for side in [-1.0_f32, 1.0] {
        let mut slots = Vec::new();
        let label_every = axes.len().div_ceil(28).max(1);
        for (i, label) in labels
            .iter()
            .enumerate()
            .filter(|(i, _)| i.is_multiple_of(label_every))
        {
            let (x, y) = point(i, 1.0);
            if i == 0 || 2 * i == labels.len() {
                if side > 0.0 {
                    center(
                        scene,
                        x,
                        if i == 0 {
                            y - ROW - LABEL_GAP
                        } else {
                            y + LABEL_GAP
                        },
                        label,
                        None,
                    );
                }
            } else if (x - origin.0) * side > 0.0 {
                slots.push(((x, y), label));
            }
        }
        slots.sort_by(|a, b| a.0.1.total_cmp(&b.0.1));
        let mut last = top;
        let count = slots.len();
        for (i, ((x, y), label)) in slots.into_iter().enumerate() {
            let max_y = origin.1 + radius + ROW - (count - i) as f32 * ROW;
            let row = (y - 10.0).max(last).min(max_y);
            last = row + ROW;
            let dy = ((row + 10.0 - origin.1) / radius).abs().max(0.0);
            let edge = radius * (1.0 - dy * dy).max(0.0).sqrt();
            let near = origin.0 + side * ((x - origin.0).abs().max(edge) + LABEL_GAP);
            line(scene, (x, y), (near - side * 4.0, row + 10.0), axis, 0.8);
            let start = if side < 0.0 {
                near - scene.measure(label)
            } else {
                near
            };
            scene.text(start, row, label, None);
        }
    }
    let plot_bottom = if angular && axes.len() >= 3 {
        (0..axes.len())
            .map(|i| point(i, 1.0).1)
            .fold(origin.1, f32::max)
    } else {
        origin.1 + radius
    };
    let mut y = plot_bottom + ROW + 30.0;
    if legend {
        for (i, ((name, values), &color)) in series.iter().zip(&colors).enumerate() {
            line(
                scene,
                (MARGIN, y + 10.0),
                (MARGIN + 28.0, y + 10.0),
                color,
                2.4,
            );
            marker(scene, (MARGIN + 14.0, y + 10.0), i, color);
            let caption = format!(
                "{name}: {}",
                values
                    .iter()
                    .map(|&v| exact(v))
                    .collect::<Vec<_>>()
                    .join("  ·  ")
            );
            for row in wrap(scene, &caption, width - 2.0 * MARGIN - 40.0) {
                scene.text(MARGIN + 40.0, y, &row, None);
                y += ROW;
            }
        }
    }
    let mut notes = Vec::new();
    if ticks == 0 {
        notes.push(format!("Scale: {} – {}", exact(low), exact(high)));
    }
    if keyed {
        notes.extend(
            axes.iter()
                .enumerate()
                .map(|(i, axis)| format!("{}  {}", i + 1, axis.name)),
        );
    }
    if clipped {
        notes.push("Values outside the radial scale are clipped at its limits.".into());
    }
    notes.extend(metadata);
    for note in notes {
        for row in wrap(scene, &note, width - 2.0 * MARGIN) {
            scene.text(MARGIN, y, &row, None);
            y += ROW;
        }
    }
    scene.resize(width, y + MARGIN)
}

fn marker(scene: &mut Scene, (x, y): Point, index: usize, color: Rgb) {
    let r = 4.0;
    let edge = Some(scene.paper());
    match index % 4 {
        0 => circle(scene, (x, y), r, Some(color), edge, 1.0),
        1 => polygon(
            scene,
            &[(x - r, y), (x, y - r), (x + r, y), (x, y + r)],
            Some(color),
            edge,
            1.0,
        ),
        2 => scene.rect(x - r, y - r, 2.0 * r, 2.0 * r, 0.0, Some(color), edge),
        _ => polygon(
            scene,
            &[(x, y - r), (x + r, y + r), (x - r, y + r)],
            Some(color),
            edge,
            1.0,
        ),
    }
}
