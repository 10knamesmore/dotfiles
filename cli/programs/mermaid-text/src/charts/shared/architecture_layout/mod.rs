use rustc_hash::FxHashMap;

use crate::{
    Options,
    color::{Paint, Rgb, Style},
    graphics::Scene,
};

mod layout;
mod wires;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    pub fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
        }
    }

    fn letter(self) -> &'static str {
        match self {
            Self::Top => "T",
            Self::Right => "R",
            Self::Bottom => "B",
            Self::Left => "L",
        }
    }
}

pub struct Node {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub icon: String,
    pub details: Vec<String>,
    pub parent: Option<String>,
    pub boundary: bool,
    pub style: Style,
}

pub struct End {
    pub id: String,
    pub side: Option<Side>,
    pub group: bool,
    pub arrow: bool,
}

pub struct Edge {
    pub from: End,
    pub to: End,
    pub label: String,
    pub style: Style,
}

#[derive(Clone, Copy, Default)]
struct LinkPaint {
    line: Paint,
    arrow: Paint,
    text: Paint,
}

#[derive(Default)]
pub struct Palette {
    node: Style,
    boundary: Style,
    person: Style,
    link: Style,
    arrow: Option<Rgb>,
    enabled: bool,
}

impl Palette {
    pub fn new(options: &Options) -> Self {
        if !options.color {
            return Self::default();
        }
        let theme = |key| options.config.theme_color(key);
        let node = options.theme_style();
        let boundary = node.overlay(Style {
            fill: theme("clusterBkg"),
            stroke: theme("clusterBorder"),
            text: None,
        });
        let person = node.overlay(Style {
            fill: theme("personBkg"),
            stroke: theme("personBorder"),
            text: None,
        });
        Self {
            node,
            boundary,
            person,
            link: Style {
                fill: None,
                stroke: options.theme_line(),
                text: theme("textColor").or(node.text),
            },
            arrow: None,
            enabled: true,
        }
    }

    pub fn architecture(options: &Options) -> Self {
        let mut palette = Self::new(options);
        if palette.enabled {
            let theme = |key| options.config.theme_color(key);
            palette.boundary.stroke = theme("archGroupBorderColor").or(palette.boundary.stroke);
            palette.link.stroke = theme("archEdgeColor").or(palette.link.stroke);
            palette.arrow = theme("archEdgeArrowColor");
        }
        palette
    }

    fn element(&self, node: &Node) -> Style {
        if !self.enabled {
            return Style::default();
        }
        let theme = if node.boundary {
            self.boundary
        } else if matches!(node.kind.as_str(), "Person" | "Person_Ext") {
            self.person
        } else {
            self.node
        };
        theme.overlay(node.style)
    }

    fn relationship(&self, edge: &Edge) -> LinkPaint {
        if !self.enabled {
            return LinkPaint::default();
        }
        let style = self.link.overlay(edge.style);
        LinkPaint {
            line: style.border(),
            arrow: Paint {
                fg: edge.style.stroke.or(self.arrow).or(style.stroke),
                bg: None,
            },
            text: style.ink(),
        }
    }
}

#[derive(Default)]
pub struct Diagram {
    heading: Vec<String>,
    pub nodes: Vec<Node>,
    ids: FxHashMap<String, usize>,
    pub edges: Vec<Edge>,
    pub align: Vec<(bool, Vec<String>)>,
    pub direction: Option<Side>,
    pub shapes_per_row: Option<usize>,
    pub groups_per_row: Option<usize>,
}

impl Diagram {
    pub fn add(&mut self, node: Node) -> Result<(), String> {
        if node.id.is_empty() {
            return Err("empty element identifier".into());
        }
        if self.nodes.len() == 2048 {
            return Err("diagram exceeds 2048 elements".into());
        }
        if self.ids.insert(node.id.clone(), self.nodes.len()).is_some() {
            return Err(format!("duplicate element identifier {:?}", node.id));
        }
        self.nodes.push(node);
        Ok(())
    }

    pub fn index(&self, id: &str) -> Result<usize, String> {
        self.ids
            .get(id)
            .copied()
            .ok_or_else(|| format!("unknown element {id:?}"))
    }

    fn endpoint(&self, end: &End) -> Result<usize, String> {
        let i = self.index(&end.id)?;
        if end.group {
            self.index(
                self.nodes[i]
                    .parent
                    .as_deref()
                    .ok_or_else(|| format!("{} has no enclosing group", end.id))?,
            )
        } else {
            Ok(i)
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.edges.len() > 4096 {
            return Err("diagram exceeds 4096 connections".into());
        }
        for node in &self.nodes {
            let mut parent = node.parent.as_deref();
            let mut depth = 0;
            while let Some(id) = parent {
                let p = &self.nodes[self.index(id)?];
                if !p.boundary {
                    return Err(format!("{id:?} is not a boundary"));
                }
                if id == node.id {
                    return Err(format!("cyclic containment at {id:?}"));
                }
                depth += 1;
                if depth > 48 {
                    return Err("containment exceeds 48 levels or contains a cycle".into());
                }
                parent = p.parent.as_deref();
            }
        }
        for edge in &self.edges {
            for end in [&edge.from, &edge.to] {
                self.endpoint(end)?;
            }
        }
        for (_, ids) in &self.align {
            for id in ids {
                let node = &self.nodes[self.index(id)?];
                if node.boundary {
                    return Err("align requires services or junctions".into());
                }
            }
        }
        Ok(())
    }

    pub fn render(&self, options: &Options, palette: &Palette) -> Result<String, String> {
        self.validate()?;
        layout::render(self, options, palette)
    }

    pub fn image(
        &self,
        options: &Options,
        palette: &Palette,
        scene: &mut Scene,
    ) -> Result<(), String> {
        self.validate()?;
        layout::image(self, options, palette, scene)
    }
}

pub struct Input<'a>(pub &'a str);

impl<'a> Input<'a> {
    pub fn rest(&self) -> &'a str {
        self.0.trim_start()
    }

    pub fn take(&mut self, prefix: &str) -> bool {
        if let Some(s) = self.rest().strip_prefix(prefix) {
            self.0 = s;
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, s: &str) -> Result<(), String> {
        self.take(s)
            .then_some(())
            .ok_or_else(|| format!("expected {s:?}"))
    }

    pub fn id(&mut self) -> Result<&'a str, String> {
        let s = self.rest();
        let end = s
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(s.len());
        if end == 0 {
            return Err("expected identifier".into());
        }
        self.0 = &s[end..];
        Ok(&s[..end])
    }

    pub fn line(&mut self) -> &'a str {
        let (line, tail) = self.0.split_once('\n').unwrap_or((self.0, ""));
        self.0 = tail;
        line.trim()
    }

    pub fn quoted(&mut self) -> Result<String, String> {
        let s = self.rest();
        let q = s.chars().next().ok_or("expected quoted text")?;
        let mut text = String::new();
        let mut chars = s[q.len_utf8()..].char_indices();
        while let Some((i, c)) = chars.next() {
            if c == q {
                self.0 = &s[i + 2 * q.len_utf8()..];
                return Ok(text);
            }
            if c == '\\' {
                let (_, v) = chars.next().ok_or("unfinished escape")?;
                if v != q && v != '\\' {
                    text.push(c);
                }
                text.push(v);
            } else {
                text.push(c);
            }
        }
        Err("unclosed quoted text".into())
    }

    pub fn delimited(&mut self, close: &str) -> Result<String, String> {
        if self.rest().starts_with(['\'', '"']) {
            let value = self.quoted()?;
            self.expect(close)?;
            Ok(value)
        } else {
            let (s, tail) = self
                .0
                .split_once(close)
                .ok_or_else(|| format!("missing {close:?}"))?;
            self.0 = tail;
            Ok(s.trim().to_owned())
        }
    }

    pub fn heading(&mut self, key: &str, d: &mut Diagram) -> Result<bool, String> {
        let text = match key {
            "title" => self.line().to_owned(),
            "accDescr" if self.take("{") => self.delimited("}")?,
            "accTitle" | "accDescr" => {
                self.expect(":")?;
                self.line().to_owned()
            }
            _ => return Ok(false),
        };
        d.heading.push(text);
        Ok(true)
    }
}

pub fn parse<T: Copy>(
    source: &str,
    parse_header: impl FnOnce(&str) -> Option<T>,
    mut statement: impl FnMut(&mut Diagram, &mut Input<'_>, T) -> Result<(), String>,
) -> Result<Diagram, String> {
    let input = source.trim_start();
    let (header, body) = input.split_at(input.find(char::is_whitespace).unwrap_or(input.len()));
    let header_options = parse_header(header)
        .ok_or_else(|| format!("unknown architecture diagram header {header:?}"))?;
    let mut d = Diagram::default();
    let mut input = Input(body);
    while !input.rest().is_empty() {
        let at = input.rest();
        if let Err(error) = statement(&mut d, &mut input, header_options) {
            let line = body[..body.len() - at.len()]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
                + 1;
            return Err(format!("{header} line {line}: {error}"));
        }
    }
    Ok(d)
}
