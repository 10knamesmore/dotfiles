use super::{Item, Region};
use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint, Style},
    text::wrap,
};
use std::f64::consts::{PI, TAU};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy)]
pub(super) struct Circle {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

pub(super) fn arrangement(sets: &[&Region], intersections: &[(u32, &Region)]) -> Vec<Circle> {
    let max_size = sets
        .iter()
        .map(|s| s.size.unwrap_or(10.))
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    let mut circles: Vec<_> = sets
        .iter()
        .map(|s| Circle {
            x: 0.0,
            y: 0.0,
            r: (s.size.unwrap_or(10.) / max_size).sqrt().clamp(0.40, 1.0),
        })
        .collect();
    let n = circles.len();
    if n == 1 {
        return circles;
    }
    if n > 3 {
        for (i, c) in circles.iter_mut().enumerate() {
            let angle = TAU * i as f64 / n as f64 - PI / 2.;
            c.x = 0.74 * angle.cos();
            c.y = 0.74 * angle.sin();
            c.r = 1.0;
        }
        return circles;
    }
    let triple = intersections
        .iter()
        .find(|(m, _)| *m == 7)
        .and_then(|(_, r)| {
            let size = r.size.unwrap_or_else(|| {
                sets.iter()
                    .map(|s| s.size.unwrap_or(10.))
                    .fold(f64::INFINITY, f64::min)
                    / 9.
            });
            (size > 0.).then_some(size)
        });
    let pair_count = intersections
        .iter()
        .filter(|(m, r)| m.count_ones() == 2 && r.size != Some(0.))
        .count();
    let distance = |i: usize, j: usize| {
        let mask = 1 << i | 1 << j;
        let a = circles[i].r;
        let b = circles[j].r;
        let size = intersections
            .iter()
            .find(|(m, _)| *m == mask)
            .map(|(_, r)| r.size);
        let overlap = match (size, triple) {
            (Some(Some(0.)), _) => -0.12,
            (Some(Some(v)), _) => {
                let small = sets[i]
                    .size
                    .unwrap_or(10.)
                    .min(sets[j].size.unwrap_or(10.))
                    .max(f64::MIN_POSITIVE);
                let fraction = (v / small).clamp(0.03, 0.88);
                let wanted = fraction * PI * a.min(b).powi(2);
                let (mut lo, mut hi) = ((a - b).abs(), a + b);
                for _ in 0..24 {
                    let d = (lo + hi) / 2.;
                    let x = (a * a - b * b + d * d) / (2. * d);
                    let area = a * a * (x / a).clamp(-1., 1.).acos()
                        + b * b * ((d - x) / b).clamp(-1., 1.).acos()
                        - d * (a * a - x * x).max(0.).sqrt();
                    if area > wanted {
                        lo = d;
                    } else {
                        hi = d;
                    }
                }
                return (lo + hi) / 2.;
            }
            (_, Some(shared)) => {
                let small = sets[i]
                    .size
                    .unwrap_or(10.)
                    .min(sets[j].size.unwrap_or(10.))
                    .max(f64::MIN_POSITIVE);
                (0.4 + 1.45 * (shared / small).sqrt()).clamp(0.44, 1.55)
            }
            (Some(None), _) if pair_count == 3 => 0.24,
            (Some(None), _) => 0.74,
            (None, _) => -0.18,
        };
        a + b - a.min(b) * overlap
    };
    let ab = distance(0, 1);
    if n == 3 {
        let ac = distance(0, 2);
        let bc = distance(1, 2);
        let x = ((ab * ab + ac * ac - bc * bc) / (2. * ab)).clamp(-ac, ac);
        circles[2].x = x;
        circles[2].y = (ac * ac - x * x).sqrt();
    }
    circles[1].x = ab;
    circles
}

struct Plot {
    width: usize,
    height: usize,
    contour: Vec<u8>,
    outlines: Vec<u32>,
    tangent: Vec<char>,
    membership: Vec<u32>,
    clearance: Vec<f64>,
}

impl Plot {
    fn rasterize(circles: &[Circle], width: usize) -> Self {
        let min_x = circles
            .iter()
            .map(|c| c.x - c.r)
            .fold(f64::INFINITY, f64::min);
        let min_y = circles
            .iter()
            .map(|c| c.y - c.r)
            .fold(f64::INFINITY, f64::min);
        let max_x = circles
            .iter()
            .map(|c| c.x + c.r)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = circles
            .iter()
            .map(|c| c.y + c.r)
            .fold(f64::NEG_INFINITY, f64::max);
        let scale = (width as f64 - 2.) / (max_x - min_x);
        let height = ((max_y - min_y) * scale / 2.).ceil() as usize + 2;
        let cells = width * height;
        let mut plot = Self {
            width,
            height,
            contour: vec![0; cells],
            outlines: vec![0; cells],
            tangent: vec![' '; cells],
            membership: vec![0; cells],
            clearance: vec![f64::INFINITY; cells],
        };
        for (i, c) in circles.iter().enumerate() {
            let px = (c.x - min_x) * scale + 0.75;
            let py = (c.y - min_y) * scale / 2. + 0.5;
            let r = c.r * scale;
            for y in 0..height {
                for x in 0..width {
                    let d = (x as f64 + 0.5 - px).hypot((y as f64 + 0.5 - py) * 2.) - r;
                    let cell = y * width + x;
                    if d < 0. {
                        plot.membership[cell] |= 1 << i;
                    }
                    plot.clearance[cell] = plot.clearance[cell].min(d.abs());
                }
            }
            let steps = (TAU * r * 6.).ceil() as usize;
            for step in 0..steps {
                let angle = TAU * step as f64 / steps as f64;
                let (sy, cx) = angle.sin_cos();
                let x = ((px + r * cx) * 2.) as usize;
                let y = ((py + r * sy / 2.) * 4.) as usize;
                let cell = (y / 4) * width + x / 2;
                let dot = [[0, 1, 2, 6], [3, 4, 5, 7]][x % 2][y % 4];
                plot.contour[cell] |= 1 << dot;
                plot.outlines[cell] |= 1 << i;
                plot.tangent[cell] = if cx.abs() < 0.44 {
                    '-'
                } else if sy.abs() < 0.30 {
                    '|'
                } else if sy * cx > 0. {
                    '/'
                } else {
                    '\\'
                };
            }
        }
        plot
    }

    fn paint(&self, canvas: &mut Canvas, sets: &[Style], intersections: &[(u32, Style)]) {
        for (cell, (&inside, &outline)) in self.membership.iter().zip(&self.outlines).enumerate() {
            if inside | outline == 0 {
                continue;
            }
            let mut paint = Paint::default();
            for (i, style) in sets.iter().enumerate() {
                if inside & (1 << i) != 0 {
                    paint.bg = style.fill.or(paint.bg);
                }
                if outline & (1 << i) != 0 {
                    paint.fg = style.stroke.or(style.fill).or(paint.fg);
                }
            }
            for &(mask, style) in intersections {
                if inside & mask == mask {
                    paint.bg = style.fill.or(paint.bg);
                }
                if (inside | outline) & mask == mask && outline & mask != 0 {
                    paint.fg = style.stroke.or(paint.fg);
                }
            }
            canvas.paint(cell % self.width, cell / self.width, 1, paint);
        }
    }

    fn position(&self, mask: u32, cols: usize, rows: usize) -> Option<(usize, usize)> {
        let cols = cols.max(1);
        if cols > self.width || rows > self.height {
            return None;
        }
        let mut best = None;
        let mut score = -1.0;
        for y in 0..=self.height - rows {
            let mut run = 0;
            for x in 0..self.width {
                if (y..y + rows).all(|yy| {
                    let i = yy * self.width + x;
                    self.membership[i] == mask && self.contour[i] == 0
                }) {
                    run += 1;
                } else {
                    run = 0;
                }
                if run >= cols {
                    let center = (y + rows / 2) * self.width + x - cols / 2;
                    let s = self.clearance[center];
                    if s > score {
                        score = s;
                        best = Some((x + 1 - cols, y));
                    }
                }
            }
        }
        best
    }
}

pub(super) fn draw(
    sets: &[&Region],
    intersections: &[(u32, &Region)],
    options: &Options,
) -> Result<String, String> {
    let circles = arrangement(sets, intersections);
    let regions: Vec<_> = sets
        .iter()
        .enumerate()
        .map(|(i, r)| (1 << i, *r))
        .chain(intersections.iter().copied())
        .collect();
    let theme = options.theme_style();
    let region_style = |r: &Region| {
        if options.color {
            r.style
        } else {
            Style::default()
        }
    };
    let default_text = Style {
        text: theme.text,
        ..Style::default()
    };
    let set_styles: Vec<_> = sets
        .iter()
        .map(|r| {
            let style = region_style(r);
            let mut resolved = theme.overlay(style);
            resolved.stroke = style.stroke.or(style.fill).or(theme.stroke);
            resolved
        })
        .collect();
    let mut intersection_styles: Vec<_> = intersections
        .iter()
        .filter(|(_, r)| r.size != Some(0.))
        .map(|&(mask, r)| (mask, region_style(r)))
        .collect();
    intersection_styles.sort_by_key(|(mask, _)| mask.count_ones());
    let paint_regions = set_styles
        .iter()
        .chain(intersection_styles.iter().map(|(_, style)| style))
        .any(|s| s.fill.is_some() || s.stroke.is_some());
    let item_color = |item: &Item| {
        if options.color {
            item.style.text.or(theme.text)
        } else {
            None
        }
    };
    let mut width = options.width.clamp(30, 68);
    if sets.len() == 1 {
        width = width.min(38);
    }
    loop {
        let plot = Plot::rasterize(&circles, width);
        let mut canvas = Canvas::new(width, plot.height)?;
        for (i, &dots) in plot.contour.iter().enumerate().filter(|(_, v)| **v != 0) {
            let ch = if options.ascii {
                match dots {
                    d if d & !0b0000_1001 == 0 => '\'',
                    d if d & !0b1100_0000 == 0 => '.',
                    d if d & !0b0001_1011 == 0 => '-',
                    d if d & !0b1110_0100 == 0 => '_',
                    _ => plot.tangent[i],
                }
            } else {
                char::from_u32(0x2800 + u32::from(dots)).unwrap()
            };
            canvas.put(i % width, i / width, ch)?;
        }
        if paint_regions {
            plot.paint(&mut canvas, &set_styles, &intersection_styles);
        }
        let mut legends = Vec::new();
        let mut retry = false;
        for (idx, (mask, region)) in regions.iter().enumerate() {
            let style = if idx < sets.len() {
                set_styles[idx]
            } else {
                default_text.overlay(region_style(region))
            };
            let text_paint = Paint {
                fg: style.text,
                bg: None,
            };
            let mut name = region.name(options.ascii);
            if let Some(value) = region.size {
                name.push_str(&format!(" ({value})"));
            }
            let mut lines = Vec::new();
            if let [id] = region.members.as_slice()
                && region.label.as_ref().is_some_and(|label| label != id)
            {
                lines.push((id.clone(), style.text));
            }
            let label_width = width.clamp(8, 24);
            lines.extend(
                wrap(&name, label_width)
                    .into_iter()
                    .map(|line| (line, style.text)),
            );
            for item in &region.items {
                let color = item_color(item);
                lines.extend(
                    wrap(item.text(), label_width)
                        .into_iter()
                        .map(|line| (line, color)),
                );
            }
            let w = lines.iter().map(|(line, _)| line.width()).max().unwrap();
            if let Some((x, y)) = plot.position(*mask, w, lines.len()) {
                for (i, (line, fg)) in lines.iter().enumerate() {
                    let cols = line.width();
                    let x = x + (w - cols) / 2;
                    canvas.text(x, y + i, line)?;
                    canvas.paint(x, y + i, cols, Paint { fg: *fg, bg: None });
                }
            } else {
                let marker = format!("{}", idx + 1);
                let at = plot.position(*mask, marker.len(), 1);
                if let Some((x, y)) = at {
                    canvas.text(x, y, &marker)?;
                    canvas.paint(x, y, marker.len(), text_paint);
                } else if region.size != Some(0.) && width < 112 {
                    retry = true;
                    break;
                }
                // Incompatible metric areas can leave an exclusive region with no pixel.
                // State its membership explicitly in the key as well.
                let prefix = if at.is_some() {
                    format!("{marker}  ")
                } else {
                    String::new()
                };
                let membership = region
                    .members
                    .join(if options.ascii { " & " } else { " ∩ " });
                let label = if region.label.is_some() {
                    format!("{membership}: {name}")
                } else {
                    name
                };
                legends.extend(
                    wrap(&format!("{prefix}{label}"), options.width.max(16))
                        .into_iter()
                        .map(|line| ("", line, style.ink())),
                );
                for item in &region.items {
                    let paint = Paint {
                        fg: item_color(item),
                        bg: style.fill,
                    };
                    legends.extend(
                        wrap(item.text(), options.width.saturating_sub(3).max(13))
                            .into_iter()
                            .map(|line| ("   ", line, paint)),
                    );
                }
            }
        }
        if retry {
            width = (width * 3 / 2).min(112);
            continue;
        }
        let mut result = canvas.finish().trim_end().to_owned();
        if !legends.is_empty() {
            let mut colors = Layer::default();
            let mut plain = String::new();
            for (row, (indent, line, paint)) in legends.into_iter().enumerate() {
                if row != 0 {
                    plain.push('\n');
                }
                colors.paint(indent.len(), row, line.width(), paint);
                plain.push_str(indent);
                plain.push_str(&line);
            }
            result.push('\n');
            result.push_str(&colors.apply(&plain));
        }
        return Ok(result);
    }
}
