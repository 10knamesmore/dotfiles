use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::ops::Bound::{Excluded, Unbounded};

use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rgb {
    channels: [u8; 3],
    alpha: u8,
}

// Terminal colors compare by their visible RGB.
impl PartialEq for Rgb {
    fn eq(&self, other: &Self) -> bool {
        self.channels == other.channels && (self.alpha == 0) == (other.alpha == 0)
    }
}

impl Eq for Rgb {}

impl Rgb {
    const TRANSPARENT: Self = Self::from_rgba(0, 0, 0, 0);

    fn opaque(channels: [u8; 3]) -> Self {
        Self {
            channels,
            alpha: 255,
        }
    }

    pub(crate) const fn from_rgba(r: u8, g: u8, b: u8, alpha: u8) -> Self {
        let channels = if alpha == 0 { [0; 3] } else { [r, g, b] };
        Self { channels, alpha }
    }

    pub(crate) const fn rgba(self) -> [u8; 4] {
        let [r, g, b] = self.channels;
        [r, g, b, self.alpha]
    }

    pub(crate) const fn is_transparent(self) -> bool {
        self.alpha == 0
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        let color = csscolorparser::parse(value.trim()).ok()?;
        if color.a <= 0.0 {
            return Some(Self::TRANSPARENT);
        }
        let [r, g, b, a] = color.to_rgba8();
        Some(Self::from_rgba(r, g, b, a.max(1)))
    }

    pub(crate) fn hex(self) -> String {
        let [r, g, b] = self.channels;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    fn sgr(self, prefix: &str, output: &mut String) {
        let [r, g, b] = self.channels;
        write!(output, "{prefix}{r};{g};{b}m").unwrap();
    }
}

fn split_css(
    input: &str,
    mut is_separator: impl FnMut(char) -> bool,
) -> impl Iterator<Item = &str> {
    let mut depth = 0usize;
    input.split(move |ch| match ch {
        '(' => {
            depth += 1;
            false
        }
        ')' => {
            depth = depth.saturating_sub(1);
            false
        }
        _ => depth == 0 && is_separator(ch),
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub fill: Option<Rgb>,
    pub stroke: Option<Rgb>,
    pub text: Option<Rgb>,
}

impl Style {
    pub(crate) fn css(css: &str) -> Self {
        let mut style = Self::default();
        for declaration in split_css(css, |ch| matches!(ch, ',' | ';')) {
            let Some((name, value)) = declaration.split_once(':') else {
                continue;
            };
            let name = name.trim().to_ascii_lowercase();
            let target = match name.as_str() {
                "fill" | "background" | "background-color" => &mut style.fill,
                "stroke" | "border" | "border-color" => &mut style.stroke,
                "color" | "text-color" => &mut style.text,
                _ => continue,
            };
            let value = value.trim();
            let value = if value.contains("\\,") {
                Cow::Owned(value.replace("\\,", ","))
            } else {
                Cow::Borrowed(value)
            };
            let color = if value.eq_ignore_ascii_case("none") {
                Some(Rgb::TRANSPARENT)
            } else if name == "border" {
                split_css(&value, char::is_whitespace).find_map(Rgb::parse)
            } else {
                Rgb::parse(&value)
            };
            if color.is_some() {
                *target = color;
            }
        }
        style
    }

    pub(crate) fn overlay(self, over: Self) -> Self {
        Self {
            fill: over.fill.or(self.fill),
            stroke: over.stroke.or(self.stroke),
            text: over.text.or(self.text),
        }
    }

    pub(crate) fn ink(self) -> Paint {
        Paint {
            fg: self.text,
            bg: self.fill,
        }
    }

    pub(crate) fn border(self) -> Paint {
        Paint {
            fg: self.stroke,
            bg: self.fill,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Paint {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

const RESET: &str = "\x1b[0m";
const FOREGROUND: &str = "\x1b[38;2;";
const BACKGROUND: &str = "\x1b[48;2;";

impl Paint {
    // Transparency overrides the style cascade while letting the underlying paint show through.
    fn visible(self) -> Self {
        Self {
            fg: self.fg.filter(|color| !color.is_transparent()),
            bg: self.bg.filter(|color| !color.is_transparent()),
        }
    }

    fn write_transition(self, mut previous: Self, output: &mut String) {
        if (previous.fg.is_some() && self.fg.is_none())
            || (previous.bg.is_some() && self.bg.is_none())
        {
            output.push_str(RESET);
            previous = Self::default();
        }
        if let Some(fg) = self.fg.filter(|&fg| Some(fg) != previous.fg) {
            fg.sgr(FOREGROUND, output);
        }
        if let Some(bg) = self.bg.filter(|&bg| Some(bg) != previous.bg) {
            bg.sgr(BACKGROUND, output);
        }
    }
}

// Each key starts a run. `None` restores the terminal's default color, also used before the first key.
type ColorRuns = BTreeMap<usize, Option<Rgb>>;

#[derive(Default)]
struct Row {
    fg: ColorRuns,
    bg: ColorRuns,
}

fn paint_run(runs: &mut ColorRuns, x: usize, end: Option<usize>, color: Rgb) {
    let before = runs.range(..x).next_back().and_then(|(_, &color)| color);
    if let Some(end) = end {
        let after = runs.range(..=end).next_back().and_then(|(_, &color)| color);
        if after == Some(color) {
            runs.remove(&end);
        } else {
            runs.insert(end, after);
        }
    }
    let overwritten: Vec<_> = runs
        .range((Excluded(x), end.map_or(Unbounded, Excluded)))
        .map(|(&column, _)| column)
        .collect();
    for column in overwritten {
        runs.remove(&column);
    }
    if before == Some(color) {
        runs.remove(&x);
    } else {
        runs.insert(x, Some(color));
    }
}

#[derive(Default)]
pub(crate) struct Layer {
    rows: BTreeMap<usize, Row>,
}

impl Layer {
    pub(crate) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(crate) fn paint(&mut self, x: usize, y: usize, width: usize, paint: Paint) {
        let paint = paint.visible();
        if width == 0 || paint == Paint::default() {
            return;
        }
        let row = self.rows.entry(y).or_default();
        let end = x.checked_add(width);
        if let Some(fg) = paint.fg {
            paint_run(&mut row.fg, x, end, fg);
        }
        if let Some(bg) = paint.bg {
            paint_run(&mut row.bg, x, end, bg);
        }
    }

    /// A wide grapheme takes its paint from its first terminal column.
    pub(crate) fn apply(&self, plain: &str) -> String {
        if self.is_empty() {
            return plain.to_owned();
        }
        let mut output = String::with_capacity(plain.len());
        let mut rows = self.rows.iter().peekable();
        for (y, line) in plain.split('\n').enumerate() {
            if y > 0 {
                output.push('\n');
            }
            let Some((_, row)) = rows.next_if(|&(&row_y, _)| row_y == y) else {
                output.push_str(line);
                continue;
            };
            let mut fg = row.fg.iter().peekable();
            let mut bg = row.bg.iter().peekable();
            let mut paint = Paint::default();
            let mut current = Paint::default();
            let mut x = 0usize;
            for grapheme in line.graphemes(true) {
                let width = grapheme.width();
                if width > 0 {
                    while let Some((_, &color)) = fg.next_if(|&(&column, _)| column <= x) {
                        paint.fg = color;
                    }
                    while let Some((_, &color)) = bg.next_if(|&(&column, _)| column <= x) {
                        paint.bg = color;
                    }
                    paint.write_transition(current, &mut output);
                    current = paint;
                    x = x.saturating_add(width);
                }
                output.push_str(grapheme);
            }
            Paint::default().write_transition(current, &mut output);
        }
        output
    }
}

/// A styled range of UTF-8 bytes in the decoded line, with an exclusive end.
#[derive(Serialize)]
pub(crate) struct Span {
    pub start: usize,
    pub end: usize,
    pub fg: Option<String>,
    pub bg: Option<String>,
}

fn sgr_color(mut input: &str) -> Option<(Rgb, &str)> {
    fn channel(input: &mut &str, separator: u8) -> Option<u8> {
        let end = input.bytes().take(4).position(|byte| byte == separator)?;
        let value = &input[..end];
        if (end > 1 && value.starts_with('0')) || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        *input = &input[end + 1..];
        value.parse().ok()
    }

    let r = channel(&mut input, b';')?;
    let g = channel(&mut input, b';')?;
    let b = channel(&mut input, b'm')?;
    Some((Rgb::opaque([r, g, b]), input))
}

fn visit_line(mut line: &str, mut visit: impl FnMut(&str, Paint)) -> Result<(), String> {
    let mut paint = Paint::default();
    loop {
        let end = line.find(char::is_control).unwrap_or(line.len());
        if end > 0 {
            visit(&line[..end], paint);
        }
        line = &line[end..];
        if line.is_empty() {
            return if paint == Paint::default() {
                Ok(())
            } else {
                Err("diagram line ends without an SGR reset".into())
            };
        }
        if let Some(rest) = line.strip_prefix(RESET) {
            paint = Paint::default();
            line = rest;
        } else if let Some(input) = line.strip_prefix(FOREGROUND) {
            let (color, rest) = sgr_color(input).ok_or("invalid foreground SGR")?;
            paint.fg = Some(color);
            line = rest;
        } else if let Some(input) = line.strip_prefix(BACKGROUND) {
            let (color, rest) = sgr_color(input).ok_or("invalid background SGR")?;
            paint.bg = Some(color);
            line = rest;
        } else {
            return Err("unsupported control character in diagram output".into());
        }
    }
}

pub(crate) fn validate_line(line: &str) -> Result<(), String> {
    visit_line(line, |_, _| {})
}

pub(crate) fn decode_line(line: &str) -> Result<(String, Vec<Span>), String> {
    let mut plain = String::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut previous = Paint::default();
    visit_line(line, |text, paint| {
        let start = plain.len();
        plain.push_str(text);
        if paint != Paint::default() {
            if paint == previous {
                spans.last_mut().unwrap().end = plain.len();
            } else {
                spans.push(Span {
                    start,
                    end: plain.len(),
                    fg: paint.fg.map(Rgb::hex),
                    bg: paint.bg.map(Rgb::hex),
                });
            }
        }
        previous = paint;
    })?;
    Ok((plain, spans))
}

pub(crate) fn with_base(text: &str, base: Paint) -> Result<String, String> {
    let base = base.visible();
    if base == Paint::default() {
        return Ok(text.to_owned());
    }
    let mut output = String::with_capacity(text.len());
    for (y, line) in text.split('\n').enumerate() {
        if y > 0 {
            output.push('\n');
        }
        let mut previous = Paint::default();
        visit_line(line, |text, explicit| {
            let paint = Paint {
                fg: explicit.fg.or(base.fg),
                bg: explicit.bg.or(base.bg),
            };
            paint.write_transition(previous, &mut output);
            output.push_str(text);
            previous = paint;
        })?;
        Paint::default().write_transition(previous, &mut output);
    }
    Ok(output)
}
