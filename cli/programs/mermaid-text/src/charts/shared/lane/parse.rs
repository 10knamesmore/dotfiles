use super::*;
use rustc_hash::{FxHashMap, FxHashSet};

fn direction(word: Option<&str>) -> Result<Direction, String> {
    let (horizontal, reverse) = match word {
        None | Some("TD" | "TB") => (false, false),
        Some("BT") => (false, true),
        Some("LR") => (true, false),
        Some("RL") => (true, true),
        _ => return Err("expected direction TB, TD, BT, LR or RL".into()),
    };
    Ok(Direction {
        horizontal,
        reverse,
    })
}

pub(super) fn diagram(source: &str, syntax: Syntax) -> Result<Diagram, String> {
    let mut lines = source.lines().enumerate();
    let header = lines.next().ok_or("missing diagram header")?.1;
    let (header, tail) = header.split_once(';').unwrap_or((header, ""));
    let mut words = header.split_whitespace();
    let name = words.next().unwrap_or("");
    if !syntax.headers.contains(&name) {
        return Err("expected swimlane-beta, agentflow or agentflow-beta".into());
    }
    let direction = direction(words.next())?;
    if words.next().is_some() {
        return Err(format!("unexpected text in {name} header"));
    }
    let mut parser = Parser {
        diagram: Diagram {
            family: syntax.family,
            direction,
            items: vec![Item::new("", 0)],
            edges: vec![],
            title: String::new(),
            description: String::new(),
        },
        ids: FxHashMap::default(),
        styles: crate::node_styles::NodeStyles::default(),
        default_edge: Default::default(),
        edge_styles: FxHashMap::default(),
        edge_ids: FxHashSet::default(),
        scope: vec![],
        previous: vec![],
    };
    // Newlines separate statements except inside a data block or shape label.
    let mut stmt = String::new();
    let mut brackets = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut first = 1;
    for (line, text) in std::iter::once((0, tail)).chain(lines) {
        if text.trim().is_empty() || text.trim_start().starts_with("%%") {
            continue;
        }
        if stmt.is_empty() {
            first = line + 1;
        } else {
            stmt.push('\n');
        }
        for c in text.chars().chain(std::iter::once(';')) {
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            } else {
                match c {
                    '"' => quote = Some(c),
                    '\'' if stmt.trim_end().ends_with([':', '[', '{', ',']) => quote = Some(c),
                    '[' => brackets.push(']'),
                    '{' => brackets.push('}'),
                    '(' => brackets.push(')'),
                    ']' | '}' | ')' if brackets.last() == Some(&c) => {
                        brackets.pop();
                    }
                    ';' if brackets.is_empty() => {
                        parser
                            .statement(stmt.trim(), &syntax)
                            .map_err(|e| format!("{name} line {first}: {e}"))?;
                        stmt.clear();
                        first = line + 1;
                        continue;
                    }
                    _ => {}
                }
            }
            stmt.push(c);
        }
        if !stmt.is_empty() {
            stmt.pop(); // synthetic statement separator
        }
    }
    if !stmt.trim().is_empty() {
        return Err(format!("{name} line {first}: unclosed label or metadata"));
    }
    if !parser.scope.is_empty() {
        return Err(format!("{name}: missing end"));
    }
    for (id, index) in &parser.ids {
        parser.diagram.items[*index].style = parser.styles.resolve(id, Default::default());
    }
    for (index, edge) in parser.diagram.edges.iter_mut().enumerate() {
        edge.style = parser
            .default_edge
            .overlay(parser.edge_styles.get(&index).copied().unwrap_or_default());
    }
    Ok(parser.diagram)
}

pub struct Parser {
    styles: crate::node_styles::NodeStyles,
    default_edge: crate::color::Style,
    edge_styles: FxHashMap<usize, crate::color::Style>,
    diagram: Diagram,
    ids: FxHashMap<String, usize>,
    edge_ids: FxHashSet<String>,
    // None is a global block.
    scope: Vec<Option<usize>>,
    previous: Vec<usize>,
}

impl Parser {
    fn owner(&self) -> usize {
        self.scope.last().copied().flatten().unwrap_or(0)
    }

    fn item(&mut self, name: &str, claim: bool) -> usize {
        let owner = self.owner();
        let id = *self.ids.entry(name.to_owned()).or_insert_with(|| {
            self.diagram.items.push(Item::new(name, owner));
            self.diagram.items.len() - 1
        });
        let item = &mut self.diagram.items[id];
        if claim && !item.group {
            if matches!(self.scope.last(), Some(None)) {
                item.global = true;
                item.owner = 0;
            } else if !item.global && item.owner == 0 {
                item.owner = owner;
            } else if self.diagram.family == Family::Swimlane && owner != 0 && owner != item.owner {
                self.diagram.items[owner].lost_member = true;
            }
        }
        id
    }

    pub fn open_global(&mut self) {
        self.scope.push(None);
        self.previous.clear();
    }

    pub fn open_group(&mut self, mut input: &str, label: Option<&str>) -> Result<(), String> {
        let owner = self.owner();
        let id = if let Some(label) = label {
            self.item(label, false)
        } else {
            let id = self.node(&mut input, false)?;
            if !input.is_empty() {
                return Err(format!("unexpected group declaration: {input}"));
            }
            id
        };
        let item = &mut self.diagram.items[id];
        if item.group {
            return Err(format!("group declared twice: {}", item.label));
        }
        item.group = true;
        item.owner = owner;
        self.scope.push(Some(id));
        self.previous.clear();
        Ok(())
    }

    pub fn connector(&mut self, mut input: &str) -> Result<(), String> {
        let id = self.node(&mut input, true)?;
        if !input.is_empty() {
            return Err("unexpected text after connector".into());
        }
        self.diagram.items[id].owner = self.owner();
        self.diagram.items[id].global = self.owner() == 0;
        self.diagram.items[id].shape = "connector".into();
        self.previous = vec![id];
        Ok(())
    }

    fn statement(&mut self, s: &str, syntax: &Syntax) -> Result<(), String> {
        if s.is_empty() {
            return Ok(());
        }
        let agent = self.diagram.family == Family::Agentflow;
        let word = s.split_whitespace().next().unwrap();
        let rest = s[word.len()..].trim_start();
        if s == "end" {
            self.scope.pop().ok_or("unexpected end")?;
            self.previous.clear();
            return Ok(());
        }
        if (syntax.statement)(self, word, rest)? {
            return Ok(());
        }
        if let Some(text) = s.strip_prefix("accTitle:") {
            self.diagram.title = text.trim().into();
        } else if let Some(text) = s.strip_prefix("accDescr:") {
            self.diagram.description = text.trim().into();
        } else if matches!(word, "classDef" | "class" | "style") {
            if let Some((ids, value)) = rest.split_once(char::is_whitespace) {
                match word {
                    "classDef" => self.styles.define(ids, crate::color::Style::css(value)),
                    "class" => self.styles.assign(ids, value),
                    _ => self.styles.set(ids, crate::color::Style::css(value)),
                }
            }
        } else if word == "linkStyle" {
            if let Some((targets, properties)) = rest.split_once(char::is_whitespace) {
                let style = crate::color::Style::css(properties);
                for target in targets.split(',') {
                    if target == "default" {
                        self.default_edge = self.default_edge.overlay(style);
                    } else if let Ok(index) = target.parse::<usize>() {
                        let current = self.edge_styles.entry(index).or_default();
                        *current = current.overlay(style);
                    }
                }
            }
        } else if word == "click" {
            return Ok(());
        } else {
            let mut input = s;
            if let Some((id, fields)) = s.split_once("@{")
                && self.edge_ids.contains(id.trim())
            {
                metadata(fields.strip_suffix('}').ok_or("invalid edge metadata")?)?;
                return Ok(());
            }
            let mut from = if is_link(input) {
                if self.previous.is_empty() {
                    return Err("edge has no source".into());
                }
                self.previous.clone()
            } else {
                self.nodes(&mut input)?
            };
            while !input.is_empty() {
                if !is_link(input) {
                    let id = take_id(&mut input)?;
                    input = input
                        .strip_prefix('@')
                        .ok_or("expected edge operator or edge id followed by @")?;
                    self.edge_ids.insert(id.to_owned());
                }
                let (line, tip, label) = link(&mut input, agent)?;
                let to = self.nodes(&mut input)?;
                for &a in &from {
                    for &b in &to {
                        if self.diagram.edges.len() >= 16384 {
                            return Err("too many expanded edges".into());
                        }
                        self.diagram.edges.push(Edge {
                            style: Default::default(),
                            from: a,
                            to: b,
                            line,
                            tip,
                            label: label.clone(),
                        });
                    }
                }
                from = to;
            }
            self.previous = from;
        }
        Ok(())
    }

    fn nodes(&mut self, s: &mut &str) -> Result<Vec<usize>, String> {
        let mut ids = vec![self.node(s, true)?];
        while let Some(rest) = s.strip_prefix('&') {
            *s = rest.trim_start();
            ids.push(self.node(s, true)?);
        }
        Ok(ids)
    }

    fn node(&mut self, s: &mut &str, claim: bool) -> Result<usize, String> {
        let name = take_id(s)?;
        let id = self.item(name, claim);
        let item = &mut self.diagram.items[id];
        for (open, close, shape) in [
            ("(((", ")))", "double-circle"),
            ("([", "])", "stadium"),
            ("((", "))", "circle"),
            ("[(", ")]", "database"),
            ("[[", "]]", "tool"),
            ("[/", "/]", "input"),
            ("{{", "}}", "hexagon"),
            ("[", "]", "rect"),
            ("(", ")", "rounded"),
            ("{", "}", "decision"),
        ] {
            if let Some(rest) = s.strip_prefix(open) {
                let end = if rest.starts_with('"') {
                    let q = quoted_end(rest).ok_or("unclosed quoted label")?;
                    if !rest[q..].starts_with(close) {
                        return Err(format!("expected {close} after quoted label"));
                    }
                    q
                } else {
                    rest.find(close)
                        .ok_or_else(|| format!("unclosed shape, expected {close}"))?
                };
                item.label = label(&rest[..end]);
                item.shape = shape.into();
                *s = rest[end + close.len()..].trim_start();
                break;
            }
        }
        if let Some(rest) = s.strip_prefix("@{") {
            let mut depth = 0;
            let end = structural(rest)
                .find_map(|(i, c)| match c {
                    '{' => {
                        depth += 1;
                        None
                    }
                    '}' if depth == 0 => Some(i),
                    '}' => {
                        depth -= 1;
                        None
                    }
                    _ => None,
                })
                .ok_or("unclosed metadata")?;
            let data = metadata(&rest[..end])?;
            if let Some(text) = data.label {
                item.label = label(&text);
            }
            if let Some(shape) = data.shape {
                item.shape = shape;
            }
            if let Some(view) = data.view {
                item.collapsed = view == "collapsed";
            }
            *s = rest[end + 1..].trim_start();
        }
        if let Some(rest) = s.strip_prefix(":::") {
            *s = rest;
            self.styles.assign(name, take_id(s)?);
        }
        Ok(id)
    }
}

pub fn id_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.') || !c.is_ascii() && !c.is_whitespace()
}

fn take_id<'a>(s: &mut &'a str) -> Result<&'a str, String> {
    let end = s
        .char_indices()
        .find(|&(i, c)| !id_char(c) || is_link(&s[i..]))
        .map_or(s.len(), |(i, _)| i);
    if end == 0 {
        return Err(format!("expected id near {s:?}"));
    }
    let name = &s[..end];
    *s = s[end..].trim_start();
    Ok(name)
}

fn is_link(s: &str) -> bool {
    s.starts_with("--") || s.starts_with("-.") || s.starts_with("==")
}

fn link(s: &mut &str, agent: bool) -> Result<(Line, Tip, String), String> {
    let mut middle = String::new();
    if let Some(rest) = s.strip_prefix("--")
        && !s.starts_with("-->")
        && !s.starts_with("---")
        && !s.starts_with("--x")
    {
        let end = rest
            .find("-->")
            .or_else(|| rest.find("--x"))
            .ok_or("expected --> or --x after edge label")?;
        middle = label(&rest[..end]);
        *s = &rest[end..];
    }
    for (token, style, tip) in [
        ("-->", Line::Solid, Tip::Arrow),
        ("--x", Line::Solid, Tip::Cross),
        ("-.->", Line::Dotted, Tip::Arrow),
        ("-.-", Line::Dotted, Tip::None),
        ("==>", Line::Thick, Tip::Arrow),
        ("---", Line::Solid, Tip::None),
        ("===", Line::Thick, Tip::None),
    ] {
        if agent && !matches!(token, "-->" | "--x" | "-.-") {
            continue;
        }
        if let Some(rest) = s.strip_prefix(token) {
            *s = rest.trim_start();
            if let Some(rest) = s.strip_prefix('|') {
                let (value, after) = rest.split_once('|').ok_or("unclosed |edge label|")?;
                middle = label(value);
                *s = after.trim_start();
            }
            return Ok((style, tip, middle));
        }
    }
    Err(format!("unsupported edge operator near {s:?}"))
}

fn label(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(s);
    s.trim_matches('`')
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace("\\n", "\n")
        .replace("\\\"", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

fn quoted_end(s: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in s.char_indices().skip(1) {
        if !escaped && c == '"' {
            return Some(i + 1);
        }
        escaped = !escaped && c == '\\';
    }
    None
}

fn structural(s: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut quote = None;
    let mut escaped = false;
    s.char_indices().filter(move |&(i, c)| {
        if let Some(q) = quote {
            if !escaped && c == q {
                quote = None;
            }
            escaped = !escaped && c == '\\';
            false
        } else if c == '"' || (c == '\'' && s[..i].trim_end().ends_with([':', '[', '{', ','])) {
            quote = Some(c);
            false
        } else {
            true
        }
    })
}

#[derive(serde::Deserialize)]
struct Metadata {
    label: Option<String>,
    shape: Option<String>,
    view: Option<String>,
}

fn metadata(s: &str) -> Result<Metadata, String> {
    // Mermaid encloses both YAML flow mappings and indented block mappings in @{...}.
    yaml_serde::from_str(&format!("{{{s}}}"))
        .or_else(|_| yaml_serde::from_str(s))
        .map_err(|e| format!("invalid metadata: {e}"))
}
