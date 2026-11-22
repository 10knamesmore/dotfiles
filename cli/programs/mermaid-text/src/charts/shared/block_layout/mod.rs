use rustc_hash::FxHashMap;

use crate::{
    Options,
    color::{Paint, Style},
};

mod layout;
mod wires;

pub use layout::image;
pub use layout::render;

#[derive(Default)]
pub struct Diagram {
    pub nodes: Vec<Node>,
    pub names: FxHashMap<String, usize>,
    pub edges: Vec<Edge>,
    pub caption: Vec<String>,
}

pub struct Node {
    pub label: String,
    pub kind: Kind,
    pub style: Style,
}

pub enum Kind {
    Block(Shape),
    Group(Grid),
    Requirement {
        kind: String,
        fields: Vec<(String, String)>,
    },
}

#[derive(Default)]
pub struct Grid {
    pub columns: Option<usize>,
    pub slots: Vec<(Option<usize>, usize)>,
}

#[derive(Clone, Copy, Default)]
pub enum Shape {
    #[default]
    Box,
    Round,
    Circle,
    DoubleCircle,
    Stadium,
    Routine,
    Cylinder,
    Diamond,
    Hexagon,
    Flag,
    Slanted(char, char),
    Arrows(u8),
}

pub const NORTH: u8 = 1;
pub const EAST: u8 = 2;
pub const SOUTH: u8 = 4;
pub const WEST: u8 = 8;

#[derive(Clone, Copy, PartialEq)]
pub enum Tip {
    None,
    Arrow,
    Cross,
    Circle,
}

#[derive(Clone, Copy)]
pub enum Line {
    Solid,
    Thick,
    Dotted,
}

pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub start: Tip,
    pub end: Tip,
    pub line: Line,
}

impl Diagram {
    pub fn insert(&mut self, name: &str, node: Node) -> Result<usize, String> {
        let index = self.nodes.len();
        if index >= 4096 {
            return Err("diagram exceeds 4096 blocks".into());
        }
        if !name.is_empty() && self.names.insert(name.to_owned(), index).is_some() {
            return Err(format!("duplicate block name: {name}"));
        }
        self.nodes.push(node);
        Ok(index)
    }
}

#[derive(Default)]
pub struct DiagramColors {
    pub nodes: Style,
    pub line: Paint,
    pub label: Paint,
}

impl DiagramColors {
    pub fn new(options: &Options) -> Option<Self> {
        options.color.then(|| {
            let nodes = options.theme_style();
            Self {
                nodes,
                line: Paint {
                    fg: options.theme_line(),
                    bg: None,
                },
                label: Paint {
                    fg: nodes.text,
                    bg: None,
                },
            }
        })
    }
}

pub fn body(source: &str) -> &str {
    let source = source.trim_start();
    let end = source
        .find(|c: char| c.is_whitespace() || c == ';')
        .unwrap_or(source.len());
    &source[end..]
}

// Preserve newlines and punctuation inside quoted Mermaid labels.
pub fn statements(s: &str) -> Result<Vec<(usize, &str)>, String> {
    let mut parts = Vec::new();
    let (mut start, mut first, mut line) = (0, 1, 1);
    let mut quoted = false;
    for (at, c) in s.char_indices() {
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted && matches!(c, ';' | '\n') {
            let part = s[start..at].trim();
            if !part.is_empty() {
                parts.push((first, part));
            }
            start = at + 1;
            first = line + usize::from(c == '\n');
        }
        line += usize::from(c == '\n');
    }
    if quoted {
        return Err(format!("unterminated quoted label on line {first}"));
    }
    if !s[start..].trim().is_empty() {
        parts.push((first, s[start..].trim()));
    }
    Ok(parts)
}

pub fn label(s: &str) -> String {
    s.replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace("&nbsp;", " ")
        .replace("#nbsp;", " ")
        .replace('\t', " ")
}

pub fn class_names(s: &str) -> bool {
    s.split(',').all(|s| {
        !s.trim().is_empty()
            && s.trim()
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
    })
}

pub enum PaintStatement<'a> {
    Define { names: &'a str, style: Style },
    Assign { targets: &'a str, classes: &'a str },
    Direct { targets: &'a str, style: Style },
}

pub fn paint_statement(line: &str) -> Result<Option<PaintStatement<'_>>, String> {
    let (kind, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    if !matches!(kind, "style" | "classDef" | "class") {
        return Ok(None);
    }
    let (targets, props) = rest
        .trim()
        .split_once(char::is_whitespace)
        .ok_or("missing style or class")?;
    let valid = if kind == "class" {
        class_names(props)
    } else {
        // A CSS value may include commas (e.g. rgb() or a font family list).
        props
            .split_once(':')
            .is_some_and(|(key, val)| class_names(key) && !val.trim().is_empty())
    };
    if !class_names(targets) || !valid {
        return Err(format!("invalid {kind} statement: {line}"));
    }
    Ok(Some(match kind {
        "classDef" => PaintStatement::Define {
            names: targets,
            style: Style::css(props),
        },
        "class" => PaintStatement::Assign {
            targets,
            classes: props,
        },
        _ => PaintStatement::Direct {
            targets,
            style: Style::css(props),
        },
    }))
}
