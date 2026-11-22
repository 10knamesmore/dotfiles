use crate::{
    canvas::Canvas,
    color::{Paint, Rgb},
    graphics::Scene,
    text::wrap,
};
use std::{borrow::Cow, fmt::Write};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn header<'a>(
    source: &'a str,
    headers: &[&str],
) -> Result<(&'a str, impl Iterator<Item = (usize, &'a str)>), String> {
    let mut lines = source.lines().enumerate();
    let (_, first) = lines.next().ok_or("missing diagram header")?;
    let first = first.trim_start();
    let (kind, suffix) = first.split_at(first.find([' ', '\t', ':', '%']).unwrap_or(first.len()));
    if !headers.contains(&kind) {
        return Err(format!("unsupported diagram header: {kind}"));
    }
    Ok((suffix, lines))
}

pub struct Document<'a> {
    pub statements: Vec<(usize, String)>,
    pub title: Cow<'a, str>,
    accessible_title: &'a str,
    descriptions: Vec<&'a str>,
}

impl Document<'_> {
    pub fn finish(self, drawing: &str) -> String {
        [self.title.as_ref(), self.accessible_title, drawing]
            .into_iter()
            .chain(self.descriptions)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn image_metadata(&self, scene: &Scene) -> (ImageLabel, ImageLabel) {
        let heading = [self.title.as_ref(), self.accessible_title]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        (
            ImageLabel::new(scene, &heading),
            ImageLabel::new(scene, &self.descriptions.join("\n")),
        )
    }
}

pub const IMAGE_PAD: f32 = 24.0;
pub const IMAGE_LINE: f32 = 22.0;

pub struct ImageLabel {
    lines: Vec<(String, f32)>,
    pub width: f32,
}

impl ImageLabel {
    pub fn new(scene: &Scene, text: &str) -> Self {
        Self::measure(scene, text.lines().map(|line| line.replace('\t', " ")))
    }

    pub fn wrapped(scene: &Scene, text: &str, columns: usize) -> Self {
        if text.is_empty() {
            Self::measure(scene, [])
        } else {
            Self::measure(scene, wrap(&text.replace('\t', " "), columns))
        }
    }

    fn measure(scene: &Scene, lines: impl IntoIterator<Item = String>) -> Self {
        let mut width = 0.0_f32;
        let lines = lines
            .into_iter()
            .map(|line| {
                let advance = scene.measure(&line);
                width = width.max(advance);
                (line, advance)
            })
            .collect();
        Self { lines, width }
    }

    pub fn height(&self) -> f32 {
        IMAGE_LINE * self.lines.len() as f32
    }

    pub fn draw(&self, scene: &mut Scene, x: f32, y: f32, color: Option<Rgb>) {
        self.draw_aligned(scene, x, y, color, false);
    }

    pub fn center(&self, scene: &mut Scene, x: f32, y: f32, color: Option<Rgb>) {
        self.draw_aligned(scene, x, y, color, true);
    }

    fn draw_aligned(&self, scene: &mut Scene, x: f32, y: f32, color: Option<Rgb>, centered: bool) {
        for (row, (line, width)) in self.lines.iter().enumerate() {
            let left = if centered { x - width / 2.0 } else { x };
            scene.text(left, y + row as f32 * IMAGE_LINE, line, color);
        }
    }
}

pub fn dashed_image_path(scene: &mut Scene, points: &[(f32, f32)], color: Rgb, width: f32) {
    const DASH: f32 = 5.0;
    const PERIOD: f32 = 9.0;
    let mut path = String::new();
    let mut distance = 0.0_f32;
    for pair in points.windows(2) {
        let [(x, y), (end_x, end_y)] = [pair[0], pair[1]];
        let (dx, dy) = (end_x - x, end_y - y);
        let length = dx.hypot(dy);
        if length == 0.0 {
            continue;
        }
        while distance < length {
            let begin = (distance / length).max(0.0);
            let end = ((distance + DASH) / length).min(1.0);
            if end > begin {
                write!(
                    path,
                    "M {} {} L {} {} ",
                    x + dx * begin,
                    y + dy * begin,
                    x + dx * end,
                    y + dy * end
                )
                .unwrap();
            }
            distance += PERIOD;
        }
        distance = (distance - length).rem_euclid(PERIOD) - PERIOD;
    }
    scene.path(&path, None, Some(color), width);
}

pub fn parse<'a>(
    rest: &'a str,
    lines: impl Iterator<Item = (usize, &'a str)>,
) -> Result<Document<'a>, String> {
    let mut lines = std::iter::once((0, rest)).chain(lines);
    let mut statements = Vec::new();
    let mut title = "";
    let mut accessible_title = "";
    let mut descriptions = Vec::new();
    while let Some((n, raw)) = lines.next() {
        let mut quote = None;
        let line = uncomment(raw, &mut quote);
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if let Some(value) = s.strip_prefix("title")
            && (value.is_empty() || value.starts_with(char::is_whitespace))
        {
            title = value.trim();
        } else if let Some(value) = s
            .strip_prefix("accTitle")
            .and_then(|s| s.trim_start().strip_prefix(':'))
        {
            accessible_title = value.trim();
        } else if let Some(value) = s
            .strip_prefix("accDescr")
            .and_then(|s| s.trim_start().strip_prefix(':'))
        {
            descriptions.push(value.trim());
        } else if let Some(mut value) = s
            .strip_prefix("accDescr")
            .and_then(|s| s.trim_start().strip_prefix('{'))
        {
            loop {
                if let Some((last, tail)) = value.split_once('}') {
                    if !uncomment(tail, &mut None).trim().is_empty() {
                        return Err(format!("unexpected text after accDescr (line {})", n + 1));
                    }
                    descriptions.push(last.trim());
                    break;
                }
                descriptions.push(value.trim());
                value = lines.next().ok_or("unterminated accDescr")?.1;
            }
        } else {
            let mut statement = line.trim_start().to_owned();
            while quote.is_some() {
                let (_, next) = lines
                    .next()
                    .ok_or_else(|| format!("unterminated string on line {}", n + 1))?;
                statement.push('\n');
                statement.push_str(uncomment(next, &mut quote));
            }
            statements.push((n + 1, statement));
        }
    }
    Ok(Document {
        statements,
        title: title.into(),
        accessible_title,
        descriptions,
    })
}

fn uncomment<'a>(line: &'a str, quote: &mut Option<char>) -> &'a str {
    let mut escape = false;
    for (i, c) in line.char_indices() {
        if escape {
            escape = false;
        } else if let Some(q) = *quote {
            if c == '\\' {
                escape = true;
            }
            if c == q {
                *quote = None;
            }
        } else if matches!(c, '"' | '\'') {
            *quote = Some(c);
        } else if line[i..].starts_with("%%") {
            return &line[..i];
        }
    }
    line
}

pub fn quoted(input: &str) -> Result<(String, &str), String> {
    let q = input
        .chars()
        .next()
        .filter(|c| matches!(c, '\'' | '"'))
        .ok_or("expected a quoted string")?;
    let mut value = String::new();
    let mut chars = input.char_indices().skip(1);
    while let Some((i, c)) = chars.next() {
        if c == q {
            return Ok((value, input[i + 1..].trim_start()));
        }
        if c == '\\' {
            let (_, escaped) = chars.next().ok_or("incomplete escape in quoted string")?;
            if escaped != q && escaped != '\\' {
                value.push('\\');
            }
            value.push(escaped);
        } else if c == '\t' {
            value.push(' ');
        } else {
            value.push(c);
        }
    }
    Err("unterminated quoted string".into())
}

pub fn width(s: &str) -> usize {
    s.graphemes(true).map(UnicodeWidthStr::width).sum()
}

/// Union orthogonal strokes before choosing corner and junction glyphs.
pub struct Strokes {
    canvas: Canvas,
    columns: usize,
    mask: Vec<u8>,
}

impl Strokes {
    pub fn new(columns: usize, rows: usize) -> Result<Self, String> {
        let canvas = Canvas::new(columns, rows)?;
        Ok(Self {
            canvas,
            columns,
            mask: vec![0; columns * rows],
        })
    }
    pub fn horizontal(&mut self, a: usize, b: usize, y: usize, color: Option<Rgb>) {
        self.line(true, y, (a, b), false, color);
    }
    pub fn vertical(&mut self, x: usize, a: usize, b: usize, color: Option<Rgb>) {
        self.line(false, x, (a, b), false, color);
    }
    pub fn line(
        &mut self,
        horizontal: bool,
        fixed: usize,
        (a, b): (usize, usize),
        dotted: bool,
        color: Option<Rgb>,
    ) {
        let (start, end) = (a.min(b), a.max(b));
        let (offset, step, next, prev, dots) = if horizontal {
            (fixed * self.columns, 1, 2, 8, 16)
        } else {
            (fixed, self.columns, 4, 1, 32)
        };
        for n in start..end {
            let i = offset + n * step;
            self.mask[i] |= next;
            self.mask[i + step] |= prev;
            if dotted && n > start {
                self.mask[i] |= dots;
            }
        }
        if let Some(color) = color.filter(|_| start < end) {
            let paint = Paint {
                fg: Some(color),
                bg: None,
            };
            if horizontal {
                self.canvas.paint(start, fixed, end - start + 1, paint);
            } else {
                for y in start..=end {
                    self.canvas.paint(fixed, y, 1, paint);
                }
            }
        }
    }
    pub fn draw(self, ascii: bool, crossing: char) -> Result<Canvas, String> {
        self.draw_with_corners(ascii, crossing, ['┌', '┐', '└', '┘'])
    }

    pub fn draw_with_corners(
        mut self,
        ascii: bool,
        crossing: char,
        [tl, tr, bl, br]: [char; 4],
    ) -> Result<Canvas, String> {
        let glyphs = [
            ' ', '│', '─', bl, '│', '│', tl, '├', '─', br, '─', '┴', tr, '┤', '┬', crossing,
        ];
        for (i, &mask) in self.mask.iter().enumerate().filter(|(_, m)| **m != 0) {
            let ch = if mask == 26 {
                if ascii { '.' } else { '┄' }
            } else if mask == 37 {
                if ascii { ':' } else { '┆' }
            } else if ascii {
                match mask & 15 {
                    1 | 4 | 5 => '|',
                    2 | 8 | 10 => '-',
                    _ => '+',
                }
            } else {
                glyphs[(mask & 15) as usize]
            };
            self.canvas.put(i % self.columns, i / self.columns, ch)?;
        }
        Ok(self.canvas)
    }
}
