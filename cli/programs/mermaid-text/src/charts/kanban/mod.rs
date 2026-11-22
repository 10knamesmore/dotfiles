use super::shared::board::{self, Pen, Rows, indent, paint, panel, theme_color, width};
use crate::{
    Options,
    canvas::Canvas,
    color::{Paint, Style},
    text::wrap,
};

mod image;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        render(
            board::body(source, &["kanban"])?,
            options,
            Pen::new(options),
        )
    }
}

#[derive(Default)]
struct Card {
    label: String,
    details: Rows,
}

impl Card {
    fn field(&mut self, key: &str, value: String) {
        match key {
            "label" => self.label = value,
            "assigned" => self.details.push(format!("@{value}")),
            "ticket" => self.details.push(format!("#{value}")),
            _ => self.details.push(format!("{key}: {value}")),
        }
    }

    fn rows(&self, inner: usize) -> Rows {
        std::iter::once(&self.label)
            .chain(&self.details)
            .flat_map(|s| wrap(s, inner))
            .collect()
    }
}

struct Lane {
    indent: usize,
    heading: Card,
    cards: Vec<Card>,
}

fn take_line<'a>(input: &mut &'a str) -> &'a str {
    let (line, next) = input.split_once('\n').unwrap_or((input, ""));
    *input = next;
    line
}

fn horizontal(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

fn scalar(input: &mut &str) -> Result<String, String> {
    if let Some(q @ ('"' | '\'')) = input.chars().next() {
        let mut result = String::new();
        let mut chars = input[1..].chars();
        while let Some(c) = chars.next() {
            if c == q {
                if q == '\'' && chars.as_str().starts_with('\'') {
                    chars.next();
                    result.push('\'');
                    continue;
                }
                *input = chars.as_str();
                return Ok(result);
            }
            if q == '"' && c == '\\' {
                let escape = chars.next().ok_or("incomplete metadata escape")?;
                result.push(match escape {
                    'n' => '\n',
                    't' => '\t',
                    '"' | '\\' | '/' => escape,
                    _ => return Err(format!("unsupported metadata escape: \\{escape}")),
                });
            } else {
                result.push(c);
            }
        }
        return Err("unclosed quoted metadata value".into());
    }
    let end = input.find([',', '\n', '}']).unwrap_or(input.len());
    let value = input[..end].trim();
    if value.is_empty() || value.contains(['[', ']', '{']) {
        return Err("expected a scalar kanban metadata value".into());
    }
    *input = &input[end..];
    Ok(value.into())
}

fn metadata(input: &mut &str, card: &mut Card) -> Result<(), String> {
    *input = &input[2..];
    loop {
        *input = input.trim_start();
        if let Some(after) = input.strip_prefix('}') {
            *input = after;
            return Ok(());
        }
        let end = input
            .find([':', '}', ',', '\n'])
            .ok_or("unclosed kanban metadata")?;
        let key = input[..end].trim();
        if !input[end..].starts_with(':')
            || key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("expected a metadata key followed by ':'".into());
        }
        *input = horizontal(&input[end + 1..]);
        let value = scalar(input)?;
        card.field(key, value);
        *input = horizontal(input);
        if let Some(after) = input.strip_prefix(',') {
            *input = after;
        } else if !input.starts_with(['}', '\n']) {
            return Err("expected ',', newline or '}' after metadata".into());
        }
    }
}

const SHAPES: &[(&str, &str)] = &[
    ("((", "))"),
    ("{{", "}}"),
    ("))", "(("),
    ("-)", "(-"),
    ("(-", "-)"),
    ("[", "]"),
    ("(", ")"),
    (")", "("),
];

fn parse_card(input: &mut &str) -> Result<Card, String> {
    let first = input.lines().next().unwrap_or_default();
    let mut stop = first
        .find(['[', '(', ')', '{', '}', ']', '@'])
        .unwrap_or(first.len());
    if let Some(comment) = first.find("%%") {
        stop = stop.min(comment);
    }
    if first[..stop].ends_with('-') && first[stop..].starts_with(')') {
        stop -= 1;
    }
    let id = input[..stop].trim();
    *input = &input[stop..];
    let label =
        if let Some(&(open, close)) = SHAPES.iter().find(|(open, _)| input.starts_with(open)) {
            *input = &input[open.len()..];
            let start = horizontal(input);
            if let Some(quoted) = start.strip_prefix('"') {
                let (s, tail) = if let Some(md) = quoted.strip_prefix('`') {
                    md.split_once("`\"").ok_or("unclosed Markdown label")?
                } else {
                    quoted.split_once('"').ok_or("unclosed quoted label")?
                };
                *input = horizontal(tail)
                    .strip_prefix(close)
                    .ok_or("expected shape delimiter after quoted label")?;
                s
            } else {
                let (s, tail) = input
                    .split_once(close)
                    .ok_or_else(|| format!("unclosed kanban shape (expected {close})"))?;
                *input = tail;
                s.trim()
            }
        } else {
            id
        };
    if label.is_empty() {
        return Err("expected a kanban label".into());
    }
    let mut card = Card {
        label: label.into(),
        ..Card::default()
    };
    *input = horizontal(input);
    if input.starts_with("@{") {
        metadata(input, &mut card)?;
    }
    if card.label.contains("<br") {
        card.label = card
            .label
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n");
    }
    let tail = take_line(input).trim();
    if !(tail.is_empty() || tail.starts_with("%%")) {
        return Err(format!("unexpected text after kanban node: {tail}"));
    }
    Ok(card)
}

fn parse(mut input: &str) -> Result<Vec<Lane>, String> {
    let mut lanes: Vec<Lane> = Vec::new();
    let mut line = 2;
    while !input.is_empty() {
        let start = input;
        input = horizontal(input);
        let level = indent(&start[..start.len() - input.len()]);
        let text = input.lines().next().unwrap_or_default();
        if text.is_empty() || text.starts_with("%%") {
            take_line(&mut input);
        } else if text.starts_with("::") {
            let decoration = take_line(&mut input).split("%%").next().unwrap().trim();
            let lane = lanes
                .last_mut()
                .ok_or_else(|| format!("line {line}: decoration before a kanban node"))?;
            let node = lane.cards.last_mut().unwrap_or(&mut lane.heading);
            if let Some(class) = decoration
                .strip_prefix(":::")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                node.field("class", class.into());
            } else if let Some(icon) = decoration
                .strip_prefix("::icon(")
                .and_then(|s| s.strip_suffix(')'))
                .filter(|s| !s.is_empty() && !s.contains(['(', ')']))
            {
                node.field("icon", icon.into());
            } else {
                return Err(format!("line {line}: invalid kanban decoration"));
            }
        } else {
            let card = parse_card(&mut input).map_err(|e| format!("kanban line {line}: {e}"))?;
            if let Some(lane) = lanes.last_mut()
                && level > lane.indent
            {
                lane.cards.push(card);
            } else {
                lanes.push(Lane {
                    indent: level,
                    heading: card,
                    cards: vec![],
                });
            }
        }
        line += start[..start.len() - input.len()]
            .bytes()
            .filter(|&c| c == b'\n')
            .count();
    }
    if lanes.is_empty() {
        return Err("kanban needs at least one column".into());
    }
    Ok(lanes)
}

fn lane_style(options: &Options, index: usize) -> Style {
    // Mermaid's section-1 selector uses cScale2.
    let slot = index + 2;
    let scale = (slot < 12)
        .then(|| theme_color(options, &format!("cScale{slot}")))
        .flatten();
    Style {
        fill: scale.or_else(|| theme_color(options, "clusterBkg")),
        stroke: scale.or_else(|| theme_color(options, "clusterBorder")),
        text: (slot < 12)
            .then(|| theme_color(options, &format!("cScaleLabel{slot}")))
            .flatten(),
    }
}

fn render(body: &str, options: &Options, pen: Pen) -> Result<String, String> {
    let lanes = parse(body)?;
    let limit = options.width;
    let per_row = ((limit + 1) / 19).clamp(1, lanes.len());
    let inner = ((limit + 1) / per_row).saturating_sub(5).clamp(8, 48);
    let mut output = String::new();
    let mut plain_size = 0;
    let card = Style {
        stroke: theme_color(options, "nodeBorder"),
        ..Style::default()
    };
    for (band, group) in lanes.chunks(per_row).enumerate() {
        let panels: Vec<_> = group
            .iter()
            .map(|lane| {
                let sections: Vec<_> = std::iter::once(&lane.heading)
                    .chain(&lane.cards)
                    .map(|c| c.rows(inner))
                    .collect();
                let heights = sections.iter().map(Vec::len).collect::<Vec<_>>();
                (panel(&sections, inner.min(14), pen), heights)
            })
            .collect();
        let w = panels.iter().map(|(p, _)| width(&p[0]) + 1).sum::<usize>() - 1;
        let h = panels.iter().map(|(p, _)| p.len()).max().unwrap();
        let mut canvas = Canvas::new(w, h)?;
        let mut x = 0;
        for (i, (p, heights)) in panels.into_iter().enumerate() {
            paint(&mut canvas, x, 0, &p)?;
            let w = width(&p[0]);
            if options.color {
                let style = lane_style(options, band * per_row + i);
                let heading = style.ink();
                let end = heights[0] + 1;
                let border = Paint {
                    fg: style.stroke,
                    bg: None,
                };
                canvas.paint(x, 0, w, border);
                for y in 1..end {
                    canvas.paint(x, y, 1, border);
                    canvas.paint(x + 1, y, w - 2, heading);
                    canvas.paint(x + w - 1, y, 1, border);
                }
                canvas.paint(x, end, w, border);
                let mut y = end;
                for height in heights.into_iter().skip(1) {
                    canvas.paint_box(x, y, w, height + 2, card);
                    y += height + 1;
                }
            }
            x += w + 1;
        }
        if !output.is_empty() {
            output.push_str("\n\n");
            plain_size += 2;
        }
        let band = canvas.finish();
        plain_size += band.len();
        if band.contains('\x1b') {
            for line in band.lines().filter(|s| s.contains('\x1b')) {
                plain_size -= line.len() - crate::color::decode_line(line)?.0.len();
            }
        }
        if plain_size > 16 * 1024 * 1024 {
            return Err("kanban exceeds 16 MiB of output".into());
        }
        output.push_str(&band);
    }
    Ok(output)
}
