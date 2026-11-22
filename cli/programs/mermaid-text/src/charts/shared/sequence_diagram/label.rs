use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wrap {
    Fit,
    Always,
    Never,
}

pub struct Label<'a> {
    pub text: Cow<'a, str>,
    width: usize,
    widest_grapheme: usize,
    wrap: Wrap,
}

#[derive(Clone, Copy)]
pub struct Line<'a> {
    pub text: &'a str,
    pub(super) width: usize,
}

impl<'a> Line<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Self {
            text,
            width: text.width(),
        }
    }
}

impl<'a> Label<'a> {
    pub fn new(text: impl Into<Cow<'a, str>>) -> Self {
        let text = text.into();
        Self {
            width: text
                .split('\n')
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or(0),
            widest_grapheme: widest_grapheme(&text),
            text,
            wrap: Wrap::Fit,
        }
    }

    pub fn parse(text: impl Into<Cow<'a, str>>) -> Result<Self, String> {
        let text = text.into();
        let trimmed = text.trim();
        let (wrap, prefix) = label_prefix(trimmed);
        let payload = &trimmed[prefix..];
        let mut pieces = break_lines(payload);
        let first = pieces.next().unwrap_or_default();
        let mut label = if first == payload && !payload.contains(['#', '\t']) {
            match text {
                Cow::Borrowed(text) => Self::new(&text.trim()[prefix..]),
                Cow::Owned(_) => Self::new(payload.to_owned()),
            }
        } else {
            let mut decoded = String::new();
            decoded.push_str(&decode_entities(first)?.replace('\t', "    "));
            for line in pieces {
                decoded.push('\n');
                decoded.push_str(&decode_entities(line)?.replace('\t', "    "));
            }
            Self::new(decoded)
        };
        label.wrap = wrap;
        Ok(label)
    }

    fn limit(&self, limit: usize) -> usize {
        match self.wrap {
            Wrap::Never => usize::MAX,
            Wrap::Fit => limit,
            Wrap::Always => limit.min(40),
        }
    }

    pub(super) fn fitted_width(&self, limit: usize) -> usize {
        self.width.min(self.limit(limit).max(self.widest_grapheme))
    }

    pub fn lines(&self, limit: usize) -> Vec<Line<'_>> {
        let limit = self.limit(limit);
        self.text
            .split('\n')
            .flat_map(|line| wrapped_lines(line, limit))
            .collect()
    }
}

fn label_prefix(text: &str) -> (Wrap, usize) {
    let trimmed = text.strip_prefix(':').unwrap_or(text);
    let Some((prefix, remainder)) = trimmed.split_once(':') else {
        return (Wrap::Fit, 0);
    };
    let wrap = if prefix.eq_ignore_ascii_case("wrap") {
        Wrap::Always
    } else if prefix.eq_ignore_ascii_case("nowrap") {
        Wrap::Never
    } else {
        return (Wrap::Fit, 0);
    };
    (
        wrap,
        text.len() - remainder.trim_start_matches([' ', '\t']).len(),
    )
}

fn widest_grapheme(text: &str) -> usize {
    text.graphemes(true)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

fn wrapped_lines(text: &str, limit: usize) -> Vec<Line<'_>> {
    let limit = limit.max(widest_grapheme(text));
    if text.width() <= limit {
        return vec![Line::new(text)];
    }
    let boundaries: Vec<_> = text
        .split_word_bound_indices()
        .map(|(index, _)| index)
        .collect();
    let mut lines = Vec::new();
    let mut remainder = text;
    while !remainder.is_empty() {
        let start = text.len() - remainder.len();
        let mut boundaries = &boundaries[boundaries.partition_point(|&index| index <= start)..];
        let mut width = 0;
        let mut end = remainder.len();
        let mut word_end = None;
        for (index, grapheme) in remainder.grapheme_indices(true) {
            while let Some((&boundary, remaining)) = boundaries.split_first() {
                if boundary > start + index {
                    break;
                }
                if boundary == start + index {
                    word_end = Some(index);
                }
                boundaries = remaining;
            }
            let grapheme_width = grapheme.width();
            if width + grapheme_width > limit {
                end = word_end.unwrap_or(index);
                break;
            }
            width += grapheme_width;
        }
        let (line, remaining) = remainder.split_at(end);
        lines.push(Line::new(line.trim_end()));
        remainder = remaining.trim_start();
    }
    lines
}

fn break_lines(mut text: &str) -> impl Iterator<Item = &str> {
    let mut finished = false;
    std::iter::from_fn(move || {
        if finished {
            return None;
        }
        for (index, _) in text.match_indices('<') {
            let candidate = &text[index..];
            if candidate
                .get(..3)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<br"))
            {
                let tail = candidate[3..].trim_start();
                let tail = tail.strip_prefix('/').unwrap_or(tail).trim_start();
                if let Some(rest) = tail.strip_prefix('>') {
                    let line = &text[..index];
                    text = rest;
                    return Some(line);
                }
            }
        }
        finished = true;
        Some(text)
    })
}

pub fn entity_at(text: &str) -> Option<(&str, &str)> {
    let body = text.strip_prefix('#')?;
    let len = body.bytes().take_while(u8::is_ascii_alphanumeric).count();
    (len > 0).then_some(())?;
    Some((&body[..len], body[len..].strip_prefix(';')?))
}

pub fn decode_entities(mut text: &str) -> Result<Cow<'_, str>, String> {
    let original = text;
    let mut decoded = None::<String>;
    while let Some(index) = text.find('#') {
        let tail = &text[index..];
        if let Some((entity, remainder)) = entity_at(tail) {
            let character = match entity {
                "amp" => Some('&'),
                "apos" => Some('\''),
                "copy" => Some('©'),
                "gt" => Some('>'),
                "hearts" => Some('♥'),
                "hellip" => Some('…'),
                "infin" => Some('∞'),
                "lt" => Some('<'),
                "mdash" => Some('—'),
                "nbsp" => Some('\u{a0}'),
                "ndash" => Some('–'),
                "quot" => Some('"'),
                "reg" => Some('®'),
                _ => entity.parse::<u32>().ok().and_then(char::from_u32),
            }
            .filter(|ch| !ch.is_control())
            .ok_or_else(|| format!("unsupported text entity `#{entity};`"))?;
            let buffer = decoded.get_or_insert_with(|| {
                let offset = original.len() - text.len();
                original[..offset].to_owned()
            });
            buffer.push_str(&text[..index]);
            buffer.push(character);
            text = remainder;
        } else {
            if let Some(buffer) = &mut decoded {
                buffer.push_str(&text[..=index]);
            }
            text = &tail[1..];
        }
    }
    Ok(match decoded {
        Some(mut decoded) => {
            decoded.push_str(text);
            Cow::Owned(decoded)
        }
        None => Cow::Borrowed(original),
    })
}
