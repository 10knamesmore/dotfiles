use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::Options;
use crate::color::{Layer, Paint};

mod image;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const BRANCH_COLUMNS: usize = 4;
const EMPTY_BRANCH: &str = "    ";

struct Node<'a> {
    label: String,
    shape: Shape,
    icon: Option<&'a str>,
    classes: Option<&'a str>,
    depth: usize,
    parent: Option<usize>,
    last_sibling: bool,
}

struct Ancestor {
    indent: usize,
    index: usize,
    last_child: Option<usize>,
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &crate::Options) -> Result<String, String> {
        let nodes = parse(source)?;
        draw(&nodes, options)
    }
}

fn parse(source: &str) -> Result<Vec<Node<'_>>, String> {
    let header = source
        .strip_prefix("mindmap")
        .ok_or("expected a mindmap header")?;
    let (header_tail, next) = split_line(header);
    let (mut remaining, mut line_number) =
        if header_tail.trim().is_empty() || header_tail.trim_start().starts_with("%%") {
            (next, 2)
        } else if header_tail.starts_with([' ', '\t']) {
            (header, 1)
        } else {
            return Err("expected a mindmap header".into());
        };
    let mut nodes: Vec<Node<'_>> = Vec::new();
    let mut path: Vec<Ancestor> = Vec::new();
    while !remaining.is_empty() {
        let current_line = line_number;
        let at_line = |error| format!("mindmap line {current_line}: {error}");
        let (line, next) = split_line(remaining);
        let text = line.trim_start();
        if text.is_empty() || text.starts_with("%%") {
            remaining = next;
            line_number += 1;
            continue;
        }
        if let Some(decoration) = parse_decoration(text).map_err(at_line)? {
            let node = nodes
                .last_mut()
                .ok_or_else(|| at_line("node decoration needs a preceding node"))?;
            match decoration {
                Decoration::Icon(icon) => node.icon = Some(icon),
                Decoration::Classes(classes) => node.classes = Some(classes),
            }
            remaining = next;
            line_number += 1;
            continue;
        }

        let indent_bytes = line.len() - text.len();
        let indent = line[..indent_bytes].chars().count();
        let (label, shape, rest) = parse_label(&remaining[indent_bytes..]).map_err(at_line)?;
        line_number += remaining[..remaining.len() - rest.len()]
            .bytes()
            .filter(|&byte| byte == b'\n')
            .count();
        remaining = rest;
        while path
            .last()
            .is_some_and(|ancestor| ancestor.indent >= indent)
        {
            path.pop();
        }
        if path.is_empty() && !nodes.is_empty() {
            return Err(at_line("expected a single root"));
        }
        let node = nodes.len();
        if let Some(parent) = path.last_mut()
            && let Some(previous) = parent.last_child.replace(node)
        {
            nodes[previous].last_sibling = false;
        }
        nodes.push(Node {
            label,
            shape,
            icon: None,
            classes: None,
            depth: path.len(),
            parent: path.last().map(|ancestor| ancestor.index),
            last_sibling: true,
        });
        path.push(Ancestor {
            indent,
            index: node,
            last_child: None,
        });
    }
    if nodes.is_empty() {
        return Err("mindmap has no root".into());
    }
    Ok(nodes)
}

fn split_line(text: &str) -> (&str, &str) {
    text.split_once('\n').unwrap_or((text, ""))
}

fn before_comment(text: &str) -> &str {
    let mut quoted = false;
    for (index, marker) in text.match_indices(['"', '%']) {
        if marker == "\"" {
            quoted = !quoted;
        } else if !quoted && text[index..].starts_with("%%") {
            return text[..index].trim_end();
        }
    }
    text.trim_end()
}

enum Decoration<'a> {
    Icon(&'a str),
    Classes(&'a str),
}

fn parse_decoration(text: &str) -> Result<Option<Decoration<'_>>, &'static str> {
    if let Some(classes) = text.strip_prefix(":::") {
        let classes = before_comment(classes).trim();
        if classes.is_empty() {
            return Err("expected a class name after :::");
        }
        return Ok(Some(Decoration::Classes(classes)));
    }
    if text
        .get(..7)
        .is_some_and(|start| start.eq_ignore_ascii_case("::icon("))
    {
        let (icon, tail) = text[7..]
            .split_once(')')
            .ok_or("expected a closing ) after the icon name")?;
        let icon = icon.trim();
        if icon.is_empty() || icon.contains('(') {
            return Err("expected an icon name inside ::icon(...)");
        }
        if !is_statement_end(tail) {
            return Err("unexpected text after the icon");
        }
        return Ok(Some(Decoration::Icon(icon)));
    }
    Ok(None)
}

fn is_statement_end(tail: &str) -> bool {
    let tail = tail.trim_start();
    tail.is_empty() || tail.starts_with("%%")
}

#[derive(Clone, Copy)]
enum Shape {
    Plain,
    Rectangle,
    Rounded,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

fn parse_label(input: &str) -> Result<(String, Shape, &str), &'static str> {
    let (first, next) = split_line(input);
    let first = before_comment(first);
    let Some(open) = first.find(['[', '(', ')', '{', '}']) else {
        return Ok((display_label(first, LabelKind::Plain)?, Shape::Plain, next));
    };
    let open = if first.starts_with("-)") { 0 } else { open };
    if input[..open].ends_with('@') {
        return Err("unsupported mindmap node shape");
    }
    let (body, close, shape) = [
        ("(-", "-)", Shape::Cloud),
        ("-)", "(-", Shape::Bang),
        ("((", "))", Shape::Circle),
        ("))", "((", Shape::Bang),
        ("{{", "}}", Shape::Hexagon),
        ("[", "]", Shape::Rectangle),
        ("(", ")", Shape::Rounded),
        (")", "(", Shape::Cloud),
    ]
    .into_iter()
    .find_map(|(start, end, shape)| {
        input[open..]
            .strip_prefix(start)
            .map(|body| (body, end, shape))
    })
    .ok_or("unsupported mindmap node shape")?;

    let (label, tail, kind) = if let Some(text) = body.trim_start().strip_prefix("\"`") {
        let (label, tail) = text.split_once("`\"").ok_or("unclosed Markdown label")?;
        (label, tail.trim_start(), LabelKind::Markdown)
    } else if let Some(text) = body.trim_start().strip_prefix('"') {
        let (label, tail) = text.split_once('"').ok_or("unclosed quoted label")?;
        (label, tail.trim_start(), LabelKind::Plain)
    } else if let Some(text) = body.trim_start().strip_prefix('`') {
        let (label, tail) = text.split_once('`').ok_or("unclosed code label")?;
        (label, tail.trim_start(), LabelKind::Code)
    } else {
        let mut end = body
            .find(['[', ']', '(', ')', '{', '}'])
            .ok_or("unclosed node shape")?;
        if close == "-)" && body[..end].ends_with('-') {
            end -= 1;
        }
        (&body[..end], &body[end..], LabelKind::Plain)
    };
    let tail = tail
        .strip_prefix(close)
        .ok_or("expected a matching node shape delimiter")?;
    let (tail, next) = split_line(tail);
    if !is_statement_end(tail) {
        return Err("unexpected text after the node shape");
    }
    Ok((display_label(label, kind)?, shape, next))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    Plain,
    Markdown,
    Code,
}

fn display_label(text: &str, kind: LabelKind) -> Result<String, &'static str> {
    let mut text = text;
    let multiline = text.contains('\n');
    let mut continuation_indent = 0;
    let mut first_is_continuation = false;
    if multiline {
        text = text.trim_end();
        if let Some((first, rest)) = text.split_once('\n')
            && first.trim().is_empty()
        {
            text = rest;
            first_is_continuation = true;
        }
        continuation_indent = text
            .split('\n')
            .skip(usize::from(!first_is_continuation))
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
            .min()
            .unwrap_or(0);
    }
    let mut result = String::with_capacity(text.len());
    for (index, mut line) in text.split('\n').enumerate() {
        if index > 0 {
            result.push('\n');
        }
        if index > 0 || first_is_continuation {
            let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
            line = &line[indent.min(continuation_indent)..];
        }
        let start = result.len();
        render_label_line(line, kind, &mut result)?;
        if kind == LabelKind::Markdown {
            let plain = plain_emphasis(&result[start..]);
            result.truncate(start);
            result.push_str(&plain);
        }
    }
    if result.trim().width() == 0 {
        if text.trim().width() == 0 {
            return Err("expected a nonempty node label");
        }
        result.push_str("[empty label]");
    }
    Ok(result)
}

fn render_label_line(
    mut line: &str,
    kind: LabelKind,
    out: &mut String,
) -> Result<(), &'static str> {
    while let Some(index) = line.find(['<', '&', '#', '[', '!', '`', '\t']) {
        out.push_str(&line[..index]);
        line = &line[index..];
        match line.as_bytes()[0] {
            b'<' if kind != LabelKind::Code => {
                if let Some((tag, rest)) = html_tag(line)? {
                    if tag.eq_ignore_ascii_case("br") {
                        out.push('\n');
                    } else if tag.eq_ignore_ascii_case("img") {
                        out.push_str("[image]");
                    }
                    line = rest;
                    continue;
                }
            }
            b'&' | b'#' => {
                if let Some((value, rest)) = text_entity(line) {
                    out.push(value);
                    line = rest;
                    continue;
                }
            }
            b'[' | b'!' if kind == LabelKind::Markdown => {
                let image = line.starts_with('!');
                if let Some((label, rest)) = markdown_link(&line[usize::from(image)..]) {
                    if image {
                        out.push_str("[image: ");
                    }
                    render_label_line(label, LabelKind::Plain, out)?;
                    if image {
                        out.push(']');
                    }
                    line = rest;
                    continue;
                }
            }
            b'`' if kind == LabelKind::Markdown => {
                if let Some((code, rest)) = line[1..]
                    .split_once('`')
                    .filter(|(code, _)| !code.is_empty())
                {
                    out.push('`');
                    out.push_str(&code.replace('\t', "    "));
                    out.push('`');
                    line = rest;
                    continue;
                }
            }
            b'\t' => {
                out.push_str("    ");
                line = &line[1..];
                continue;
            }
            _ => {}
        }
        out.push(line.as_bytes()[0] as char);
        line = &line[1..];
    }
    out.push_str(line);
    Ok(())
}

fn html_tag(text: &str) -> Result<Option<(&str, &str)>, &'static str> {
    if let Some(comment) = text.strip_prefix("<!--") {
        let (_, tail) = comment
            .split_once("-->")
            .ok_or("unclosed HTML comment in label")?;
        return Ok(Some(("", tail)));
    }
    let name_at = text[1..].strip_prefix('/').unwrap_or(&text[1..]);
    if !name_at.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
        return Ok(None);
    }
    let name_end = name_at
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
        .unwrap_or(name_at.len());
    let (name, body) = name_at.split_at(name_end);
    if !body.starts_with(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '/' | '>')) {
        return Ok(None);
    }
    let mut quote = None;
    for (index, ch) in body.char_indices() {
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            }
        } else {
            match ch {
                '"' | '\'' => quote = Some(ch),
                '<' => return Err("unclosed HTML tag in label"),
                '>' => {
                    if ["script", "style", "iframe", "object"]
                        .iter()
                        .any(|tag| name.eq_ignore_ascii_case(tag))
                    {
                        return Err("unsupported HTML element in label");
                    }
                    return Ok(Some((name, &body[index + 1..])));
                }
                _ => {}
            }
        }
    }
    Err("unclosed HTML tag in label")
}

fn text_entity(text: &str) -> Option<(char, &str)> {
    let body = text[1..].strip_prefix('#').unwrap_or(&text[1..]);
    let end = body.find(|ch: char| !ch.is_ascii_alphanumeric())?;
    let (name, tail) = body.split_at(end);
    let rest = tail.strip_prefix(';')?;
    let value = match name {
        "amp" => '&',
        "apos" => '\'',
        "copy" => '©',
        "gt" => '>',
        "hearts" => '♥',
        "hellip" => '…',
        "infin" => '∞',
        "lt" => '<',
        "mdash" => '—',
        "nbsp" => '\u{a0}',
        "ndash" => '–',
        "quot" => '"',
        "reg" => '®',
        _ => {
            let number = if text.starts_with("&#")
                && let Some(hex) = name.strip_prefix(['x', 'X'])
            {
                u32::from_str_radix(hex, 16)
            } else if text.starts_with('#') || text.starts_with("&#") {
                name.parse::<u32>()
            } else {
                return None;
            };
            char::from_u32(number.ok()?)?
        }
    };
    (!value.is_control()).then_some((value, rest))
}

fn markdown_link(text: &str) -> Option<(&str, &str)> {
    let end = text.strip_prefix('[')?.find(['[', ']'])? + 1;
    let dest = text[end..].strip_prefix("](")?;
    let mut depth = 0;
    for (index, ch) in dest.char_indices() {
        match ch {
            '[' => break,
            '(' => depth += 1,
            ')' if depth == 0 => return Some((&text[1..end], &dest[index + 1..])),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn plain_emphasis(text: &str) -> String {
    let mut markers = text.match_indices(['*', '_', '`', '\\']).peekable();
    let mut open = [None; 4];
    let mut removed = Vec::new();
    while let Some((index, piece)) = markers.next() {
        let slot = match piece {
            "*" => 0,
            "_" => 2,
            "`" => {
                if let Some((end, _)) = markers.find(|&(_, piece)| piece == "`") {
                    removed.extend([(index, 1), (end, 1)]);
                }
                continue;
            }
            "\\" => {
                if markers.peek().is_some_and(|&(next, _)| next == index + 1) {
                    markers.next();
                    removed.push((index, 1));
                }
                continue;
            }
            _ => continue,
        };
        let len = if markers.peek() == Some(&(index + 1, piece)) {
            markers.next();
            2
        } else {
            1
        };
        let before = text[..index].chars().next_back();
        let after = text[index + len..].chars().next();
        let underscores_in_word = piece == "_"
            && before.is_some_and(char::is_alphanumeric)
            && after.is_some_and(char::is_alphanumeric);
        if underscores_in_word {
            continue;
        }
        let opener = &mut open[slot + len - 1];
        if before.is_some_and(|ch| !ch.is_whitespace())
            && let Some(start) = opener.take()
            && start + len < index
        {
            removed.extend([(start, len), (index, len)]);
        } else if after.is_some_and(|ch| !ch.is_whitespace()) {
            *opener = Some(index);
        }
    }
    removed.sort_unstable();
    let mut result = String::with_capacity(text.len());
    let mut start = 0;
    for (index, len) in removed {
        result.push_str(&text[start..index]);
        start = index + len;
    }
    result.push_str(&text[start..]);
    result
}

struct Guides {
    branch: &'static str,
    last: &'static str,
    through: &'static str,
}

fn palette(options: &Options) -> [Paint; 12] {
    let mut palette = [Paint::default(); 12];
    if options.color {
        for (i, paint) in palette.iter_mut().enumerate() {
            paint.fg = options.config.theme_color(&format!("cScaleLabel{i}"));
            paint.bg = options.config.theme_color(&format!("cScale{i}"));
        }
        palette[0].fg = options
            .config
            .theme_color("gitBranchLabel0")
            .or(palette[0].fg);
        palette[0].bg = options.config.theme_color("git0").or(palette[0].bg);
    }
    palette
}

fn draw(nodes: &[Node<'_>], options: &Options) -> Result<String, String> {
    let guides = if options.ascii {
        Guides {
            branch: "|-- ",
            last: "`-- ",
            through: "|   ",
        }
    } else {
        Guides {
            branch: "├── ",
            last: "└── ",
            through: "│   ",
        }
    };
    let mut prefix = String::new();
    let mut prefix_lengths = vec![0];
    let mut output = String::new();
    let mut colors = Layer::default();
    let palette = palette(options);
    let mut slot = 0;
    let mut y = 0;

    for node in nodes {
        if node.depth == 1 {
            slot = slot % (palette.len() - 1) + 1;
        }
        let label_paint = palette[slot];
        prefix.truncate(prefix_lengths[node.depth]);
        prefix_lengths.truncate(node.depth + 1);
        let (mut branch, continuation) = match (node.depth, node.last_sibling) {
            (0, _) => ("", ""),
            (_, true) => (guides.last, EMPTY_BRANCH),
            (_, false) => (guides.branch, guides.through),
        };
        let label_columns = options
            .width
            .checked_sub(node.depth * BRANCH_COLUMNS)
            .filter(|&remaining| remaining > 0)
            .unwrap_or(options.width);
        draw_label(node, label_columns, |text| {
            if output.len() + prefix.len() + branch.len() + text.len() + 1 > MAX_OUTPUT_BYTES {
                return Err(format!("mindmap output exceeds {MAX_OUTPUT_BYTES} bytes"));
            }
            output.push_str(&prefix);
            output.push_str(branch);
            output.push_str(text);
            output.push('\n');
            let x = node.depth * BRANCH_COLUMNS;
            colors.paint(
                0,
                y,
                x,
                Paint {
                    fg: label_paint.bg,
                    bg: None,
                },
            );
            colors.paint(x, y, text.width(), label_paint);
            y += 1;
            branch = continuation;
            Ok(())
        })?;
        prefix.push_str(continuation);
        prefix_lengths.push(prefix.len());
    }
    output.pop();
    Ok(colors.apply(&output))
}

fn draw_label(
    node: &Node<'_>,
    columns: usize,
    mut emit: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let last = if let Some((leading, last)) = node.label.rsplit_once('\n') {
        for line in leading.split('\n') {
            wrap_label(line, columns, &mut emit)?;
        }
        last
    } else {
        &node.label
    };
    let mut tail = Cow::Borrowed(last);
    for (prefix, decoration) in [(" [icon: ", node.icon), (" [class: ", node.classes)] {
        if let Some(value) = decoration {
            let text = tail.to_mut();
            text.push_str(prefix);
            text.extend(value.chars().map(|c| if c == '\t' { ' ' } else { c }));
            text.push(']');
        }
    }
    wrap_label(&tail, columns, emit)
}

fn wrap_label(
    text: &str,
    limit: usize,
    mut emit: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    if text.width() <= limit {
        return emit(text);
    }

    let mut boundaries = text
        .split_word_bound_indices()
        .map(|(index, _)| index)
        .peekable();
    let mut start = 0;
    let mut start_columns = 0;
    let mut columns = 0;
    let mut word_break = (0, 0);
    for (index, cluster) in text.grapheme_indices(true) {
        while let Some(&boundary) = boundaries.peek().filter(|&&boundary| boundary <= index) {
            if boundary == index {
                word_break = (index, columns);
            }
            boundaries.next();
        }
        let width = cluster.width();
        let next_columns = columns + width;
        if width > 0 && columns > start_columns && next_columns - start_columns > limit {
            let (end, end_columns) = if word_break.0 > start && next_columns - word_break.1 <= limit
            {
                word_break
            } else {
                (index, columns)
            };
            let line = text[start..end].trim_end_matches(' ');
            if !line.is_empty() {
                emit(line)?;
            }
            (start, start_columns) = (end, end_columns);
        }
        if start != 0 && start == index && cluster == " " {
            (start, start_columns) = (index + 1, next_columns);
        }
        columns = next_columns;
    }
    if start < text.len() {
        emit(&text[start..])?;
    }
    Ok(())
}
