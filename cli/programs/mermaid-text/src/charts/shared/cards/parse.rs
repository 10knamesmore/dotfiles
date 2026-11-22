use std::collections::BTreeMap;

use crate::{charts::shared::Direction, color::Style};

use super::{CardStyle, Diagram, End, Link, Name, Namespace, Node, Relationship};

pub type Parsed<T> = Result<T, &'static str>;

pub trait Grammar {
    const HEADERS: &'static [&'static str];
    const IDENTIFIER_QUOTE: char;
    const MEMBER_QUOTES: &'static [char] = &[];
    const DIRECTIONAL_LABELS: bool = false;

    fn statement<'a>(parser: &mut Parser<'a>, text: &'a str, line: usize) -> Parsed<()>;
    fn member<'a>(node: &mut Node<'a>, text: &'a str) -> Parsed<()>;
    fn connector<'a>(text: &mut &'a str) -> Parsed<(Link, [End<'a>; 2])>;
    fn unquoted_alias<'a>(text: &mut &'a str) -> Parsed<&'a str>;

    fn open_quote(_text: &str, _open: Option<char>) -> Option<char> {
        None
    }

    fn generic<'a>(_text: &mut &'a str) -> Parsed<Option<&'a str>> {
        Ok(None)
    }

    fn annotation<'a>(_text: &mut &'a str) -> Parsed<Option<&'a str>> {
        Ok(None)
    }

    fn extra_identifier_character(_c: char) -> bool {
        false
    }
}

#[derive(Default)]
pub struct Parser<'a> {
    diagram: Diagram<'a>,
    ids: BTreeMap<(Option<usize>, &'a str), usize>,
    scopes: Vec<(usize, usize)>,
    namespaces: BTreeMap<(Option<usize>, &'a str), usize>,
    declared: Vec<bool>,
    classes: Vec<(usize, &'a str)>,
    styles: BTreeMap<&'a str, CardStyle>,
    direct_colors: Vec<(usize, Style)>,
    block: Option<(usize, usize)>,
}

pub(super) fn parse<G: Grammar>(source: &str) -> Result<Diagram<'_>, String> {
    let source = source.trim_start_matches('\u{feff}');
    let mut lines = source
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, *offset, line.trim_end_matches(['\r', '\n'])))
        })
        .enumerate()
        .filter_map(|(index, (start, end, line))| {
            let text = line.trim();
            if text.is_empty() || (text.starts_with("%%") && !text.starts_with("%%{")) {
                return None;
            }
            Some((index + 1, start, end, line))
        });
    let (number, _, _, header) = lines
        .next()
        .ok_or("expected classDiagram or erDiagram header")?;
    let header = header.trim();
    if !G::HEADERS.contains(&header) {
        return Err(if header.starts_with("%%{") {
            format!("line {number}: Mermaid directives are not supported")
        } else {
            format!("line {number}: expected a standalone classDiagram or erDiagram header")
        });
    }
    let mut parser = Parser::default();
    while let Some((number, start, mut end, raw)) = lines.next() {
        let mut text = raw.trim();
        if parser.block.is_none() {
            let mut open = G::open_quote(text, None);
            while let Some(quote) = open {
                let Some((_, _, last, line)) = lines.next() else {
                    break;
                };
                end = last;
                open = G::open_quote(line, Some(quote));
            }
            text = source[start..end].trim();
        }
        let result = if text.starts_with("%%{") {
            Err("Mermaid directives are not supported")
        } else if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
        {
            Err("unexpected control character")
        } else if parser.block.is_some() {
            parser.read_block::<G>(text)
        } else {
            G::statement(&mut parser, text, number)
        };
        result.map_err(|error| format!("{header} line {number}: {error}"))?;
    }
    if let Some((index, number)) = parser.block {
        let name = parser.diagram.nodes[index].name.written;
        return Err(format!(
            "{header} line {number}: unclosed member block for {name}"
        ));
    }
    if let Some(&(_, number)) = parser.scopes.last() {
        return Err(format!("{header} line {number}: unclosed namespace"));
    }
    parser.finish()
}

impl<'a> Parser<'a> {
    fn node(&mut self, mut name: Name<'a>) -> usize {
        let parent = self.scopes.last().map(|&(index, _)| index);
        let scope = if name.id == name.written {
            self.qualified(name.id, parent)
                .or_else(|| self.qualified(name.id, None))
        } else {
            None
        };
        let parent = if let Some((scope, leaf)) = scope {
            name = Name {
                id: leaf,
                written: leaf,
            };
            Some(scope)
        } else {
            parent
        };
        *self.ids.entry((parent, name.id)).or_insert_with(|| {
            let index = self.diagram.nodes.len();
            self.declared.push(false);
            self.diagram.nodes.push(Node {
                name,
                namespace: parent,
                alias: None,
                generic: None,
                annotations: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                notes: Vec::new(),
                style: CardStyle::default(),
            });
            index
        })
    }

    pub fn common_statement<G: Grammar>(&mut self, keyword: &str, text: &'a str) -> Parsed<bool> {
        if text.is_empty() || text.starts_with('}') {
            self.close_namespaces(text)?;
            return Ok(true);
        }
        let mut rest = text[keyword.len()..].trim_start();
        match keyword {
            "direction" => {
                self.diagram.direction = Some(match rest {
                    "TB" | "TD" => Direction::Down,
                    "BT" => Direction::Up,
                    "LR" => Direction::Right,
                    "RL" => Direction::Left,
                    _ => return Err("expected direction TB, TD, BT, LR or RL"),
                });
            }
            "classDef" => {
                let targets = name_list::<G>(&mut rest)?;
                let style = card_style(rest)?;
                for name in targets {
                    self.styles.entry(name.id).or_default().apply(style);
                }
            }
            "style" => {
                let target = name::<G>(&mut rest)?;
                let style = card_style(rest)?;
                let index = self.node(target);
                self.diagram.nodes[index].style.apply_frame(style);
                self.direct_colors.push((index, style.color));
            }
            "cssClass" => self.assign_style::<G>(rest)?,
            "title" if take(&mut rest, ":") => {
                self.diagram.title = Some(nonempty(rest, "expected text after title")?);
            }
            unsupported @ ("namespace" | "subgraph" | "end" | "accTitle" | "accDescr") => {
                return Err(match unsupported {
                    "namespace" => "namespace blocks require classDiagram",
                    "subgraph" | "end" => "ER subgraph grouping is not supported",
                    _ => "accessibility metadata is handled by the diagram entry point",
                });
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn assign_style<G: Grammar>(&mut self, mut text: &'a str) -> Parsed<()> {
        let targets = if let Some(mut list) = quoted(&mut text, '"')? {
            let targets = name_list::<G>(&mut list)?;
            finish(list)?;
            targets
        } else {
            name_list::<G>(&mut text)?
        };
        let style = name::<G>(&mut text)?.id;
        finish(text.trim_end_matches(';').trim_end())?;
        for target in targets {
            let index = self.node(target);
            self.classes.push((index, style));
        }
        Ok(())
    }

    pub fn node_for(&mut self, name: Name<'a>) -> &mut Node<'a> {
        let index = self.node(name);
        &mut self.diagram.nodes[index]
    }

    pub fn note(&mut self, text: &'a str) {
        self.diagram.notes.push(text);
    }

    pub fn declare(&mut self, name: Name<'a>) -> usize {
        let index = self.node(name);
        self.declared[index] = true;
        index
    }

    pub fn declaration<G: Grammar>(
        &mut self,
        name: Name<'a>,
        mut text: &'a str,
        line: usize,
    ) -> Parsed<()> {
        let generic = G::generic(&mut text)?;
        let alias = if take(&mut text, "[") {
            let value = if let Some(value) = quoted(&mut text, '"')? {
                if !take(&mut text, "]") {
                    return Err("expected ']' after the display label");
                }
                value
            } else {
                G::unquoted_alias(&mut text)?
            };
            (!value.trim().is_empty()).then_some(value)
        } else {
            None
        };
        let style = inline_style::<G>(&mut text)?;
        let stereotype = G::annotation(&mut text)?;
        let index = self.declare(name);
        if let Some(style) = style {
            self.classes.push((index, style));
        }
        let node = &mut self.diagram.nodes[index];
        if let Some(alias) = alias {
            node.alias = Some(alias);
        }
        if let Some(generic) = generic {
            node.generic = Some(generic);
        }
        if let Some(annotation) = stereotype {
            node.annotations.push(annotation);
        }
        if take(&mut text, "{") {
            self.block = Some((index, line));
            self.read_block::<G>(text)
        } else {
            self.close_namespaces(text)
        }
    }

    fn read_block<G: Grammar>(&mut self, text: &'a str) -> Parsed<()> {
        let (index, _) = self.block.unwrap();
        let (member, tail) = block_line(text, G::MEMBER_QUOTES)?;
        let node = &mut self.diagram.nodes[index];
        if !member.is_empty() {
            G::member(node, member)?;
        }
        if let Some(tail) = tail {
            self.block = None;
            self.close_namespaces(tail)?;
        }
        Ok(())
    }

    pub fn relationship<G: Grammar>(
        &mut self,
        left: Name<'a>,
        left_type: Option<&'a str>,
        mut text: &'a str,
    ) -> Parsed<()> {
        let left_style = inline_style::<G>(&mut text)?;
        let (link, ends) = G::connector(&mut text)?;
        let right = name::<G>(&mut text)?;
        let right_type = G::generic(&mut text)?;
        let right_style = inline_style::<G>(&mut text)?;
        let label = if take(&mut text, ":") {
            if let Some(label) = quoted(&mut text, '"')? {
                finish(text)?;
                Some(label)
            } else {
                Some(nonempty(text, "expected a relationship label after ':'")?)
            }
        } else {
            finish(text)?;
            if G::DIRECTIONAL_LABELS {
                return Err("expected ':' and a relationship label");
            }
            None
        };
        let left = self.node(left);
        let right = self.node(right);
        for (index, argument) in [(left, left_type), (right, right_type)] {
            if let Some(argument) = argument {
                self.diagram.nodes[index].generic.get_or_insert(argument);
            }
        }
        self.classes.extend(
            [(left, left_style), (right, right_style)]
                .into_iter()
                .filter_map(|(index, style)| style.map(|style| (index, style))),
        );
        self.diagram.relationships.push(Relationship {
            left,
            right,
            ends,
            link,
            label,
            directional_label: G::DIRECTIONAL_LABELS,
        });
        Ok(())
    }

    pub fn open_namespace(
        &mut self,
        name: Name<'a>,
        label: Option<&'a str>,
        line: usize,
    ) -> Parsed<()> {
        let mut parent = self.scopes.last().map(|&(index, _)| index);
        if name.id == name.written {
            for part in name.id.split('.') {
                if part.is_empty() {
                    return Err("empty part in a namespace path");
                }
                parent = Some(self.namespace(parent, part));
            }
        } else {
            parent = Some(self.namespace(parent, name.id));
        }
        let index = parent.unwrap();
        if let Some(label) = label {
            self.diagram.namespaces[index].label = Some(label);
        }
        self.scopes.push((index, line));
        Ok(())
    }

    fn namespace(&mut self, parent: Option<usize>, name: &'a str) -> usize {
        *self.namespaces.entry((parent, name)).or_insert_with(|| {
            let index = self.diagram.namespaces.len();
            self.diagram.namespaces.push(Namespace {
                name,
                label: None,
                parent,
            });
            index
        })
    }

    fn close_namespaces(&mut self, mut text: &'a str) -> Parsed<()> {
        while take(&mut text, "}") {
            self.scopes
                .pop()
                .ok_or("closing '}' without an open namespace")?;
        }
        finish(text)
    }

    fn qualified(&self, name: &'a str, mut parent: Option<usize>) -> Option<(usize, &'a str)> {
        let (path, leaf) = name.rsplit_once('.')?;
        for part in path.split('.') {
            parent = self.namespaces.get(&(parent, part)).copied();
            parent?;
        }
        Some((parent?, leaf))
    }

    fn finish(mut self) -> Result<Diagram<'a>, String> {
        let mut scoped = BTreeMap::new();
        for (&(namespace, name), &index) in &self.ids {
            if namespace.is_some() && self.declared[index] {
                scoped
                    .entry(name)
                    .and_modify(|target| *target = None)
                    .or_insert(Some(index));
            }
        }
        let mut target: Vec<_> = (0..self.diagram.nodes.len()).collect();
        for (&(namespace, id), &index) in &self.ids {
            if namespace.is_some() || self.declared[index] {
                continue;
            }
            let qualified = self
                .qualified(id, None)
                .and_then(|(scope, leaf)| self.ids.get(&(Some(scope), leaf)).copied());
            target[index] = match (qualified, scoped.get(id).copied()) {
                (Some(target), _) | (_, Some(Some(target))) => target,
                (_, Some(None)) => {
                    return Err(format!(
                        "ambiguous class '{id}'; use its namespace-qualified name"
                    ));
                }
                _ => index,
            };
        }
        let default = self.styles.get("default").copied().unwrap_or_default();
        let mut styles = vec![default; target.len()];
        for (node, class) in self.classes {
            if let Some(&style) = self.styles.get(class) {
                styles[target[node]].apply(style);
            }
        }
        for (node, color) in self.direct_colors {
            let style = &mut styles[target[node]];
            style.color = style.color.overlay(color);
        }
        let mut nodes: Vec<_> = std::mem::take(&mut self.diagram.nodes)
            .into_iter()
            .map(Some)
            .collect();
        for (index, &into) in target.iter().enumerate() {
            if into != index {
                let mut source = nodes[index].take().unwrap();
                let node = nodes[into].as_mut().unwrap();
                node.annotations.append(&mut source.annotations);
                node.fields.append(&mut source.fields);
                node.methods.append(&mut source.methods);
                node.notes.append(&mut source.notes);
                node.generic = node.generic.or(source.generic);
                node.style.apply_frame(source.style);
            }
        }
        let mut output_index = vec![0; nodes.len()];
        for (index, node) in nodes.into_iter().enumerate() {
            if let Some(mut node) = node {
                output_index[index] = self.diagram.nodes.len();
                styles[index].apply_frame(node.style);
                node.style = styles[index];
                self.diagram.nodes.push(node);
            }
        }
        for edge in &mut self.diagram.relationships {
            edge.left = output_index[target[edge.left]];
            edge.right = output_index[target[edge.right]];
        }
        Ok(self.diagram)
    }
}

pub fn link(text: &mut &str) -> Option<Link> {
    if take(text, "--") {
        Some(Link::Solid)
    } else if take(text, "..") {
        Some(Link::Dashed)
    } else {
        None
    }
}

fn name_list<'a, G: Grammar>(text: &mut &'a str) -> Parsed<Vec<Name<'a>>> {
    let mut names = vec![name::<G>(text)?];
    while take(text, ",") {
        names.push(name::<G>(text)?);
    }
    Ok(names)
}

pub fn inline_style<'a, G: Grammar>(text: &mut &'a str) -> Parsed<Option<&'a str>> {
    if take(text, ":::") {
        Ok(Some(name::<G>(text)?.id))
    } else {
        Ok(None)
    }
}

fn card_style(text: &str) -> Parsed<CardStyle> {
    let mut style = CardStyle {
        color: Style::css(text),
        ..CardStyle::default()
    };
    let mut found = false;
    for property in text.trim_end_matches(';').split(',') {
        let Some((key, value)) = property.split_once(':') else {
            continue;
        };
        found = true;
        let value = value.trim();
        let number = || value.trim_end_matches("px").trim().parse::<f64>().ok();
        match key.trim() {
            "stroke-width" => style.heavy = number().map(|width| width >= 2.0),
            "stroke-dasharray" => style.dashed = Some(!matches!(value, "none" | "0" | "0px")),
            "rx" | "ry" => style.rounded = number().map(|radius| radius > 0.0),
            _ => {}
        }
    }
    if found {
        Ok(style)
    } else {
        Err("expected CSS property:value pairs")
    }
}

pub fn keyword(text: &str) -> &str {
    text.split(|c: char| c.is_whitespace() || matches!(c, ':' | '{'))
        .next()
        .unwrap()
}

pub fn name<'a, G: Grammar>(text: &mut &'a str) -> Parsed<Name<'a>> {
    let source = *text;
    if let Some(id) = quoted(text, G::IDENTIFIER_QUOTE)? {
        nonempty(id, "empty quoted identifier")?;
        return Ok(Name {
            id,
            written: &source[..id.len() + 2],
        });
    }
    let mut end = 0;
    for (index, c) in source.char_indices() {
        let valid =
            c.is_alphanumeric() || matches!(c, '_' | '-' | '.') || G::extra_identifier_character(c);
        if !valid
            || (c == '-' && source[index..].starts_with("--"))
            || (c == '.' && source[index..].starts_with(".."))
        {
            break;
        }
        end = index + c.len_utf8();
    }
    if end == 0 {
        return Err("expected a class or entity identifier");
    }
    let (id, rest) = source.split_at(end);
    *text = rest.trim_start();
    Ok(Name { id, written: id })
}

fn block_line<'a>(text: &'a str, delimiters: &[char]) -> Parsed<(&'a str, Option<&'a str>)> {
    let mut quote = None;
    for (index, c) in text.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None => match c {
                c if delimiters.contains(&c) => quote = Some(c),
                '{' => return Err("nested member blocks are not supported"),
                '}' => {
                    return Ok((
                        text[..index].trim_end(),
                        Some(text[index + 1..].trim_start()),
                    ));
                }
                _ => {}
            },
        }
    }
    if quote.is_some() {
        return Err("unclosed quotes in member block");
    }
    Ok((text, None))
}

pub fn quoted<'a>(text: &mut &'a str, delimiter: char) -> Parsed<Option<&'a str>> {
    let Some(body) = text.strip_prefix(delimiter) else {
        return Ok(None);
    };
    let (value, rest) = body.split_once(delimiter).ok_or("unclosed quoted text")?;
    *text = rest.trim_start();
    Ok(Some(value))
}

pub fn nonempty<'a>(value: &'a str, message: &'static str) -> Parsed<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        Err(message)
    } else {
        Ok(value)
    }
}

pub fn finish(text: &str) -> Parsed<()> {
    if text.is_empty() {
        Ok(())
    } else {
        Err("unexpected text after statement")
    }
}

pub fn take(text: &mut &str, token: &str) -> bool {
    let Some(rest) = text.strip_prefix(token) else {
        return false;
    };
    *text = rest.trim_start();
    true
}

pub fn phrase(text: &mut &str, phrase: &str) -> bool {
    let mut rest = *text;
    for (index, word) in phrase.split_ascii_whitespace().enumerate() {
        if index > 0 {
            let next = rest.trim_start();
            if next.len() == rest.len() {
                return false;
            }
            rest = next;
        }
        let Some((prefix, tail)) = rest.split_at_checked(word.len()) else {
            return false;
        };
        if !prefix.eq_ignore_ascii_case(word) {
            return false;
        }
        rest = tail;
    }
    if rest
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace() && !matches!(c, '|' | '}' | '{' | '"' | '`' | ','))
        && !["--", "..", ".-", "-."]
            .into_iter()
            .any(|link| rest.starts_with(link))
    {
        return false;
    }
    *text = rest.trim_start();
    true
}
