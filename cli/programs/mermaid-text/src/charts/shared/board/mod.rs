pub mod image;
pub mod tree;

use crate::color::{Layer, Paint, Rgb};
use crate::{Options, canvas::Canvas, text::wrap};
use unicode_width::UnicodeWidthStr;

pub type Rows = Vec<String>;

pub fn theme_color(options: &Options, key: &str) -> Option<Rgb> {
    options
        .color
        .then(|| options.config.theme_color(key))
        .flatten()
}

pub fn body<'a>(source: &'a str, headers: &[&str]) -> Result<&'a str, String> {
    let source = source.trim_start();
    let (head, after) = source.split_at(source.find(char::is_whitespace).unwrap_or(source.len()));
    let (same_line, following) = after.split_once('\n').unwrap_or((after, ""));
    let body = if same_line.trim().is_empty() || same_line.trim_start().starts_with("%%") {
        following
    } else {
        after
    };
    if body.lines().count() > 8192 {
        return Err("diagram exceeds 8192 input rows".into());
    }
    if !headers.contains(&head) {
        return Err(format!("unknown board diagram header: {head}"));
    }
    Ok(body)
}

#[derive(Clone, Copy)]
pub struct Pen {
    ascii: bool,
    pub corners: [char; 4],
}

impl Pen {
    pub fn new(options: &Options) -> Self {
        let natural = if options.ascii {
            ['+'; 4]
        } else {
            ['╭', '╮', '╰', '╯']
        };
        Self {
            ascii: options.ascii,
            corners: options.corners.resolve(options.ascii, natural),
        }
    }

    pub fn ch(self, c: char) -> char {
        if !self.ascii {
            return c;
        }
        match c {
            '─' | '━' => '-',
            '│' => '|',
            '╲' => '\\',
            '╱' => '/',
            '▶' => '>',
            '▼' => 'v',
            '·' => '.',
            '●' => '*',
            _ => '+',
        }
    }

    pub fn stroke(self, c: char, length: usize) -> String {
        self.ch(c).to_string().repeat(length)
    }
}

pub fn width(s: &str) -> usize {
    s.width()
}

// Project fragment boundaries onto the rows returned by wrap(), which may
// expand tabs, discard a gap at a break, or split a fragment across rows.
pub fn paint_wrapped<'a>(
    colors: &mut Layer,
    x: usize,
    y: usize,
    lines: &[String],
    fragments: impl IntoIterator<Item = (&'a str, Paint)>,
) {
    let mut source = String::new();
    let mut spans = Vec::new();
    for (text, paint) in fragments {
        let start = source.len();
        source.push_str(&text.replace('\t', "    "));
        if paint != Paint::default() && source.len() > start {
            spans.push((start..source.len(), paint));
        }
    }
    let mut spans = spans.into_iter().peekable();
    let mut offset = 0;
    for (dy, line) in lines.iter().enumerate() {
        if spans.peek().is_none() {
            break;
        }
        let Some(gap) = source[offset..].find(line) else {
            break;
        };
        let start = offset + gap;
        offset = start + line.len();
        while let Some((range, paint)) = spans.peek().filter(|(r, _)| r.start < offset) {
            let left = range.start.max(start);
            let right = range.end.min(offset);
            if left < right {
                colors.paint(
                    x + width(&source[start..left]),
                    y + dy,
                    width(&source[left..right]),
                    *paint,
                );
            }
            if range.end > offset {
                break;
            }
            spans.next();
        }
    }
}

pub fn indent(s: &str) -> usize {
    s.chars().fold(0, |col, c| {
        if c == '\t' {
            (col / 4 + 1) * 4
        } else {
            col + 1
        }
    })
}

pub fn padded(s: &str, columns: usize) -> String {
    format!("{s}{}", " ".repeat(columns.saturating_sub(width(s))))
}

// All rows in a panel occupy the same number of display cells, including its frame.
pub fn panel(groups: &[Rows], min_inner: usize, pen: Pen) -> Rows {
    let inner = groups
        .iter()
        .flatten()
        .map(|s| width(s))
        .max()
        .unwrap_or(0)
        .max(min_inner);
    let rule = |left, mid, right| format!("{left}{}{right}", pen.stroke(mid, inner + 2));
    let [tl, tr, bl, br] = pen.corners;
    let mut rows = vec![rule(tl, '─', tr)];
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            rows.push(if i == 1 {
                rule(pen.ch('┝'), '━', pen.ch('┥'))
            } else {
                rule(pen.ch('├'), '─', pen.ch('┤'))
            });
        }
        for text in group {
            rows.push(format!(
                "{} {} {}",
                pen.ch('│'),
                padded(text, inner),
                pen.ch('│')
            ));
        }
    }
    rows.push(rule(bl, '─', br));
    rows
}

pub fn paint(canvas: &mut Canvas, x: usize, y: usize, rows: &[String]) -> Result<(), String> {
    for (dy, line) in rows.iter().enumerate() {
        canvas.text(x, y + dy, line)?;
    }
    Ok(())
}

#[derive(Default)]
pub struct Captions {
    title: String,
    accessible_title: String,
    description: String,
}

pub fn after_word<'a>(s: &'a str, word: &str) -> Option<&'a str> {
    let prefix = s.get(..word.len())?;
    let rest = &s[word.len()..];
    (prefix.eq_ignore_ascii_case(word) && rest.starts_with(char::is_whitespace))
        .then(|| rest.trim())
}

impl Captions {
    pub fn consume<'a>(
        &mut self,
        text: &str,
        lines: &mut impl Iterator<Item = &'a str>,
    ) -> Result<bool, String> {
        if let Some(title) = after_word(text, "title") {
            self.title = title.into();
            return Ok(true);
        }
        for (key, out) in [
            ("accTitle", &mut self.accessible_title),
            ("accDescr", &mut self.description),
        ] {
            if !text
                .get(..key.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(key))
            {
                continue;
            }
            let rest = text[key.len()..].trim_start();
            if let Some(value) = rest.strip_prefix(':') {
                *out = value.trim().into();
                return Ok(true);
            }
            if key == "accDescr"
                && let Some(start) = rest.strip_prefix('{')
            {
                let mut part = start;
                out.clear();
                loop {
                    let (value, tail) = part
                        .split_once('}')
                        .map_or((part, None), |(v, t)| (v, Some(t)));
                    let value = value.trim();
                    if !value.is_empty() {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(value);
                    }
                    if let Some(tail) = tail {
                        if !tail.trim().is_empty() {
                            return Err("unexpected text after accDescr block".into());
                        }
                        return Ok(true);
                    }
                    part = lines.next().ok_or("unclosed accDescr block")?;
                }
            }
        }
        Ok(false)
    }

    pub fn prepend(&self, graph: String, limit: usize) -> String {
        let title = if self.title.is_empty() {
            &self.accessible_title
        } else {
            &self.title
        };
        let mut rows = Rows::new();
        for s in [title, &self.description] {
            if !s.is_empty() {
                rows.extend(wrap(s, limit.max(10)));
            }
        }
        if !rows.is_empty() && !graph.is_empty() {
            rows.push(String::new());
        }
        rows.push(graph);
        rows.join("\n")
    }
}
