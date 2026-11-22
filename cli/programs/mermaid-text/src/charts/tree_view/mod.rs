use super::shared::board::{
    self, Captions, Pen, indent, paint_wrapped, theme_color,
    tree::{Forest, branches},
};
use crate::{
    Options,
    color::{Layer, Paint, Rgb},
    text::wrap,
};

mod image;

const HEADERS: &[&str] = &["treeView", "treeView-beta"];

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let body = board::body(source, HEADERS)?;
        render(body, options, Pen::new(options))
    }
}

struct TreePaint {
    line: Paint,
    label: Paint,
    icon: Paint,
    description: Paint,
    highlight: Option<Rgb>,
}

impl TreePaint {
    fn new(options: &Options) -> Self {
        let fg = |key: &str| Paint {
            fg: theme_color(options, key),
            bg: None,
        };
        Self {
            line: fg("treeView.lineColor"),
            label: fg("treeView.labelColor"),
            icon: fg("treeView.iconColor"),
            description: fg("treeView.descriptionColor"),
            highlight: theme_color(options, "treeView.highlightBg"),
        }
    }
}

enum Annotation<'a> {
    Class(&'a str),
    Icon(&'a str),
    Description(&'a str),
}

struct TreeLabel<'a> {
    name: &'a str,
    annotations: Vec<Annotation<'a>>,
    highlighted: bool,
}

fn tree_label(text: &str) -> Result<TreeLabel<'_>, String> {
    let starts_annotation =
        |s: &str| s.starts_with(":::") || s.starts_with("icon(") || s.starts_with("##");
    let (name, mut rest) = if let Some(q @ ('\'' | '"')) = text.chars().next() {
        text[1..]
            .split_once(q)
            .ok_or("unclosed quoted tree label")?
    } else {
        let end = text
            .char_indices()
            .find(|&(i, _)| {
                starts_annotation(&text[i..])
                    && (i == 0 || text[..i].ends_with(char::is_whitespace))
            })
            .map_or(text.len(), |(i, _)| i);
        (text[..end].trim_end(), &text[end..])
    };
    if name.is_empty() {
        return Err("expected a tree label before the annotation".into());
    }
    let mut annotations = Vec::new();
    let mut highlighted = false;
    while !rest.trim_start().is_empty() {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix(":::") {
            let after = after.trim_start();
            let n = after.find(char::is_whitespace).unwrap_or(after.len());
            let class = &after[..n];
            if class.is_empty()
                || !class
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err("invalid tree class annotation".into());
            }
            annotations.push(Annotation::Class(class));
            highlighted |= class == "highlight";
            rest = &after[n..];
        } else if let Some(after) = rest.strip_prefix("icon(") {
            let (icon, tail) = after
                .split_once(')')
                .ok_or("unclosed tree icon annotation")?;
            if !icon
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ':'))
            {
                return Err("invalid tree icon name".into());
            }
            if !(icon.is_empty() || icon == "none") {
                annotations.push(Annotation::Icon(icon));
            }
            rest = tail;
        } else if let Some(description) = rest.strip_prefix("##") {
            annotations.push(Annotation::Description(description.trim()));
            break;
        } else {
            return Err(format!("unexpected text after tree label: {rest}"));
        }
    }
    Ok(TreeLabel {
        name,
        annotations,
        highlighted,
    })
}

fn tree_row(line: &str) -> Result<(usize, &str), String> {
    // The branch's column is its depth in box-drawing input. One extra level
    // allows an unadorned root followed by branches in the same column.
    let mut col = 0;
    for (i, c) in line.char_indices() {
        match c {
            ' ' | '│' | '┃' => col += 1,
            '\t' => col = (col / 4 + 1) * 4,
            '├' | '└' | '┣' | '┗' if line[i + c.len_utf8()..].starts_with(['─', '━']) =>
            {
                let tail = &line[i + c.len_utf8()..];
                let tail = tail.trim_start_matches(['─', '━']);
                let label = tail.trim_start();
                if label.is_empty() {
                    return Err("tree branch has no label".into());
                }
                return Ok((col + 1, label));
            }
            _ => break,
        }
    }
    let text = line.trim_start();
    Ok((indent(&line[..line.len() - text.len()]), text))
}

fn parse(body: &str) -> Result<(Captions, Forest<TreeLabel<'_>>), String> {
    let mut forest = Forest::default();
    let mut captions = Captions::default();
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let text = line.trim();
        if text.is_empty() || text.starts_with("%%") {
            continue;
        }
        if forest.nodes.is_empty() && captions.consume(text, &mut lines)? {
            continue;
        }
        let (level, text) = tree_row(line)?;
        forest.push(level, tree_label(text)?)?;
    }
    if forest.nodes.is_empty() {
        return Err("treeView needs at least one node".into());
    }
    Ok((captions, forest))
}

fn render(body: &str, options: &Options, pen: Pen) -> Result<String, String> {
    let (captions, forest) = parse(body)?;
    let theme = TreePaint::new(options);
    let limit = options.width;
    let mut colors = Layer::default();
    let draw = |node: &TreeLabel<'_>, y, x, columns| {
        let mut label = node.name.to_owned();
        let mut fragments = vec![(label.len(), theme.label)];
        for annotation in &node.annotations {
            let (text, paint) = match annotation {
                Annotation::Class(class) => (format!(" [{class}]"), theme.label),
                Annotation::Icon(icon) => (format!(" [{icon}]"), theme.icon),
                Annotation::Description(description) => {
                    (format!("  : {description}"), theme.description)
                }
            };
            label.push_str(&text);
            fragments.push((label.len(), paint));
        }
        if node.highlighted {
            for (_, paint) in &mut fragments {
                paint.bg = theme.highlight;
            }
        }
        let lines = wrap(&label, columns);
        for dy in 0..lines.len() {
            colors.paint(0, y + dy, x, theme.line);
        }
        if fragments
            .iter()
            .any(|(_, paint)| *paint != Paint::default())
        {
            let mut start = 0;
            let parts = fragments.into_iter().map(|(end, paint)| {
                let from = std::mem::replace(&mut start, end);
                (&label[from..end], paint)
            });
            paint_wrapped(&mut colors, x, y, &lines, parts);
        }
        lines
    };
    let rows = branches(&forest.nodes, 0, !forest.nodes[0].last, limit, pen, draw);
    Ok(captions.prepend(colors.apply(&rows.join("\n")), limit))
}
