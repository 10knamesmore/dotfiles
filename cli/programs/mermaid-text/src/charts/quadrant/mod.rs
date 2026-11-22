use super::shared::plot::{
    Point, Result, delimiter, exact, foreground, keyword, label, list, number, parse_source,
    symbol, text_color, width,
};
use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint, Rgb, Style},
    text::wrap,
};
use rustc_hash::FxHashMap;

mod image;

fn axis_names(s: &str) -> Result<[String; 2]> {
    let (mut quoted, mut escaped) = (false, false);
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && c == '\\' {
            escaped = true;
        } else if c == '"' {
            quoted = !quoted;
        } else if !quoted && s[i..].starts_with("-->") {
            return Ok([label(&s[..i])?, label(&s[i + 3..])?]);
        }
    }
    Ok([label(s)?, String::new()])
}

#[derive(Clone, Copy, Default)]
struct PointStyle {
    colors: Style,
    stroke_width: Option<f64>,
    radius: Option<f64>,
}

impl PointStyle {
    fn overlay(self, over: Self) -> Self {
        Self {
            colors: self.colors.overlay(over.colors),
            stroke_width: over.stroke_width.or(self.stroke_width),
            radius: over.radius.or(self.radius),
        }
    }

    fn marker(self) -> Paint {
        let fill = self.colors.fill;
        let stroke = self
            .stroke_width
            .filter(|&w| w > 0.0)
            .and(self.colors.stroke);
        Paint {
            fg: fill.or(stroke),
            bg: stroke.filter(|_| fill.is_some()),
        }
    }
}

fn styles(s: &str) -> Result<PointStyle> {
    let mut style = PointStyle::default();
    for field in list(s)? {
        let (key, v) = field
            .split_once(':')
            .ok_or_else(|| format!("expected quadrant style key: value, got {field}"))?;
        let v = v.trim();
        let key = key.trim();
        match key {
            "color" | "stroke-color" if !v.is_empty() && !v.chars().any(char::is_whitespace) => {
                if let Some(rgb) = Rgb::parse(v) {
                    if key == "color" {
                        style.colors.fill = Some(rgb);
                    } else {
                        style.colors.stroke = Some(rgb);
                    }
                }
            }
            "radius" | "stroke-width" => {
                let size = number(v.strip_suffix("px").unwrap_or(v))?;
                if size < 0.0 {
                    return Err(format!("unknown or invalid quadrant style: {field}"));
                }
                if key == "stroke-width" {
                    style.stroke_width = Some(size);
                } else {
                    style.radius = Some(size);
                }
            }
            _ => return Err(format!("unknown or invalid quadrant style: {field}")),
        }
    }
    Ok(style)
}

struct Datum {
    name: String,
    class: Option<String>,
    style: PointStyle,
    x: f64,
    y: f64,
}

fn point(s: &str) -> Result<Datum> {
    let open = delimiter(s, '[').ok_or_else(|| format!("unknown quadrant declaration: {s}"))?;
    let prefix = s[..open]
        .trim_end()
        .strip_suffix(':')
        .ok_or("expected point name: [x, y]")?
        .trim_end();
    let (name, class) = match delimiter(prefix, ':') {
        Some(i) => {
            let class = prefix[i..]
                .strip_prefix(":::")
                .ok_or("expected :::class on quadrant point")?;
            if class.is_empty() || class.contains(char::is_whitespace) {
                return Err(format!("invalid quadrant point class: {class}"));
            }
            (label(&prefix[..i])?, Some(class.to_owned()))
        }
        None => (label(prefix)?, None),
    };
    let (values, style) = s[open + 1..]
        .split_once(']')
        .ok_or("expected ] after quadrant coordinates")?;
    let style = if style.trim().is_empty() {
        PointStyle::default()
    } else {
        styles(style)?
    };
    let (x, y) = values
        .split_once(',')
        .ok_or("expected both x and y coordinates")?;
    let (x, y) = (number(x)?, number(y)?);
    if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return Err(format!("quadrant coordinates outside 0..1: [{x}, {y}]"));
    }
    if name.is_empty() {
        return Err("quadrant point needs a label".into());
    }
    Ok(Datum {
        name,
        class,
        style,
        x,
        y,
    })
}

struct Diagram {
    title: String,
    captions: [String; 4],
    x_names: [String; 2],
    y_names: [String; 2],
    data: Vec<Datum>,
    metadata: Vec<String>,
}

fn parse(source: &str) -> Result<Diagram> {
    let plot = parse_source(source)?;
    plot.validate_header(&["quadrantChart"])?;
    let mut title = String::new();
    let mut q = ["Q1", "Q2", "Q3", "Q4"].map(String::from);
    let (mut x_names, mut y_names) = (
        [String::new(), String::new()],
        [String::new(), String::new()],
    );
    let mut data = Vec::new();
    let mut classes = FxHashMap::default();
    for s in &plot.statements {
        let (k, v) = keyword(s);
        match k.to_ascii_lowercase().as_str() {
            "title" => {
                title = if v.starts_with('"') && v.ends_with('"') {
                    label(v)?
                } else {
                    v.to_owned()
                }
            }
            "x-axis" => x_names = axis_names(v)?,
            "y-axis" => y_names = axis_names(v)?,
            "quadrant-1" | "quadrant-2" | "quadrant-3" | "quadrant-4" => {
                q[(k.as_bytes()[9] - b'1') as usize] = label(v)?
            }
            "classdef" => {
                let (class, props) = keyword(v);
                if class.is_empty() {
                    return Err("expected quadrant class name".into());
                }
                classes.insert(class, styles(props)?);
            }
            _ => data.push(point(s)?),
        }
    }
    for datum in &mut data {
        if let Some(class) = datum.class.as_deref().and_then(|name| classes.get(name)) {
            datum.style = class.overlay(datum.style);
        }
    }
    Ok(Diagram {
        title,
        captions: q,
        x_names,
        y_names,
        data,
        metadata: plot.metadata,
    })
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String> {
        let Diagram {
            title,
            captions: q,
            x_names,
            y_names,
            data,
            metadata,
        } = parse(source)?;
        let theme = |key| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let point_fill = theme("quadrantPointFill");
        let point_text = theme("quadrantPointTextFill");
        let point_theme = PointStyle {
            colors: Style {
                fill: point_fill,
                stroke: point_fill,
                text: None,
            },
            stroke_width: None,
            radius: None,
        };
        let point_paints: Vec<_> = data
            .iter()
            .map(|d| {
                if !options.color {
                    return Paint::default();
                }
                point_theme.overlay(d.style).marker()
            })
            .collect();
        let preferred = options.width.clamp(24, 180);
        let margin = y_names
            .iter()
            .map(|s| width(s))
            .max()
            .unwrap_or(0)
            .min(preferred / 5)
            .max(3)
            + 1;
        let digits = data.len().max(1).to_string().len();
        let half = (preferred.saturating_sub(margin + 1) / 2)
            .max(9)
            .max(2 * digits + 3);
        let (left, mid, right) = (margin, margin + half, margin + 2 * half);
        let w = right + 1;
        let captions = q.map(|s| wrap(&s, half - 2));
        let cap_top = captions[0].len().max(captions[1].len());
        let cap_bottom = captions[2].len().max(captions[3].len());
        let xlabels = x_names.map(|s| wrap(&s, half - 1));
        let ylabels = y_names.map(|s| wrap(&s, margin - 1));
        let half_h = (half / 4)
            .clamp(3, 14)
            .max(ylabels[0].len().max(ylabels[1].len()) + 1);
        let y1 = cap_top + 1;
        let ym = y1 + half_h;
        let y0 = ym + half_h;
        let end = y0 + cap_bottom + 1;
        let h = end + 2 + xlabels[0].len().max(xlabels[1].len());
        let mut canvas = Canvas::new(w, h)?;
        let ascii = options.ascii;
        for y in 0..=end {
            for x in [left, mid, right] {
                canvas.put(x, y, symbol(ascii, '│', '|'))?;
            }
        }
        let [tl, tr, bl, br] = options.corners.resolve(ascii, ['┌', '┐', '└', '┘']);
        for (y, chars) in [
            (0, [tl, '┬', tr]),
            (ym, ['├', '┼', '┤']),
            (end, [bl, '┴', br]),
        ] {
            for x in left + 1..right {
                canvas.put(x, y, symbol(ascii, '─', '-'))?;
            }
            for (x, c) in [left, mid, right].into_iter().zip(chars) {
                canvas.put(x, y, symbol(ascii, c, '+'))?;
            }
        }
        for (idx, (start, y)) in [(mid, 1), (left, 1), (left, y0 + 1), (mid, y0 + 1)]
            .into_iter()
            .enumerate()
        {
            let rows = &captions[idx];
            let y = if data.is_empty() {
                if idx < 2 {
                    y + (ym - y - rows.len()) / 2
                } else {
                    ym + (end - ym - rows.len()) / 2
                }
            } else {
                y
            };
            for (j, s) in rows.iter().enumerate() {
                canvas.text(start + (half - width(s)) / 2, y + j, s)?;
            }
        }
        for (v, row) in [("1", y1), ("0.5", ym), ("0", y0)] {
            canvas.text(left - 1 - width(v), row, v)?;
        }
        for (j, s) in ylabels[1].iter().enumerate() {
            canvas.text(left - 1 - width(s), y1 + 1 + j, s)?;
        }
        for (j, s) in ylabels[0].iter().enumerate() {
            canvas.text(left - 1 - width(s), y0 - ylabels[0].len() + j, s)?;
        }
        for (x, v) in [(left, "0"), (mid - 1, "0.5"), (right, "1")] {
            canvas.text(x, end + 1, v)?;
        }
        for (j, s) in xlabels[0].iter().enumerate() {
            canvas.text(left, end + 2 + j, s)?;
        }
        for (j, s) in xlabels[1].iter().enumerate() {
            canvas.text(right + 1 - width(s), end + 2 + j, s)?;
        }

        // Quantize to the width of a printed point key. Each occupied position gets
        // one key, even when several samples share the same terminal cells.
        let unit = digits + 1;
        let mut groups = FxHashMap::default();
        let mut positions = Vec::<(Point, Vec<usize>)>::new();
        let mut ids = Vec::new();
        for (i, d) in data.iter().enumerate() {
            let bins = (half - 2 * digits - 1) / unit;
            let x = if d.x < 0.5 {
                mid - unit - ((0.5 - d.x) * 2.0 * bins as f64).round() as usize * unit
            } else if d.x > 0.5 {
                mid + unit + ((d.x - 0.5) * 2.0 * bins as f64).round() as usize * unit
            } else {
                mid
            };
            let y = y1 + ((1.0 - d.y) * (2 * half_h) as f64).round() as usize;
            let y = if d.y < 0.5 {
                y.max(ym + 1)
            } else if d.y > 0.5 {
                y.min(ym - 1)
            } else {
                ym
            };
            let p = (x, y);
            let id = *groups.entry(p).or_insert_with(|| {
                positions.push((p, Vec::new()));
                positions.len() - 1
            });
            positions[id].1.push(i);
            ids.push(id);
        }
        let mut occupied = vec![false; (2 * half_h + 1) * w];
        for (id, ((x, y), members)) in positions.iter().enumerate() {
            let key = (id + 1).to_string();
            canvas.text(*x, *y, &key)?;
            let paint = point_paints[members[0]];
            if members.iter().all(|&i| point_paints[i] == paint) {
                canvas.paint(*x, *y, key.len(), paint);
            }
            let start = (y - y1) * w + x;
            occupied[start..start + key.len()].fill(true);
        }
        for (id, ((x, y), members)) in positions.iter().enumerate() {
            if members.len() != 1 || *y == ym {
                continue;
            }
            let name = &data[members[0]].name;
            let n = width(name);
            let key_w = (id + 1).to_string().len();
            let row = &mut occupied[(y - y1) * w..(y - y1 + 1) * w];
            for start in [Some(x + key_w + 1), x.checked_sub(n + 1)]
                .into_iter()
                .flatten()
            {
                if start > left
                    && start + n < right
                    && ((*x >= mid && start > mid) || (*x <= mid && start + n <= mid))
                    && !row[start - 1..=start + n].iter().any(|b| *b)
                {
                    canvas.text(start, *y, name)?;
                    canvas.paint(start, *y, n, foreground(point_text));
                    row[start..start + n].fill(true);
                    break;
                }
            }
        }
        let mut output = Vec::new();
        if !title.is_empty() {
            output.push(text_color(title, theme("quadrantTitleFill")));
        }
        output.push(canvas.finish().trim_end().to_string());
        if !data.is_empty() {
            output.push(String::new());
        }
        for ((d, id), paint) in data.iter().zip(ids).zip(point_paints) {
            let class = d
                .class
                .as_ref()
                .map(|s| format!(" [{s}]"))
                .unwrap_or_default();
            let key = (id + 1).to_string();
            let line = format!(
                "{key}  {}{}  ({}, {})",
                d.name,
                class,
                exact(d.x),
                exact(d.y)
            );
            let mut layer = Layer::default();
            layer.paint(0, 0, key.len(), paint);
            layer.paint(key.len() + 2, 0, width(&d.name), foreground(point_text));
            output.push(layer.apply(&line));
        }
        output.extend(metadata);
        Ok(output.join("\n"))
    }
}
