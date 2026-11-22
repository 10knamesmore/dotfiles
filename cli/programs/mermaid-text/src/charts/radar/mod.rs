use super::shared::plot::{
    Point, Result, Strokes, delimiter, exact, foreground, interpolate, keyword, label, list,
    number, parse_source, scale_ticks, series_caption, series_mark, symbol, text_color, width,
};
use crate::{Options, canvas::Canvas, color::Layer, text::wrap};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::f64::consts::TAU;

mod image;

struct Named {
    id: String,
    name: String,
}

fn named(text: &str) -> Result<Named> {
    let text = text.trim();
    let (id, name) = if let Some(i) = delimiter(text, '[') {
        let name = text[i + 1..]
            .strip_suffix(']')
            .ok_or("expected ] after radar label")?;
        (text[..i].trim(), label(name)?)
    } else {
        (text, text.to_string())
    };
    if id.is_empty()
        || id
            .chars()
            .any(|c| c.is_whitespace() || "[]{},:\"".contains(c))
    {
        return Err(format!("invalid radar ID: {id:?}"));
    }
    Ok(Named {
        id: id.to_string(),
        name,
    })
}

enum Sample<'a> {
    Ordered(f64),
    Named(&'a str, f64),
}

fn sample(text: &str) -> Result<Sample<'_>> {
    if let Ok(v) = number(text) {
        Ok(Sample::Ordered(v))
    } else {
        let (key, v) = text
            .split_once(':')
            .or_else(|| text.split_once(char::is_whitespace))
            .ok_or_else(|| format!("invalid radar sample: {text}"))?;
        Ok(Sample::Named(key.trim(), number(v)?))
    }
}

struct Diagram {
    title: String,
    axes: Vec<Named>,
    series: Vec<(String, Vec<f64>)>,
    low: f64,
    high: f64,
    ticks: usize,
    polygon: bool,
    legend: bool,
    clipped: bool,
    metadata: Vec<String>,
}

fn parse(source: &str) -> Result<Diagram> {
    let plot = parse_source(source)?;
    plot.validate_header(&["radar-beta", "radar-beta:"])?;
    let mut axes = Vec::new();
    let mut curves = Vec::new();
    let mut axis_ids = FxHashMap::default();
    let mut curve_ids = FxHashSet::default();
    let mut title = String::new();
    let (mut low, mut high, mut ticks, mut polygon, mut legend) = (0.0, None, 5usize, false, true);
    for s in &plot.statements {
        let (k, v) = keyword(s);
        match k {
            "axis" => {
                for term in list(v.trim_end_matches(','))? {
                    let axis = named(term)?;
                    if axis_ids.insert(axis.id.clone(), axes.len()).is_some() {
                        return Err(format!("duplicate radar axis: {}", axis.id));
                    }
                    axes.push(axis);
                }
            }
            "curve" => {
                for term in list(v.trim_end_matches(','))? {
                    let i = delimiter(term, '{').ok_or("expected curve ID {values}")?;
                    let curve = named(&term[..i])?;
                    if !curve_ids.insert(curve.id.clone()) {
                        return Err(format!("duplicate radar curve: {}", curve.id));
                    }
                    let body = term[i + 1..]
                        .strip_suffix('}')
                        .ok_or("unexpected text after curve values")?;
                    let vals = list(body.trim_end().trim_end_matches(','))?
                        .into_iter()
                        .map(sample)
                        .collect::<Result<Vec<_>>>()?;
                    curves.push((curve, vals));
                }
            }
            "title" => title = label(v)?,
            "min" => low = number(v)?,
            "max" => high = Some(number(v)?),
            "ticks" => {
                ticks = v
                    .parse()
                    .map_err(|_| format!("invalid radar tick count: {v}"))?
            }
            "graticule" => {
                polygon = match v {
                    "polygon" => true,
                    "circle" => false,
                    _ => return Err(format!("unknown radar graticule: {v}")),
                }
            }
            "showLegend" => {
                legend = match v {
                    "true" => true,
                    "false" => false,
                    _ => return Err(format!("expected showLegend true or false, got {v}")),
                }
            }
            _ => return Err(format!("unknown radar declaration: {s}")),
        }
    }
    if axes.is_empty() && !curves.is_empty() {
        return Err("a radar curve requires at least one axis".into());
    }
    if axes.len() > 256 || ticks > 128 {
        return Err("radar layout supports up to 256 axes and 128 rings".into());
    }
    let mut series = Vec::new();
    for (curve, samples) in curves {
        let mut values = vec![None; axes.len()];
        let keyed = matches!(samples.first(), Some(Sample::Named(..)));
        for (i, s) in samples.into_iter().enumerate() {
            let (axis, v) = match s {
                Sample::Ordered(v) if !keyed => (i, v),
                Sample::Named(id, v) if keyed => (
                    *axis_ids
                        .get(id)
                        .ok_or_else(|| format!("unknown radar axis {id} in {}", curve.id))?,
                    v,
                ),
                _ => {
                    return Err(format!(
                        "mixed ordered and named values in curve {}",
                        curve.id
                    ));
                }
            };
            let dest = values
                .get_mut(axis)
                .ok_or_else(|| format!("too many values for radar curve {}", curve.id))?;
            if dest.replace(v).is_some() {
                return Err(format!(
                    "duplicate value for {} in curve {}",
                    axes[axis].id, curve.id
                ));
            }
        }
        let values = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| v.ok_or_else(|| format!("missing {} in curve {}", axes[i].id, curve.id)))
            .collect::<Result<Vec<_>>>()?;
        series.push((curve.name, values));
    }
    let high = high.unwrap_or_else(|| {
        series
            .iter()
            .flat_map(|(_, v)| v.iter().copied())
            .reduce(f64::max)
            .filter(|&v| v > low)
            .unwrap_or(low + 1.0)
    });
    if high <= low || !(high - low).is_finite() {
        return Err("radar max must exceed min by a finite amount".into());
    }
    let clipped = series
        .iter()
        .flat_map(|(_, v)| v)
        .any(|&v| v > high || v < low);
    Ok(Diagram {
        title,
        axes,
        series,
        low,
        high,
        ticks,
        polygon,
        legend,
        clipped,
        metadata: plot.metadata,
    })
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String> {
        let Diagram {
            title,
            axes,
            series,
            low,
            high,
            ticks,
            polygon,
            legend,
            clipped,
            metadata,
        } = parse(source)?;
        let preferred = options.width.clamp(24, 240);
        let panels = series.len().max(1);
        let across = (preferred / 36).clamp(1, 3).min(panels);
        let panel_width = (preferred - (across - 1) * 2) / across;
        let short: Vec<_> = axes
            .iter()
            .enumerate()
            .map(|(i, a)| {
                if width(&a.name) > panel_width / 4 || axes.len() > 18 {
                    format!("[{}]", i + 1)
                } else {
                    a.name.clone()
                }
            })
            .collect();
        let ry = ((panel_width
            .saturating_sub(2 * short.iter().map(|s| width(s)).max().unwrap_or(0) + 4))
            / 4)
        .clamp(5, 14)
        .max(ticks + 2)
        .max(axes.len().div_ceil(3));
        let rx = ry * 2;
        let pads = short.iter().map(|s| width(s)).max().unwrap_or(0) + 1;
        let (cx, cy) = (rx + pads, ry + 2);
        let (w, h) = (2 * cx + 1, 2 * cy + 1);
        if w * h * panels > 4_194_304 {
            return Err("radar panels exceed 4 million terminal cells".into());
        }
        let pos = |i: usize, unit: f64| {
            let a = TAU * i as f64 / axes.len() as f64;
            (
                (cx as f64 + a.sin() * rx as f64 * unit).round() as usize,
                (cy as f64 - a.cos() * ry as f64 * unit).round() as usize,
            )
        };
        let theme = |key: &str| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let axis_color = theme("radar.axisColor").or_else(|| theme("lineColor"));
        let graticule_color = theme("radar.graticuleColor");
        let series_color = |i| theme(&format!("cScale{i}"));
        let mut blocks = Vec::new();
        if !title.is_empty() {
            blocks.push(text_color(title, theme("titleColor")));
        }
        let across = across.min(((preferred + preferred / 8 + 2) / (w + 2)).max(1));
        let mut row_panels = Vec::new();
        let mut group_colors = Layer::default();
        for panel in 0..panels {
            let mut canvas = Canvas::new(w, h)?;
            let panel_x = row_panels.len() * (w + 2);
            let mut plot_colors = BTreeMap::new();
            let mut put = |p @ (x, y), ch, color| -> Result<()> {
                canvas.put(x, y, ch)?;
                if let Some(color) = color {
                    plot_colors.insert(p, color);
                } else {
                    plot_colors.remove(&p);
                }
                Ok(())
            };
            let dot = symbol(options.ascii, '·', '.');
            let mut dotted = |p: Point, q: Point, mark, color| -> Result<()> {
                let samples = (p.0.abs_diff(q.0) as f64 / 2.6)
                    .max(p.1.abs_diff(q.1) as f64 / 1.4)
                    .ceil() as usize;
                for j in 0..=samples {
                    let t = j as f64 / samples.max(1) as f64;
                    put(
                        (
                            interpolate(p.0 as f64, q.0 as f64, t).round() as usize,
                            interpolate(p.1 as f64, q.1 as f64, t).round() as usize,
                        ),
                        mark,
                        color,
                    )?;
                }
                Ok(())
            };
            for ring in 1..=ticks {
                let unit = ring as f64 / ticks as f64;
                if polygon && !axes.is_empty() {
                    for i in 0..axes.len() {
                        dotted(
                            pos(i, unit),
                            pos((i + 1) % axes.len(), unit),
                            dot,
                            graticule_color,
                        )?;
                    }
                } else {
                    let samples = ((ry as f64 * TAU * unit * 0.8).ceil() as usize).max(6);
                    for j in 0..samples {
                        let a = TAU * j as f64 / samples as f64;
                        let p = (
                            (cx as f64 + a.sin() * rx as f64 * unit).round() as usize,
                            (cy as f64 - a.cos() * ry as f64 * unit).round() as usize,
                        );
                        dotted(p, p, dot, graticule_color)?;
                    }
                }
            }
            for i in 0..axes.len() {
                let p = pos(i, 1.0);
                let (dx, dy) = (p.0.abs_diff(cx), p.1.abs_diff(cy));
                let mark = if dx < dy / 2 {
                    symbol(options.ascii, '┆', '|')
                } else if dy < dx / 6 {
                    symbol(options.ascii, '┄', '-')
                } else if (p.0 > cx) == (p.1 > cy) {
                    symbol(options.ascii, '╲', '\\')
                } else {
                    symbol(options.ascii, '╱', '/')
                };
                dotted((cx, cy), p, mark, axis_color)?;
            }
            put((cx, cy), symbol(options.ascii, '┼', '+'), axis_color)?;
            if let Some((_, values)) = series.get(panel) {
                let color = series_color(panel);
                let mut strokes = Strokes::default();
                let points: Vec<_> = values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| pos(i, ((v - low) / (high - low)).clamp(0.0, 1.0)))
                    .collect();
                for i in 0..points.len() {
                    strokes.segment(
                        points[i],
                        points[(i + 1) % points.len()],
                        options.ascii,
                        color,
                        &mut put,
                    )?;
                }
                strokes.paint(&mut put)?;
                let mark = series_mark(panel, options.ascii);
                for point in points {
                    put(point, mark, color)?;
                }
            }
            for ((x, y), fg) in plot_colors {
                group_colors.paint(panel_x + x, y, 1, foreground(Some(fg)));
            }
            let mut axis_label = |x, y, text: &str| -> Result<()> {
                canvas.text(x, y, text)?;
                group_colors.paint(panel_x + x, y, width(text), foreground(axis_color));
                Ok(())
            };
            // Place each label on its own side of the web; spread labels on crowded rows.
            for side in [-1, 1] {
                let mut slots = Vec::new();
                for (i, text) in short.iter().enumerate() {
                    let (x, y) = pos(i, 1.0);
                    if i == 0 || 2 * i == axes.len() {
                        if side == 1 {
                            axis_label(cx - width(text) / 2, if i == 0 { 0 } else { h - 1 }, text)?;
                        }
                    } else if (2 * i < axes.len()) == (side > 0) {
                        slots.push((y, x, text));
                    }
                }
                slots.sort_by_key(|s| s.0);
                let mut last = 0;
                let count = slots.len();
                for (i, (y, x, text)) in slots.into_iter().enumerate() {
                    let row = y.max(last + 1).min(h - 2 - (count - i - 1));
                    last = row;
                    let outer = (rx as f64
                        * (1.0 - ((row as f64 - cy as f64) / ry as f64).powi(2))
                            .max(0.0)
                            .sqrt())
                    .ceil() as usize;
                    let start = if side < 0 {
                        (cx - outer).min(x).saturating_sub(1 + width(text))
                    } else {
                        (cx + outer).max(x) + 1
                    };
                    axis_label(start, row, text)?;
                }
            }
            row_panels.push(canvas.finish());
            if row_panels.len() == across || panel + 1 == panels {
                let rows: Vec<_> = row_panels
                    .iter()
                    .map(|p| p.lines().collect::<Vec<_>>())
                    .collect();
                let mut group = String::new();
                for row in 0..h {
                    let mut line = String::new();
                    for (col, parts) in rows.iter().enumerate() {
                        let text = parts.get(row).copied().unwrap_or("");
                        line.push_str(text);
                        if col + 1 < rows.len() {
                            line.push_str(&" ".repeat(w - width(text) + 2));
                        }
                    }
                    group.push_str(line.trim_end());
                    group.push('\n');
                }
                blocks.push(group_colors.apply(group.trim_end()));
                if legend {
                    for i in (panel + 1 - row_panels.len())..=panel {
                        if let Some((name, values)) = series.get(i) {
                            let caption = series_caption(
                                series_mark(i, options.ascii),
                                name,
                                values,
                                preferred,
                            );
                            let mut colors = Layer::default();
                            colors.paint(0, 0, 1, foreground(series_color(i)));
                            blocks.push(colors.apply(&caption));
                        }
                    }
                }
                row_panels.clear();
                group_colors = Layer::default();
            }
        }
        let scale = if ticks == 0 {
            format!("Scale: {} .. {}", exact(low), exact(high))
        } else {
            format!(
                "Scale: {}",
                scale_ticks(low, high, ticks + 1)
                    .into_iter()
                    .map(|(_, s)| s)
                    .collect::<Vec<_>>()
                    .join("  ")
            )
        };
        blocks.push(text_color(wrap(&scale, preferred).join("\n"), axis_color));
        for (text, axis) in short.iter().zip(&axes) {
            if text != &axis.name {
                blocks.push(text_color(format!("{text} {}", axis.name), axis_color));
            }
        }
        if clipped {
            blocks.push("Values outside the radial scale are clipped at its limits.".into());
        }
        blocks.extend(metadata);
        Ok(blocks.join("\n"))
    }
}
