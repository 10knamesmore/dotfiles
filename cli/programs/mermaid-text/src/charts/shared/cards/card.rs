use std::{borrow::Cow, fmt::Write};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint, Style},
};

use super::{CardStyle, Diagram, Node};

const MAX_CARD_WIDTH: usize = 132;

pub(super) struct Card {
    rows: Vec<Row>,
    pub(super) width: usize,
    frame: Frame,
    color: Style,
}

enum Row {
    Text { text: String, centered: bool },
    Divider,
}

#[derive(Default)]
pub(super) struct Picture {
    plain: String,
    colors: Layer,
    rows: usize,
}

impl Picture {
    pub(super) fn paint(&mut self, x: usize, width: usize, paint: Paint) {
        self.colors.paint(x, self.rows, width, paint);
    }

    pub(super) fn line(&mut self, line: &str) {
        let _ = writeln!(self.plain, "{line}");
        self.rows += 1;
    }

    pub(super) fn finish(self) -> String {
        self.colors.apply(&self.plain)
    }
}

struct Frame {
    top: (char, char),
    divider: (char, char),
    bottom: (char, char),
    horizontal: char,
    vertical: char,
}

impl Frame {
    fn new(style: CardStyle, options: &Options) -> Self {
        let ascii = options.ascii;
        let heavy = style.heavy == Some(true);
        let dashed = style.dashed == Some(true);
        let rounded = style.rounded == Some(true);
        let (corners, divider) = match (ascii, heavy, rounded) {
            (true, _, _) => (['+'; 4], ('+', '+')),
            (_, true, _) => (['┏', '┓', '┗', '┛'], ('┣', '┫')),
            (_, _, true) => (['╭', '╮', '╰', '╯'], ('├', '┤')),
            _ => (['┌', '┐', '└', '┘'], ('├', '┤')),
        };
        let [tl, tr, bl, br] = options.corners.resolve(ascii, corners);
        let (horizontal, vertical) = match (ascii, heavy, dashed) {
            (true, _, true) => ('.', ':'),
            (true, true, _) => ('=', '|'),
            (true, _, _) => ('-', '|'),
            (_, true, true) => ('┅', '┇'),
            (_, _, true) => ('┄', '┆'),
            (_, true, _) => ('━', '┃'),
            _ => ('─', '│'),
        };
        Self {
            top: (tl, tr),
            divider,
            bottom: (bl, br),
            horizontal,
            vertical,
        }
    }
}

impl Card {
    pub(super) fn new(
        diagram: &Diagram<'_>,
        index: usize,
        width: usize,
        options: &Options,
    ) -> Self {
        let width = width.min(MAX_CARD_WIDTH);
        let node = &diagram.nodes[index];
        let mut card = Self::title(diagram, node, width, options, false);
        for (items, prefix) in [
            (&node.fields, ""),
            (&node.methods, ""),
            (&node.notes, "Note: "),
        ] {
            if !items.is_empty() {
                card.rows.push(Row::Divider);
                for text in items {
                    if prefix.is_empty() {
                        card.text(text, width, false);
                    } else {
                        card.note(text, width);
                    }
                }
            }
        }
        card
    }

    pub(super) fn reference(
        diagram: &Diagram<'_>,
        index: usize,
        width: usize,
        options: &Options,
    ) -> Self {
        Self::title(
            diagram,
            &diagram.nodes[index],
            width.min(MAX_CARD_WIDTH),
            options,
            true,
        )
    }

    fn title(
        diagram: &Diagram<'_>,
        node: &Node<'_>,
        width: usize,
        options: &Options,
        reference: bool,
    ) -> Self {
        let mut card = Self {
            width: 7,
            rows: Vec::new(),
            frame: Frame::new(node.style, options),
            color: if options.color {
                options.theme_style().overlay(node.style.color)
            } else {
                Style::default()
            },
        };
        if let Some(namespace) = node.namespace {
            let mut path = Vec::new();
            let mut scope = Some(namespace);
            while let Some(index) = scope {
                let namespace = &diagram.namespaces[index];
                path.push(namespace.label.unwrap_or(namespace.name));
                scope = namespace.parent;
            }
            path.reverse();
            card.text(&display_text(&path.join(" / ")), width, true);
        }
        if !reference {
            for annotation in &node.annotations {
                card.text(&format!("<<{annotation}>>"), width, true);
            }
        }
        let mut name = node.name.written.to_owned();
        if let Some(generic) = node.generic.filter(|_| !reference) {
            let _ = write!(name, "<{generic}>");
        }
        if let Some(alias) = node.alias.filter(|_| !reference) {
            card.text(&display_text(alias), width, true);
            card.text(&format!("[{name}]"), width, true);
        } else {
            card.text(&name, width, true);
        }
        card
    }

    fn text(&mut self, text: &str, width: usize, centered: bool) {
        for text in wrap(text, width.saturating_sub(4).max(8)) {
            self.width = self.width.max(columns(&text) + 4);
            self.rows.push(Row::Text { text, centered });
        }
    }

    fn note(&mut self, text: &str, width: usize) {
        let text = display_text(text);
        let mut lines = text.split("\\n");
        if let Some(first) = lines.next() {
            self.text(&format!("Note: {first}"), width, false);
        }
        for line in lines {
            self.text(line, width, false);
        }
    }

    pub(super) fn height(&self) -> usize {
        self.rows.len() + 2
    }

    pub(super) fn side_port(&self, y: usize) -> bool {
        matches!(self.rows.get(y - 1), Some(Row::Text { .. }))
    }

    fn line(&self, row: usize, port: Option<(usize, char)>) -> (String, Option<(usize, usize)>) {
        let right_edge = self.width - 1;
        let (left, right) = if row == 0 {
            self.frame.top
        } else if row == self.height() - 1 {
            self.frame.bottom
        } else {
            match &self.rows[row - 1] {
                Row::Divider => self.frame.divider,
                Row::Text { text, centered } => {
                    let width = columns(text);
                    let remaining = self.width - 2 - width;
                    let left = if *centered { remaining / 2 } else { 1 };
                    let right = remaining - left;
                    let bar = self.frame.vertical;
                    return (
                        format!("{bar}{:left$}{text}{:right$}{bar}", "", ""),
                        Some((1 + left, width)),
                    );
                }
            }
        };
        let line = (0..self.width)
            .map(|x| match port {
                Some((column, glyph)) if column == x => glyph,
                _ if x == 0 => left,
                _ if x == right_edge => right,
                _ => self.frame.horizontal,
            })
            .collect();
        (line, None)
    }

    fn paint_line(
        &self,
        label: Option<(usize, usize)>,
        mut paint: impl FnMut(usize, usize, Paint),
    ) {
        let border = self.color.border();
        if let Some((x, width)) = label {
            paint(0, 1, border);
            paint(
                1,
                self.width - 2,
                Paint {
                    fg: None,
                    bg: self.color.fill,
                },
            );
            paint(x, width, self.color.ink());
            paint(self.width - 1, 1, border);
        } else {
            paint(0, self.width, border);
        }
    }

    pub(super) fn paint(&self, canvas: &mut Canvas, x: usize, y: usize) -> Result<(), String> {
        for row in 0..self.height() {
            let (line, label) = self.line(row, None);
            canvas.text(x, y + row, &line)?;
            self.paint_line(label, |offset, width, paint| {
                canvas.paint(x + offset, y + row, width, paint)
            });
        }
        Ok(())
    }

    pub(super) fn append(
        &self,
        picture: &mut Picture,
        x: usize,
        port: Option<(usize, usize, char)>,
    ) {
        for row in 0..self.height() {
            let port = port.and_then(|(y, x, glyph)| (row == y).then_some((x, glyph)));
            let (line, label) = self.line(row, port);
            self.paint_line(label, |offset, width, paint| {
                picture.paint(x + offset, width, paint)
            });
            picture.line(&format!("{:x$}{line}", ""));
        }
    }

    pub(super) fn picture(&self) -> String {
        let mut picture = Picture::default();
        self.append(&mut picture, 0, None);
        picture.finish()
    }

    pub(super) fn paint_image(
        &self,
        scene: &mut crate::graphics::Scene,
        (x, y): (f32, f32),
        style: CardStyle,
    ) {
        use super::draw::image::{DX, DY, stroke};

        let right = x + (self.width - 1) as f32 * DX;
        let bottom = y + (self.height() - 1) as f32 * DY;
        let radius = if style.rounded == Some(true) {
            7.0
        } else {
            0.0
        };
        let ink = self.color.stroke.unwrap_or_else(|| scene.ink());
        let width = if style.heavy == Some(true) { 3.0 } else { 1.5 };
        let dashed = style.dashed == Some(true);
        if dashed || width > 1.5 {
            scene.rect(x, y, right - x, bottom - y, radius, self.color.fill, None);
            if dashed {
                stroke(
                    scene,
                    &[(x, y), (right, y), (right, bottom), (x, bottom), (x, y)],
                    ink,
                    width,
                    true,
                );
            } else {
                let r = radius;
                let path = format!(
                    "M{} {y}H{}Q{right} {y} {right} {}V{}Q{right} {bottom} {} {bottom}H{}Q{x} {bottom} {x} {}V{}Q{x} {y} {} {y}Z",
                    x + r,
                    right - r,
                    y + r,
                    bottom - r,
                    right - r,
                    x + r,
                    bottom - r,
                    y + r,
                    x + r,
                );
                scene.path(&path, None, Some(ink), width);
            }
        } else {
            scene.rect(
                x,
                y,
                right - x,
                bottom - y,
                radius,
                self.color.fill,
                Some(ink),
            );
        }
        for (i, row) in self.rows.iter().enumerate() {
            let row_y = y + (i + 1) as f32 * DY;
            match row {
                Row::Text { text, centered } => {
                    let left = if *centered {
                        (x + right - scene.measure(text)) / 2.0
                    } else {
                        x + DX
                    };
                    scene.text(left, row_y - DY / 2.0, text, self.color.text);
                }
                Row::Divider => stroke(
                    scene,
                    &[(x, row_y), (right, row_y)],
                    ink,
                    width.min(2.0),
                    dashed,
                ),
            }
        }
    }
}

pub(super) fn columns(text: &str) -> usize {
    text.graphemes(true).map(UnicodeWidthStr::width).sum()
}

pub(super) fn display_text(text: &str) -> Cow<'_, str> {
    let mut rendered = String::new();
    let mut start = 0;
    for (index, _) in text.match_indices('<') {
        let tail = &text[index..];
        if let Some(tag) = ["<br>", "<br/>", "<br />"].into_iter().find(|&tag| {
            tail.get(..tag.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(tag))
        }) {
            rendered.push_str(&text[start..index]);
            rendered.push('\n');
            start = index + tag.len();
        }
    }
    if start == 0 {
        Cow::Borrowed(text)
    } else {
        rendered.push_str(&text[start..]);
        Cow::Owned(rendered)
    }
}

pub(super) fn wrap(text: &str, limit: usize) -> Vec<String> {
    if text.contains('\n') {
        return text
            .split('\n')
            .flat_map(|line| wrap(line.trim_end_matches('\r'), limit))
            .collect();
    }
    let mut text = text.replace('\t', " ");
    if text.graphemes(true).any(|part| part.width() == 0) {
        text = text.graphemes(true).fold(String::new(), |mut line, part| {
            if part.width() == 0 {
                for ch in part.chars() {
                    let _ = write!(line, "\\u{{{:x}}}", u32::from(ch));
                }
            } else {
                line.push_str(part);
            }
            line
        });
    }
    if columns(&text) <= limit {
        return vec![text];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut stop = 0;
    let mut width = 0;
    for word in text.split_word_bounds() {
        let word_width = columns(word);
        if word.trim().is_empty() {
            if start == stop {
                start += word.len();
            } else {
                width += word_width;
            }
            stop += word.len();
            continue;
        }
        if width + word_width > limit && start < stop {
            lines.push(text[start..stop].trim_end().to_owned());
            start = stop;
            width = 0;
        }
        if word_width <= limit {
            width += word_width;
            stop += word.len();
            continue;
        }
        for grapheme in word.graphemes(true) {
            let grapheme_width = grapheme.width();
            if width + grapheme_width > limit && start < stop {
                lines.push(text[start..stop].trim_end().to_owned());
                start = stop;
                width = 0;
            }
            stop += grapheme.len();
            width += grapheme_width;
        }
    }
    if start < stop && !text[start..stop].trim().is_empty() {
        lines.push(text[start..stop].trim_end().to_owned());
    }
    lines
}
