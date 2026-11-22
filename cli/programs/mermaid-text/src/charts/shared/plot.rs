use crate::color::{Layer, Paint, Rgb};
use std::collections::BTreeMap;
use unicode_width::UnicodeWidthStr;

pub type Result<T> = std::result::Result<T, String>;
pub type Point = (usize, usize);

pub struct PlotSource<'a> {
    pub kind: &'a str,
    pub horizontal: bool,
    pub statements: Vec<String>,
    pub metadata: Vec<String>,
}

impl PlotSource<'_> {
    pub fn validate_header(&self, headers: &[&str]) -> Result<()> {
        if headers.contains(&self.kind) {
            Ok(())
        } else {
            Err(format!("unsupported chart: {}", self.kind))
        }
    }

    pub fn with_metadata(self, mut output: String) -> String {
        for text in self.metadata {
            output.push('\n');
            output.push_str(&text);
        }
        output
    }
}

pub fn parse_source(source: &str) -> Result<PlotSource<'_>> {
    let mut lines = source.trim().lines();
    let header = lines.next().ok_or("missing chart type")?;
    let mut header = header.split_whitespace();
    let kind = header.next().ok_or("missing chart type")?;
    let horizontal = match (kind, header.next()) {
        ("xychart" | "xychart-beta", Some("horizontal")) => true,
        ("xychart" | "xychart-beta", Some("vertical")) | (_, None) => false,
        (_, Some(word)) => return Err(format!("unexpected {kind} header option: {word}")),
    };
    if header.next().is_some() {
        return Err(format!("unexpected text after {kind} header"));
    }
    let mut statements = Vec::new();
    let mut metadata = Vec::new();
    let mut pending = String::new();
    let mut depth = 0i32;
    for line in lines.map(str::trim).filter(|s| !s.is_empty()) {
        if !pending.is_empty() {
            pending.push(' ');
        }
        pending.push_str(line);
        let structured = ["axis", "curve", "bar", "line", "accDescr"]
            .contains(&keyword(&pending).0)
            || (kind.starts_with("xychart") && keyword(&pending).0 == "x-axis");
        if structured {
            let mut quoted = false;
            let mut escaped = false;
            for c in line.chars() {
                if escaped {
                    escaped = false;
                } else if quoted && c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    quoted = !quoted;
                } else if !quoted {
                    match c {
                        '{' | '[' => depth += 1,
                        '}' | ']' => depth -= 1,
                        _ => (),
                    }
                }
            }
            if quoted || depth < 0 {
                return Err(format!("unbalanced delimiters in: {line}"));
            }
            if depth > 0 {
                continue;
            }
        }
        let statement = std::mem::take(&mut pending);
        if let Some(text) = statement
            .strip_prefix("accTitle:")
            .or_else(|| statement.strip_prefix("accDescr:"))
        {
            metadata.push(text.trim().to_owned());
        } else if keyword(&statement).0 == "accDescr" {
            metadata.push(
                keyword(&statement)
                    .1
                    .strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .ok_or("expected accDescr {description}")?
                    .trim()
                    .to_owned(),
            );
        } else {
            statements.push(statement);
        }
    }
    if !pending.is_empty() {
        return Err("unclosed chart declaration".into());
    }
    Ok(PlotSource {
        kind,
        horizontal,
        statements,
        metadata,
    })
}

pub fn keyword(s: &str) -> (&str, &str) {
    s.split_once(char::is_whitespace)
        .map(|(k, v)| (k, v.trim()))
        .unwrap_or((s, ""))
}

pub fn label(s: &str) -> Result<String> {
    let s = s.trim();
    if s.starts_with('"') {
        let text = serde_json::from_str::<String>(s)
            .map_err(|e| format!("invalid quoted label {s}: {e}"))?;
        if text.chars().any(char::is_control) {
            return Err("control character in chart label".into());
        }
        Ok(text)
    } else {
        Ok(s.replace('\t', " "))
    }
}

// Find a delimiter outside quoted labels and nested lists.
pub fn delimiter(s: &str, separator: char) -> Option<usize> {
    let (mut quoted, mut escaped, mut depth) = (false, false, 0i32);
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && c == '\\' {
            escaped = true;
        } else if c == '"' {
            quoted = !quoted;
        } else if !quoted {
            if depth == 0 && c == separator {
                return Some(i);
            }
            match c {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => (),
            }
        }
    }
    None
}

pub fn list(mut s: &str) -> Result<Vec<&str>> {
    let mut parts = Vec::new();
    loop {
        let at = delimiter(s, ',');
        let item = s[..at.unwrap_or(s.len())].trim();
        if item.is_empty() {
            return Err("empty item in chart list".into());
        }
        parts.push(item);
        match at {
            Some(i) => s = &s[i + 1..],
            None => break,
        }
    }
    Ok(parts)
}

pub fn number(s: &str) -> Result<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("expected finite number, got {s:?}"))
}

fn compact(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    if !(1e-4..1e7).contains(&v.abs()) {
        let s = format!("{v:.4e}");
        let (n, e) = s.split_once('e').unwrap();
        format!("{}e{e}", n.trim_end_matches('0').trim_end_matches('.'))
    } else {
        format!("{v:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .into()
    }
}

pub fn exact(v: f64) -> String {
    if v != 0.0 && !(1e-5..1e12).contains(&v.abs()) {
        format!("{v:e}")
    } else {
        v.to_string()
    }
}

pub fn interpolate(a: f64, b: f64, unit: f64) -> f64 {
    a * (1.0 - unit) + b * unit
}

pub fn scale_ticks(a: f64, b: f64, count: usize) -> Vec<(f64, String)> {
    let mut ticks: Vec<_> = (0..count)
        .map(|i| {
            let v = interpolate(a, b, i as f64 / count.saturating_sub(1).max(1) as f64);
            (v, compact(v))
        })
        .collect();
    if ticks
        .windows(2)
        .any(|pair| pair[0].0 != pair[1].0 && pair[0].1 == pair[1].1)
    {
        for (v, s) in &mut ticks {
            *s = exact(*v);
        }
    }
    ticks
}

pub fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub fn foreground(fg: Option<Rgb>) -> Paint {
    Paint { fg, bg: None }
}

pub fn text_color(text: String, rgb: Option<Rgb>) -> String {
    if rgb.is_none() {
        return text;
    }
    let mut layer = Layer::default();
    for (row, line) in text.lines().enumerate() {
        layer.paint(0, row, width(line), foreground(rgb));
    }
    layer.apply(&text)
}

pub fn symbol(ascii: bool, rich: char, plain: char) -> char {
    if ascii { plain } else { rich }
}

pub fn series_mark(i: usize, ascii: bool) -> char {
    let palette = if ascii {
        "ox*+@%&"
    } else {
        "●◆▲■○◇△"
    };
    palette.chars().nth(i).unwrap_or_else(|| {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"[(i - 7) % 62] as char
    })
}

pub fn series_caption(
    mark: impl std::fmt::Display,
    name: &str,
    vals: &[f64],
    columns: usize,
) -> String {
    let prefix = format!("{mark} ");
    let indent = width(&prefix);
    let mut result = format!("{prefix}{name}:");
    let mut used = width(&result);
    for val in vals {
        let value = exact(*val);
        if used + value.len() + 2 > columns && used > indent {
            result.push('\n');
            result.push_str(&" ".repeat(indent));
            used = indent;
        } else {
            result.push_str("  ");
            used += 2;
        }
        result.push_str(&value);
        used += value.len();
    }
    result
}

pub(crate) mod image {
    use std::fmt::Write;
    use unicode_segmentation::UnicodeSegmentation;

    use crate::{Options, color::Rgb, graphics::Scene};

    pub const MARGIN: f32 = 24.0;
    pub const ROW: f32 = 24.0;
    pub type Point = (f32, f32);

    pub fn theme(options: &Options, key: &str) -> Option<Rgb> {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    }

    pub fn palette(index: usize, color: bool) -> Rgb {
        let colors: &[[u8; 3]] = if color {
            &[
                [96, 165, 250],
                [245, 158, 82],
                [73, 186, 163],
                [192, 148, 232],
                [229, 193, 96],
                [230, 135, 164],
                [153, 185, 102],
                [112, 189, 217],
            ]
        } else {
            &[
                [49; 3], [113; 3], [77; 3], [141; 3], [93; 3], [58; 3], [126; 3], [68; 3],
            ]
        };
        let [r, g, b] = colors[index % colors.len()];
        Rgb::from_rgba(r, g, b, 255)
    }

    pub fn fade(color: Rgb, opacity: f32) -> Rgb {
        let [r, g, b, a] = color.rgba();
        Rgb::from_rgba(r, g, b, (f32::from(a) * opacity).round() as u8)
    }

    pub fn contrast(fill: Rgb, paper: Rgb) -> Rgb {
        let [r, g, b, a] = fill.rgba().map(|v| f32::from(v) / 255.0);
        let [pr, pg, pb, pa] = paper.rgba().map(|v| f32::from(v) / 255.0);
        let linear = |fg: f32, bg: f32| {
            let value = fg * a + (bg * pa + 1.0 - pa) * (1.0 - a);
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        let light = 0.2126 * linear(r, pr) + 0.7152 * linear(g, pg) + 0.0722 * linear(b, pb);
        if light < 0.179 {
            Rgb::from_rgba(255, 255, 255, 255)
        } else {
            Rgb::from_rgba(0, 0, 0, 255)
        }
    }

    pub fn center(scene: &mut Scene, x: f32, top: f32, text: &str, color: Option<Rgb>) {
        scene.text(x - scene.measure(text) / 2.0, top, text, color);
    }

    pub fn line(scene: &mut Scene, (x, y): Point, (u, v): Point, stroke: Rgb, width: f32) {
        scene.path(&format!("M {x} {y} L {u} {v}"), None, Some(stroke), width);
    }

    pub fn polygon(
        scene: &mut Scene,
        points: &[Point],
        fill: Option<Rgb>,
        stroke: Option<Rgb>,
        width: f32,
    ) {
        let Some((&(x, y), rest)) = points.split_first() else {
            return;
        };
        if rest.is_empty() {
            return;
        }
        let mut path = format!("M {x} {y}");
        for &(x, y) in rest {
            write!(&mut path, " L {x} {y}").unwrap();
        }
        if points.len() > 2 {
            path.push_str(" Z");
        }
        scene.path(&path, fill, stroke, width);
    }

    pub fn circle(
        scene: &mut Scene,
        (x, y): Point,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<Rgb>,
        width: f32,
    ) {
        if r > 0.0 {
            let (left, right) = (x - r, x + r);
            scene.path(
                &format!("M {left} {y} A {r} {r} 0 1 1 {right} {y} A {r} {r} 0 1 1 {left} {y} Z"),
                fill,
                stroke,
                width,
            );
        }
    }

    pub fn wrap(scene: &Scene, text: &str, max_width: f32) -> Vec<String> {
        let mut rows = Vec::new();
        for paragraph in text.split('\n') {
            let mut row = String::new();
            for word in paragraph.split_whitespace() {
                if !row.is_empty() {
                    let joined = format!("{row} {word}");
                    if scene.measure(&joined) <= max_width {
                        row = joined;
                        continue;
                    }
                    rows.push(std::mem::take(&mut row));
                }
                if scene.measure(word) <= max_width {
                    row.push_str(word);
                    continue;
                }
                for grapheme in word.graphemes(true) {
                    let next = format!("{row}{grapheme}");
                    if !row.is_empty() && scene.measure(&next) > max_width {
                        rows.push(std::mem::take(&mut row));
                        row.push_str(grapheme);
                    } else {
                        row = next;
                    }
                }
            }
            rows.push(row);
        }
        rows
    }
}

// Draw continuous paths on the terminal's 2 × 4 Braille subcell grid.
// ASCII uses the same vertices and chooses a stroke from the segment slope.
#[derive(Default)]
pub struct Strokes {
    dots: BTreeMap<Point, (u8, u8, Option<Rgb>)>,
    samples: usize,
    bold: bool,
}

impl Strokes {
    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Self::default()
        }
    }

    pub fn segment(
        &mut self,
        a: Point,
        b: Point,
        ascii: bool,
        color: Option<Rgb>,
        put: &mut impl FnMut(Point, char, Option<Rgb>) -> Result<()>,
    ) -> Result<()> {
        let (sx, sy) = if ascii { (1, 1) } else { (2, 4) };
        let (x0, y0) = ((a.0 * sx + sx / 2) as isize, (a.1 * sy + sy / 2) as isize);
        let (dx, dy) = (
            ((b.0 as isize) - (a.0 as isize)) * sx as isize,
            ((b.1 as isize) - (a.1 as isize)) * sy as isize,
        );
        let n = dx.abs().max(dy.abs()).max(1);
        self.samples += n as usize + 1;
        if self.samples > 4_194_304 {
            return Err("chart lines exceed 4 million raster samples".into());
        }
        let ch = if dy == 0 || dx.abs() > dy.abs() * 3 {
            '-'
        } else if dx == 0 || dy.abs() > dx.abs() * 3 {
            '|'
        } else if dx.signum() == dy.signum() {
            '\\'
        } else {
            '/'
        };
        for i in 0..=n {
            let x = (x0 + (dx as f64 * i as f64 / n as f64).round() as isize) as usize;
            let y = (y0 + (dy as f64 * i as f64 / n as f64).round() as isize) as usize;
            if ascii {
                put((x, y), ch, color)?;
            } else {
                const BITS: [[u8; 4]; 2] = [[1, 2, 4, 64], [8, 16, 32, 128]];
                let (mask, weight, previous) = self.dots.entry((x / 2, y / 4)).or_default();
                let (x, y) = (x % 2, y % 4);
                *mask |= BITS[x][y];
                if self.bold {
                    // Widen perpendicular to the path, within the rasterized cell.
                    *weight |= if dx.abs() >= dy.abs() {
                        BITS[x][y ^ 1]
                    } else {
                        BITS[x ^ 1][y]
                    };
                }
                *previous = color;
            }
        }
        Ok(())
    }
    pub fn paint(self, put: &mut impl FnMut(Point, char, Option<Rgb>) -> Result<()>) -> Result<()> {
        for (xy, (mask, weight, color)) in self.dots {
            let ch = match (self.bold, mask) {
                (true, 0x24) => '━',
                (true, 0xb8) => '┃',
                _ => char::from_u32(0x2800 + u32::from(mask | weight)).unwrap(),
            };
            put(xy, ch, color)?;
        }
        Ok(())
    }
}
