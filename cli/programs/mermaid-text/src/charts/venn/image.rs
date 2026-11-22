use super::{
    Panel, Region, body,
    circles::{Circle, arrangement},
    panels, parse,
};
use crate::{
    Options,
    charts::shared::plot::image::{
        MARGIN, ROW, center, circle, contrast, fade, palette, theme, wrap,
    },
    color::Style,
    graphics::Scene,
};
use std::{
    f64::consts::{PI, TAU},
    fmt::Write,
};

const WIDTH: f32 = 680.0;

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let diagram = parse(body(source, |header| {
        header.eq_ignore_ascii_case("venn-beta")
    })?)?;
    let mut y = MARGIN;
    if !diagram.title.is_empty() {
        for row in wrap(scene, &diagram.title, WIDTH - 2.0 * MARGIN) {
            center(scene, WIDTH / 2.0, y, &row, theme(options, "titleColor"));
            y += ROW;
        }
    }
    for panel in panels(&diagram)? {
        y = draw(scene, &panel, y, options);
        scene.resize(WIDTH, y + MARGIN)?;
    }
    Ok(())
}

fn draw(scene: &mut Scene, panel: &Panel<'_>, top: f32, options: &Options) -> f32 {
    let mut circles = arrangement(&panel.sets, &panel.intersections);
    let [min_x, min_y, max_x, max_y] = bounds(&circles);
    let span = if circles.len() == 1 { 285.0 } else { 580.0 };
    let scale = (span / (max_x - min_x)).min(440.0 / (max_y - min_y));
    let x_offset = (f64::from(WIDTH) - (max_x - min_x) * scale) / 2.0;
    for c in &mut circles {
        c.x = (c.x - min_x) * scale + x_offset;
        c.y = (c.y - min_y) * scale + f64::from(top);
        c.r *= scale;
    }
    let themed = options.theme_style();
    let declared = |r: &Region| {
        if options.color {
            r.style
        } else {
            Style::default()
        }
    };
    let styles: Vec<_> = panel
        .sets
        .iter()
        .enumerate()
        .map(|(i, region)| {
            let style = declared(region);
            let base = style.stroke.unwrap_or_else(|| palette(i, options.color));
            Style {
                fill: style
                    .fill
                    .or(themed.fill.map(|fill| fade(fill, 0.38)))
                    .or(Some(fade(base, 0.22))),
                stroke: style.stroke.or(style.fill).or(themed.stroke).or(Some(base)),
                text: style.text,
            }
        })
        .collect();
    for (c, style) in circles.iter().zip(&styles) {
        circle(
            scene,
            (c.x as f32, c.y as f32),
            c.r as f32,
            style.fill,
            None,
            0.0,
        );
    }
    let mut intersections: Vec<_> = panel
        .intersections
        .iter()
        .copied()
        .filter(|(_, region)| region.size != Some(0.0))
        .filter_map(|(mask, region)| {
            let style = declared(region);
            (style.fill.is_some() || style.stroke.is_some())
                .then(|| intersection_path(&circles, mask).map(|path| (mask, style, path)))
                .flatten()
        })
        .collect();
    intersections.sort_by_key(|(mask, _, _)| mask.count_ones());
    for (_, style, path) in &intersections {
        if style.fill.is_some() {
            scene.path(path, style.fill, None, 0.0);
        }
    }
    for (c, style) in circles.iter().zip(&styles) {
        circle(
            scene,
            (c.x as f32, c.y as f32),
            c.r as f32,
            None,
            style.stroke,
            2.0,
        );
    }
    for (_, style, path) in intersections {
        if style.stroke.is_some() {
            scene.path(&path, None, style.stroke, 2.3);
        }
    }

    let mut y = top + ((max_y - min_y) * scale) as f32 + 18.0;
    let mut legend = Vec::new();
    let plot = bounds(&circles);
    for (i, (mask, region)) in panel
        .sets
        .iter()
        .enumerate()
        .map(|(i, r)| (1 << i, *r))
        .chain(panel.intersections.iter().copied())
        .enumerate()
    {
        let style = declared(region);
        let base = styles.get(i).copied().unwrap_or(style);
        let default_ink = base.fill.map(|fill| contrast(fill, scene.paper()));
        let ink = style.text.or(themed.text).or(default_ink);
        let mut name = region.name(options.ascii);
        if let Some(size) = region.size {
            write!(name, " ({})", super::super::shared::plot::exact(size)).unwrap();
        }
        let mut labels: Vec<_> = wrap(scene, &name, 208.0)
            .into_iter()
            .map(|row| (row, ink))
            .collect();
        for item in &region.items {
            let color = options.color.then_some(item.style.text).flatten().or(ink);
            labels.extend(
                wrap(scene, item.text(), 208.0)
                    .into_iter()
                    .map(|row| (row, color)),
            );
        }
        let w = labels
            .iter()
            .map(|(row, _)| scene.measure(row))
            .fold(0.0, f32::max);
        let h = labels.len() as f32 * 20.0;
        if region.size != Some(0.0)
            && let Some((x, yc)) = label_position(&circles, mask, plot, w, h)
        {
            for (i, (row, color)) in labels.iter().enumerate() {
                center(scene, x, yc - h / 2.0 + i as f32 * 20.0, row, *color);
            }
            continue;
        }
        let key = (i + 1).to_string();
        let at = if region.size == Some(0.0) {
            None
        } else {
            label_position(&circles, mask, plot, scene.measure(&key), 20.0)
        };
        if let Some((x, y)) = at {
            center(scene, x, y - 10.0, &key, ink);
        }
        let members = region
            .members
            .join(if options.ascii { " & " } else { " ∩ " });
        let name = if region.label.is_some() {
            format!("{members}: {name}")
        } else {
            name
        };
        let prefix = if at.is_some() {
            format!("{key}  ")
        } else {
            String::new()
        };
        let mut rows = vec![(format!("{prefix}{name}"), style.text)];
        for item in &region.items {
            rows.push((
                item.text().into(),
                options.color.then_some(item.style.text).flatten(),
            ));
        }
        let swatch = base
            .stroke
            .or(base.fill)
            .or(style.text)
            .unwrap_or_else(|| fade(scene.ink(), 0.60));
        legend.push((swatch, rows));
    }
    let slot = (WIDTH - 2.0 * MARGIN) / 2.0;
    for pair in legend.chunks(2) {
        let mut end = y;
        for (i, (swatch, rows)) in pair.iter().enumerate() {
            let x = MARGIN + i as f32 * slot;
            let mut row_y = y;
            circle(scene, (x + 6.0, y + 10.0), 5.0, None, Some(*swatch), 1.7);
            for (n, (text, color)) in rows.iter().enumerate() {
                let indent = if n == 0 { 0.0 } else { 10.0 };
                for row in wrap(scene, text, slot - 24.0 - indent) {
                    scene.text(x + 18.0 + indent, row_y, &row, *color);
                    row_y += ROW;
                }
            }
            end = end.max(row_y);
        }
        y = end;
    }
    y + 10.0
}

fn bounds(circles: &[Circle]) -> [f64; 4] {
    circles.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |[x, y, u, v], c| {
            [
                x.min(c.x - c.r),
                y.min(c.y - c.r),
                u.max(c.x + c.r),
                v.max(c.y + c.r),
            ]
        },
    )
}

fn label_position(
    circles: &[Circle],
    mask: u32,
    bounds: [f64; 4],
    w: f32,
    h: f32,
) -> Option<(f32, f32)> {
    let (w, h) = (f64::from(w) / 2.0 + 4.0, f64::from(h) / 2.0 + 3.0);
    let [x0, y0, x1, y1] = circles
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .fold(bounds, |[x, y, u, v], (_, c)| {
            [
                x.max(c.x - c.r),
                y.max(c.y - c.r),
                u.min(c.x + c.r),
                v.min(c.y + c.r),
            ]
        });
    let mut best = None;
    let mut clearance = f64::NEG_INFINITY;
    let mut y = y0 + h;
    while y <= y1 - h {
        let mut x = x0 + w;
        while x <= x1 - w {
            let score = circles
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let (dx, dy) = ((x - c.x).abs(), (y - c.y).abs());
                    if mask & (1 << i) != 0 {
                        c.r - (dx + w).hypot(dy + h)
                    } else {
                        (dx - w).max(0.0).hypot((dy - h).max(0.0)) - c.r
                    }
                })
                .fold(f64::INFINITY, f64::min);
            if score >= 0.0 && score > clearance {
                clearance = score;
                best = Some((x as f32, y as f32));
            }
            x += 3.0;
        }
        y += 3.0;
    }
    best
}

struct Arc {
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
}

fn intersection_path(circles: &[Circle], mask: u32) -> Option<String> {
    let sets: Vec<_> = circles
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (mask & (1 << i) != 0).then_some(c))
        .collect();
    let mut arcs = Vec::new();
    for (i, c) in sets.iter().enumerate() {
        let mut angles = vec![0.0, PI, TAU];
        for (j, other) in sets.iter().enumerate() {
            let (dx, dy) = (other.x - c.x, other.y - c.y);
            let d = dx.hypot(dy);
            if i != j && d < c.r + other.r && d > (c.r - other.r).abs() {
                let delta = ((d * d + c.r * c.r - other.r * other.r) / (2.0 * d * c.r))
                    .clamp(-1.0, 1.0)
                    .acos();
                let angle = dy.atan2(dx);
                angles.extend([
                    (angle - delta).rem_euclid(TAU),
                    (angle + delta).rem_euclid(TAU),
                ]);
            }
        }
        angles.sort_by(f64::total_cmp);
        let point = |angle: f64| (c.x + c.r * angle.cos(), c.y + c.r * angle.sin());
        for pair in angles.windows(2).filter(|a| a[1] - a[0] > 1e-8) {
            let (x, y) = point(f64::midpoint(pair[0], pair[1]));
            if sets.iter().all(|s| (x - s.x).hypot(y - s.y) <= s.r + 1e-6) {
                arcs.push(Arc {
                    start: point(pair[0]),
                    end: point(pair[1]),
                    radius: c.r,
                });
            }
        }
    }
    let start = arcs.first()?.start;
    let mut path = format!("M {} {}", start.0, start.1);
    let mut end = start;
    while !arcs.is_empty() {
        let i = arcs
            .iter()
            .position(|arc| (arc.start.0 - end.0).hypot(arc.start.1 - end.1) < 1e-4)?;
        let Arc {
            end: (x, y),
            radius,
            ..
        } = arcs.swap_remove(i);
        write!(path, " A {radius} {radius} 0 0 1 {x} {y}").unwrap();
        end = (x, y);
    }
    path.push_str(" Z");
    Some(path)
}
