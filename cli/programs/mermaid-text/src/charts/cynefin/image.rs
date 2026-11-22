use super::{DOMAINS, domain_index, parse};
use crate::{
    Options,
    charts::shared::plot::image::{
        MARGIN, Point, ROW, center, fade, palette, polygon, theme, wrap,
    },
    graphics::Scene,
};
use std::fmt::Write;

const WIDTH: f32 = 824.0;

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let diagram = parse(source)?;
    let theme = |key: &str| theme(options, key);
    let lab = theme("cynefin.labelColor").or(options.theme_style().text);
    let body = theme("cynefin.textColor");
    let boundary = theme("cynefin.boundaryColor")
        .or(options.theme_line())
        .unwrap_or_else(|| fade(scene.ink(), 0.52));
    let cliff = theme("cynefin.cliffColor").unwrap_or_else(|| scene.ink());
    let arrow = theme("cynefin.arrowColor")
        .or(options.theme_line())
        .unwrap_or_else(|| scene.ink());
    let fills = DOMAINS.map(|domain| {
        let i = domain_index(domain.keyword).unwrap();
        theme(&format!("cynefin.{}Bg", domain.keyword)).unwrap_or_else(|| {
            fade(
                palette(i, options.color),
                if i == 4 {
                    0.29
                } else {
                    0.13 + (i % 2) as f32 * 0.10
                },
            )
        })
    });
    let separator = if options.ascii { " -> " } else { " → " };
    let panels: [_; 5] = std::array::from_fn(|i| {
        let domain = &DOMAINS[i];
        let mut rows = Vec::new();
        if !domain.decisions.is_empty() {
            rows.extend(wrap(scene, &domain.decisions.join(separator), 338.0));
        }
        rows.push(domain.practice.into());
        if !diagram.regions[i].items.is_empty() {
            rows.push(String::new());
            for item in &diagram.regions[i].items {
                rows.extend(wrap(scene, item, if i == 4 { 264.0 } else { 338.0 }));
            }
        }
        rows
    });
    let mut top = MARGIN;
    for text in [diagram.title, diagram.accessible_title]
        .into_iter()
        .filter(|text| !text.is_empty())
    {
        for row in wrap(scene, text, WIDTH - 2.0 * MARGIN) {
            center(scene, WIDTH / 2.0, top, &row, theme("titleColor"));
            top += ROW;
        }
    }
    let panel_height =
        |pair: &[Vec<String>]| pair.iter().map(Vec::len).max().unwrap() as f32 * ROW + 92.0;
    let y_top = top + panel_height(&panels[..2]);
    let y_bottom = y_top + (panels[4].len() + 1) as f32 * ROW + 54.0;
    let middle = f32::midpoint(y_top, y_bottom);
    let bottom = y_bottom + panel_height(&panels[2..4]);
    let (left, right, x) = (MARGIN, WIDTH - MARGIN, WIDTH / 2.0);
    let (xl, xr) = (x - 153.0, x + 153.0);
    let (yt, yb) = (y_top + 23.0, y_bottom - 23.0);
    let bend = x + 27.0;
    let paths = [
        format!(
            "M {left} {top} H {x} C {bend} {top} {bend} {y_top} {x} {y_top} L {xl} {yt} V {middle} H {left} Z"
        ),
        format!(
            "M {x} {top} H {right} V {middle} H {xr} V {yt} L {x} {y_top} C {bend} {y_top} {bend} {top} {x} {top} Z"
        ),
        format!("M {left} {middle} H {xl} V {yb} L {x} {y_bottom} V {bottom} H {left} Z"),
        format!("M {xr} {middle} H {right} V {bottom} H {x} V {y_bottom} L {xr} {yb} Z"),
        format!("M {x} {y_top} L {xr} {yt} V {yb} L {x} {y_bottom} L {xl} {yb} V {yt} Z"),
    ];
    for (path, fill) in paths.iter().zip(fills) {
        scene.path(path, Some(fill), Some(boundary), 1.5);
    }
    let mut drop = format!("M {x} {y_bottom}");
    for i in 1..12 {
        let px = x + if i % 2 == 0 { -4.0 } else { 4.0 };
        let py = y_bottom + i as f32 * (bottom - y_bottom) / 12.0;
        write!(&mut drop, " L {px} {py}").unwrap();
    }
    write!(&mut drop, " L {x} {bottom}").unwrap();
    scene.path(&drop, None, Some(cliff), 2.0);

    let near = [y_top - 40.0, y_bottom + 33.0];
    let mut transitions: Vec<(usize, usize, Vec<&str>)> = Vec::new();
    for (from, region) in diagram.regions.iter().enumerate() {
        for (target, text) in &region.outgoing {
            let to = domain_index(target.keyword).unwrap();
            let caption = text.as_deref().unwrap_or_default();
            if let Some((_, _, captions)) = transitions
                .iter_mut()
                .find(|(a, b, _)| *a == from && *b == to)
            {
                captions.push(caption);
            } else {
                transitions.push((from, to, vec![caption]));
            }
        }
    }
    let mut badges = Vec::new();
    for (i, &(from, to, _)) in transitions.iter().enumerate() {
        let side = |domain: usize| if domain.is_multiple_of(2) { -1.0 } else { 1.0 };
        let center_link = from == 4 || to == 4;
        let diagonal = !center_link && from ^ to == 3;
        let (start, end) = if center_link {
            let corner = if from == 4 { to } else { from };
            let s = side(corner);
            let inner = (x + s * 137.0, [yt + 10.0, yb - 10.0][corner / 2]);
            let outer = (x + s * 173.0, near[corner / 2]);
            if from == 4 {
                (inner, outer)
            } else {
                (outer, inner)
            }
        } else if from / 2 == to / 2 {
            (
                (x + side(from) * 114.0, near[from / 2]),
                (x + side(to) * 114.0, near[to / 2]),
            )
        } else if from % 2 == to % 2 {
            let x = x + side(from) * 205.0;
            ((x, near[from / 2]), (x, near[to / 2]))
        } else {
            (
                (x + side(from) * 81.0, near[from / 2]),
                (x + side(to) * 81.0, near[to / 2]),
            )
        };
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let length = dx.hypot(dy);
        let normal = (-dy / length, dx / length);
        let gap = if center_link { 8.0 } else { 11.0 };
        let shifted = |(x, y): Point| (x + normal.0 * gap, y + normal.1 * gap);
        let ((sx, sy), (ex, ey)) = (shifted(start), shifted(end));
        let (path, before_tip, badge) = if diagonal {
            let cx = x + side(from) * 264.0;
            (
                format!("M {sx} {sy} C {cx} {sy} {cx} {ey} {ex} {ey}"),
                (cx, ey),
                ((sx + 6.0 * cx + ex) / 8.0, (sy + ey) / 2.0),
            )
        } else {
            let (cx, cy) = (
                (sx + ex) / 2.0 + normal.0 * 8.0,
                (sy + ey) / 2.0 + normal.1 * 8.0,
            );
            let t: f32 = if center_link { 0.54 } else { 0.40 };
            let (a, b, c) = ((1.0 - t).powi(2), 2.0 * t * (1.0 - t), t * t);
            (
                format!("M {sx} {sy} Q {cx} {cy} {ex} {ey}"),
                (cx, cy),
                (a * sx + b * cx + c * ex, a * sy + b * cy + c * ey),
            )
        };
        scene.path(&path, None, Some(arrow), 2.1);
        head(scene, before_tip, (ex, ey), arrow);
        badges.push((badge.0, badge.1, (i + 1).to_string()));
    }
    for (i, (rows, (px, py))) in panels
        .iter()
        .zip([
            (x - 194.0, top + 18.0),
            (x + 194.0, top + 18.0),
            (x - 194.0, y_bottom + 55.0),
            (x + 194.0, y_bottom + 55.0),
            (x, yt + 5.0),
        ])
        .enumerate()
    {
        center(scene, px, py, DOMAINS[i].heading, lab);
        for (n, row) in rows.iter().enumerate() {
            center(scene, px, py + (n + 1) as f32 * ROW, row, body);
        }
    }
    for (x, y, key) in badges {
        let w = scene.measure(&key) + 10.0;
        scene.rect(
            x - w / 2.0,
            y - 10.0,
            w,
            20.0,
            5.0,
            Some(scene.paper()),
            Some(arrow),
        );
        center(scene, x, y - 10.0, &key, body);
    }
    let mut y = bottom + 16.0;
    let slot = (WIDTH - 2.0 * MARGIN) / 2.0;
    for (pair_index, pair) in transitions.chunks(2).enumerate() {
        let mut end = y;
        for (offset, (from, to, captions)) in pair.iter().enumerate() {
            let x = MARGIN + offset as f32 * slot;
            let key = pair_index * 2 + offset + 1;
            let from = DOMAINS[*from].heading;
            let to = DOMAINS[*to].heading;
            let heading = format!("{key}  {from}{separator}{to}");
            let mut row = y;
            for text in std::iter::once(heading.as_str())
                .chain(captions.iter().copied())
                .filter(|s| !s.is_empty())
            {
                for line in wrap(scene, text, slot - 18.0) {
                    scene.text(x, row, &line, body);
                    row += ROW;
                }
            }
            end = end.max(row);
        }
        y = end;
    }
    for text in diagram.description {
        for row in wrap(scene, text, WIDTH - 2.0 * MARGIN) {
            scene.text(MARGIN, y, &row, body);
            y += ROW;
        }
    }
    scene.resize(WIDTH, y + MARGIN)
}

fn head(scene: &mut Scene, (cx, cy): Point, (x, y): Point, color: crate::color::Rgb) {
    let (dx, dy) = (x - cx, y - cy);
    let len = dx.hypot(dy);
    let (dx, dy) = (dx / len, dy / len);
    let (u, v) = (x - 10.0 * dx, y - 10.0 * dy);
    polygon(
        scene,
        &[
            (x, y),
            (u - 4.5 * dy, v + 4.5 * dx),
            (u + 4.5 * dy, v - 4.5 * dx),
        ],
        Some(color),
        None,
        0.0,
    );
}
