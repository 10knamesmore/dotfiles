use super::shared::plot::{
    Point, Result, Strokes, delimiter, exact, foreground, keyword, label, list, number,
    parse_source, scale_ticks, series_caption, series_mark, symbol, text_color, width,
};
use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Rgb},
    text::wrap,
};
use std::collections::BTreeMap;

mod image;

const HEADERS: &[&str] = &["xychart", "xychart-beta"];

#[derive(Default)]
struct Axis {
    name: String,
    range: Option<(f64, f64)>,
    categories: Vec<String>,
}

fn axis(text: &str, is_x: bool) -> Result<Axis> {
    if let Some(i) = delimiter(text, '[') {
        if !is_x {
            return Err("y-axis needs a numeric range".into());
        }
        let items = text[i + 1..]
            .strip_suffix(']')
            .ok_or("expected closing ] on x-axis")?;
        return Ok(Axis {
            name: label(&text[..i])?,
            categories: list(items)?.into_iter().map(label).collect::<Result<_>>()?,
            range: None,
        });
    }
    if let Some((left, right)) = text.rsplit_once("-->") {
        // An arrow inside a quoted title does not introduce a range.
        if !text.ends_with('"') {
            let (name, low) = left
                .trim_end()
                .rsplit_once(char::is_whitespace)
                .unwrap_or(("", left));
            return Ok(Axis {
                name: label(name)?,
                range: Some((number(low)?, number(right)?)),
                categories: Vec::new(),
            });
        }
    }
    Ok(Axis {
        name: label(text)?,
        ..Axis::default()
    })
}

struct Series {
    name: String,
    values: Vec<f64>,
    bar: bool,
    index: usize,
}

impl Series {
    fn mark(&self, ascii: bool) -> char {
        if self.bar && self.index < 4 {
            let marks = if ascii {
                ['#', '%', '=', '@']
            } else {
                ['█', '▓', '▒', '▥']
            };
            marks[self.index]
        } else {
            series_mark(if self.bar { self.index + 3 } else { self.index }, ascii)
        }
    }
}

struct Chart {
    title: String,
    x_name: String,
    y_name: String,
    labels: Vec<String>,
    x_range: Option<(f64, f64)>,
    lo: f64,
    hi: f64,
    bars: usize,
    series: Vec<Series>,
}

impl Chart {
    fn ticks(&self) -> Vec<(f64, String)> {
        let mut ticks = scale_ticks(self.lo, self.hi, 5);
        if self.lo.min(self.hi) < 0.0
            && self.lo.max(self.hi) > 0.0
            && !ticks.iter().any(|(value, _)| *value == 0.0)
        {
            ticks.push((0.0, "0".into()));
        }
        ticks
    }

    fn category_offset(&self, index: usize, count: usize, slot: f64) -> f64 {
        if self.x_range.is_some() && count > 1 {
            index as f64 * (self.labels.len() - 1) as f64 * slot / (count - 1) as f64
        } else {
            index as f64 * slot
        }
    }

    fn value_fraction(&self, value: f64) -> f64 {
        ((value - self.lo) / (self.hi - self.lo)).clamp(0.0, 1.0)
    }

    fn clipping_note(&self) -> Option<String> {
        self.series
            .iter()
            .flat_map(|s| &s.values)
            .any(|&v| v < self.lo.min(self.hi) || v > self.lo.max(self.hi))
            .then(|| {
                format!(
                    "Values outside {}..{} are clipped at the axis limits.",
                    exact(self.lo),
                    exact(self.hi)
                )
            })
    }
}

fn palette(options: &Options) -> Vec<Option<Rgb>> {
    options
        .color
        .then(|| options.config.theme_value("xyChart.plotColorPalette"))
        .flatten()
        .and_then(serde_json::Value::as_str)
        .map(|colors| {
            let mut depth = 0usize;
            colors
                .split(move |ch| match ch {
                    '(' => {
                        depth += 1;
                        false
                    }
                    ')' => {
                        depth = depth.saturating_sub(1);
                        false
                    }
                    ',' => depth == 0,
                    _ => false,
                })
                .map(Rgb::parse)
                .collect()
        })
        .unwrap_or_default()
}

fn parse(statements: &[String]) -> Result<Chart> {
    let (mut title, mut x, mut y) = (String::new(), Axis::default(), Axis::default());
    let mut series = Vec::new();
    let (mut bars, mut lines) = (0, 0);
    for statement in statements {
        let (key, value) = keyword(statement);
        match key {
            "title" => title = label(value)?,
            "x-axis" => x = axis(value, true)?,
            "y-axis" => y = axis(value, false)?,
            "bar" | "line" => {
                let bar = key == "bar";
                let i = delimiter(value, '[').ok_or_else(|| format!("expected {key} [values]"))?;
                let mut name = label(&value[..i])?;
                let items = value[i + 1..]
                    .strip_suffix(']')
                    .ok_or("expected ] after series values")?;
                let values = list(items)?
                    .into_iter()
                    .map(number)
                    .collect::<Result<Vec<_>>>()?;
                let index = if bar {
                    bars += 1;
                    bars - 1
                } else {
                    lines += 1;
                    lines - 1
                };
                if name.is_empty() {
                    name = format!("{} {}", if bar { "Bar" } else { "Line" }, index + 1);
                }
                series.push(Series {
                    name,
                    values,
                    bar,
                    index,
                });
            }
            _ => return Err(format!("unknown XY chart declaration: {statement}")),
        }
    }
    let max_n = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    let n = max_n.max(x.categories.len()).max(1);
    let numeric = x.categories.is_empty();
    let (a, b) = x.range.unwrap_or((1.0, n as f64));
    let mut labels = std::mem::take(&mut x.categories);
    if numeric {
        labels = scale_ticks(a, b, n.max(if a != b { 2 } else { 1 }))
            .into_iter()
            .map(|(_, s)| s)
            .collect();
    } else {
        labels.extend((labels.len()..n).map(|i| format!("[{}]", i + 1)));
    }
    let mut extrema = series.iter().flat_map(|s| s.values.iter().copied());
    let first = extrema.next().unwrap_or(0.0);
    let (mut lo, mut hi) = y
        .range
        .unwrap_or_else(|| extrema.fold((first, first), |(l, h), v| (l.min(v), h.max(v))));
    if y.range.is_none() && bars > 0 || lo == hi {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    if lo == hi {
        hi = lo + 1.0;
    }
    if !(hi - lo).is_finite() || lo == hi {
        return Err("XY value range exceeds floating-point resolution".into());
    }
    Ok(Chart {
        title,
        x_name: x.name,
        y_name: y.name,
        labels,
        x_range: x.range,
        lo,
        hi,
        bars,
        series,
    })
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);
    const TITLE_COLORS: &'static [&'static str] = &["xyChart.titleColor"];

    fn render_text(source: &str, options: &Options) -> Result<String> {
        let plot = parse_source(source)?;
        plot.validate_header(HEADERS)?;
        let chart = parse(&plot.statements)?;
        let horizontal = plot.horizontal;
        let theme = |key| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let palette = palette(options);
        let style = |index: usize, series: &Series| {
            (
                series.mark(options.ascii),
                palette.get(index % palette.len().max(1)).copied().flatten(),
            )
        };
        let Chart {
            title,
            x_name,
            y_name,
            labels,
            lo,
            hi,
            bars,
            series,
            ..
        } = &chart;
        let (lo, hi, bars, n) = (*lo, *hi, *bars, labels.len());
        let ascii = options.ascii;
        let preferred = options.width.clamp(20, 200);
        let mut ticks = chart.ticks();
        let tick_w = ticks.iter().map(|(_, s)| width(s)).max().unwrap_or(1);
        let left = if horizontal {
            labels
                .iter()
                .map(|s| width(s))
                .max()
                .unwrap_or(0)
                .min((preferred / 3).max(5))
                .max(tick_w / 2)
                + 2
        } else {
            tick_w + 2
        };
        let min_slot = if horizontal {
            bars.max(1) + 1
        } else {
            (bars + 2).max(4)
        };
        let slot = if horizontal {
            min_slot
        } else {
            min_slot.max(preferred.saturating_sub(left + 1) / n)
        };
        let mut label_key = Vec::new();
        let label_columns = if horizontal { left - 1 } else { slot - 1 };
        let label_rows: Vec<_> = labels
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let rows = wrap(s, label_columns);
                if rows.len() <= 3 {
                    rows
                } else {
                    let key = format!("[{}]", i + 1);
                    label_key.push(format!("{key}  {s}"));
                    wrap(&key, label_columns)
                }
            })
            .collect();
        let label_h = label_rows.iter().map(Vec::len).max().unwrap_or(1);
        let slot = if horizontal { slot.max(label_h) } else { slot };
        let span = if horizontal {
            preferred
                .saturating_sub(left + tick_w.div_ceil(2) + 1)
                .max(12)
        } else {
            4 * ((preferred / 24).clamp(2, 6))
        };
        let (right, bottom) = if horizontal {
            (left + span, n * slot)
        } else {
            (left + n * slot, span)
        };
        let mut canvas = Canvas::new(
            right + if horizontal { tick_w + 1 } else { 1 },
            bottom + if horizontal { 3 } else { label_h + 2 },
        )?;
        let project = |cat: usize, value: f64| -> Point {
            let unit = chart.value_fraction(value);
            let pos = (unit * span as f64).round() as usize;
            if horizontal {
                (left + pos, cat)
            } else {
                (left + cat, bottom - pos)
            }
        };
        let vline = symbol(ascii, '│', '|');
        let hline = symbol(ascii, '─', '-');
        let grid = symbol(ascii, '·', '.');
        let x_axis = theme("xyChart.xAxisLineColor");
        let y_axis = theme("xyChart.yAxisLineColor");
        let grid_color = theme("xyChart.yAxisTickColor");
        let mut cell_colors = BTreeMap::new();
        let mut put = |canvas: &mut Canvas, p @ (x, y), ch, color| -> Result<()> {
            canvas.put(x, y, ch)?;
            if let Some(color) = color {
                cell_colors.insert(p, color);
            } else {
                cell_colors.remove(&p);
            }
            Ok(())
        };
        let mut tick_positions = Vec::new();
        let mut tick_labels = Vec::<(usize, usize)>::new();
        // Prioritise the endpoints and zero before intermediate ticks.
        ticks.sort_by_key(|(v, _)| match *v {
            v if v == lo || v == 0.0 => 0,
            v if v == hi => 1,
            _ => 2,
        });
        for (value, tick) in &ticks {
            let (px, py) = project(0, *value);
            let pos = if horizontal { px } else { py };
            if tick_positions.contains(&pos) {
                continue;
            }
            tick_positions.push(pos);
            let solid = *value == 0.0 || *value == lo;
            let fg = if *value == lo {
                x_axis
            } else if solid {
                y_axis
            } else {
                grid_color
            };
            if horizontal {
                for row in 0..=bottom {
                    if solid || row % 2 == 0 {
                        put(&mut canvas, (px, row), if solid { vline } else { grid }, fg)?;
                    }
                }
                let start = px.saturating_sub(width(tick) / 2);
                let end = start + width(tick);
                if tick_labels.iter().all(|&(a, b)| end < a || start > b) {
                    canvas.text(start, bottom + 1, tick)?;
                    tick_labels.push((start, end));
                }
            } else {
                for col in left..=right {
                    if solid || (col - left) % 4 == 0 {
                        put(&mut canvas, (col, py), if solid { hline } else { grid }, fg)?;
                    }
                }
                canvas.text(left - 1 - width(tick), py, tick)?;
            }
        }
        if horizontal {
            for col in left..=right {
                put(&mut canvas, (col, bottom), hline, y_axis)?;
            }
            put(&mut canvas, (left, bottom), symbol(ascii, '└', '+'), y_axis)?;
        } else {
            for row in 0..=bottom {
                put(&mut canvas, (left, row), vline, y_axis)?;
            }
            put(&mut canvas, (left, bottom), symbol(ascii, '└', '+'), x_axis)?;
        }
        let cat_center =
            |i, count| slot / 2 + chart.category_offset(i, count, slot as f64).round() as usize;
        for (i, rows) in label_rows.iter().enumerate() {
            let cat = cat_center(i, n);
            for (dy, line) in rows.iter().enumerate() {
                if horizontal {
                    canvas.text(left - 1 - width(line), cat - rows.len() / 2 + dy, line)?;
                } else {
                    canvas.text(left + cat - width(line) / 2, bottom + 1 + dy, line)?;
                }
            }
        }
        let bar_width = if horizontal {
            1
        } else {
            let fit = (slot.saturating_sub(2) / bars.max(1)).clamp(1, 5);
            if fit % 2 == 0 { fit - 1 } else { fit }
        };
        let mut strokes = Strokes::bold();
        let mut points = BTreeMap::new();
        let overlap = symbol(ascii, '⊗', '!');
        let mut merged = false;
        let mut put = |p, ch, color| put(&mut canvas, p, ch, color);
        for (index, s) in series
            .iter()
            .enumerate()
            .filter(|(_, s)| s.bar)
            .chain(series.iter().enumerate().filter(|(_, s)| !s.bar))
        {
            let (mark, color) = style(index, s);
            let mut prev = None;
            for (i, &v) in s.values.iter().enumerate() {
                let center = cat_center(i, s.values.len());
                if s.bar {
                    let start = center - (bars * bar_width / 2) + s.index * bar_width;
                    for c in start..start + bar_width {
                        let (x, y) = project(c, v);
                        let (x0, y0) = project(c, 0.0);
                        let (end, baseline) = if horizontal { (x, x0) } else { (y, y0) };
                        if end == baseline {
                            continue;
                        }
                        let mut bar_cell = |pos, mark| {
                            let cell = if horizontal { (pos, y) } else { (x, pos) };
                            put(cell, mark, color)
                        };
                        let low = end.min(baseline);
                        let high = end.max(baseline);
                        for pos in low + 1..high {
                            bar_cell(pos, mark)?;
                        }
                        if mark == '█' {
                            // With solid bars the half cells meet the projected value and baseline.
                            let (low_half, high_half) = if horizontal {
                                ('▐', '▌')
                            } else {
                                ('▄', '▀')
                            };
                            bar_cell(low, low_half)?;
                            bar_cell(high, high_half)?;
                        } else {
                            bar_cell(end, mark)?;
                        }
                    }
                } else {
                    let p = project(center, v);
                    if let Some(a) = prev {
                        strokes.segment(a, p, ascii, color, &mut put)?;
                    }
                    points
                        .entry(p)
                        .and_modify(|(ch, previous_color)| {
                            *ch = overlap;
                            if *previous_color != color {
                                *previous_color = None;
                            }
                            merged = true;
                        })
                        .or_insert((mark, color));
                    prev = Some(p);
                }
            }
        }
        strokes.paint(&mut put)?;
        for (xy, (ch, color)) in points {
            put(xy, ch, color)?;
        }
        for ((x, y), color) in cell_colors {
            canvas.paint(x, y, 1, foreground(Some(color)));
        }
        let mut blocks = Vec::new();
        if !title.is_empty() {
            blocks.push(text_color(title.clone(), theme("xyChart.titleColor")));
        }
        let (above, below) = if horizontal {
            (x_name, y_name)
        } else {
            (y_name, x_name)
        };
        if !above.is_empty() {
            blocks.push(above.clone());
        }
        blocks.push(canvas.finish().trim_end_matches('\n').to_string());
        if !below.is_empty() {
            blocks.push(format!("{}{}", " ".repeat(left), below));
        }
        blocks.extend(label_key);
        if !series.is_empty() {
            blocks.push(String::new());
            let legend_color = theme("xyChart.legendTextColor");
            for (index, s) in series.iter().enumerate() {
                let (mark, color) = style(index, s);
                let swatch = if s.bar {
                    mark.to_string().repeat(3)
                } else {
                    let stroke = symbol(ascii, '━', '-');
                    format!("{stroke}{mark}{stroke}")
                };
                let caption = series_caption(&swatch, &s.name, &s.values, preferred);
                let mut layer = Layer::default();
                for (y, line) in caption.lines().enumerate() {
                    layer.paint(0, y, width(line), foreground(legend_color));
                }
                layer.paint(0, 0, width(&swatch), foreground(color));
                blocks.push(layer.apply(&caption));
            }
        }
        if let Some(note) = chart.clipping_note() {
            blocks.push(note);
        }
        if merged {
            blocks.push(format!("{overlap}  Coincident line markers"));
        }
        Ok(plot.with_metadata(blocks.join("\n")))
    }
}
