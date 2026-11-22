use std::borrow::Cow;
use std::collections::VecDeque;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::Options;
use crate::charts::shared::Direction;
use crate::color::Style;

mod groups;
pub(super) mod image;
mod layout;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(render_image);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (direction, graph) = parse(source)?;
        graph.render(direction, options)
    }
}

fn render_image(
    source: &str,
    options: &Options,
    scene: &mut crate::graphics::Scene,
) -> Result<(), String> {
    let (direction, graph) = parse(source)?;
    graph.image(direction, options, scene)
}

#[derive(Clone, Copy)]
struct Label<'a> {
    text: &'a str,
    columns: usize,
    rows: usize,
    breaks: bool,
    note: Option<&'a str>,
}

impl<'a> Label<'a> {
    fn new(text: &'a str) -> Result<Self, String> {
        Self::measure(text, false)
    }

    fn measure(text: &'a str, breaks: bool) -> Result<Self, String> {
        if text
            .chars()
            .any(|ch| ch.is_control() && !(breaks && ch == '\n'))
        {
            return Err("control characters in labels are not supported".into());
        }
        let mut label = Self {
            text,
            columns: 0,
            rows: 0,
            breaks,
            note: None,
        };
        for line in label.lines() {
            if line.chars().any(char::is_control) {
                return Err("control characters in labels are not supported".into());
            }
            if line.starts_with(|ch: char| ch.width() == Some(0)) {
                return Err("label must start with a visible character".into());
            }
            label.columns = label.columns.max(line.width());
            label.rows += 1;
        }
        if label.columns == 0 {
            return Err("expected a visible label".into());
        }
        Ok(label)
    }

    fn lines(self) -> impl Iterator<Item = Cow<'a, str>> {
        let mut rest = Some(self.text);
        std::iter::from_fn(move || {
            let text = rest.take()?;
            if self.breaks {
                for (index, token) in text.match_indices(['<', '\n']) {
                    let tail = &text[index + 1..];
                    if token == "\n" {
                        rest = Some(tail);
                        return Some(text[..index].trim());
                    }
                    if let Some(after_tag) = line_break(tail) {
                        rest = Some(after_tag);
                        return Some(text[..index].trim());
                    }
                }
                Some(text.trim())
            } else {
                Some(text)
            }
        })
        .map(move |line| {
            if self.breaks {
                label_text(line)
            } else {
                Cow::Borrowed(line)
            }
        })
        .chain(self.note.map(Cow::Borrowed))
    }

    fn with_note(mut self, note: &'a str) -> Self {
        self.note = Some(note);
        self.columns = self.columns.max(note.width());
        self.rows += 1;
        self
    }
}

fn line_break(tail: &str) -> Option<&str> {
    let tail = tail.strip_prefix('/').unwrap_or(tail);
    let (name, tail) = tail.split_at_checked(2)?;
    if !name.eq_ignore_ascii_case("br") {
        return None;
    }
    let tail = tail.trim_start_matches(' ');
    tail.strip_prefix('>').or_else(|| tail.strip_prefix("/>"))
}

fn formatting_tag(tail: &str) -> Option<usize> {
    let end = tail.bytes().take(10).position(|byte| byte == b'>')?;
    let name = tail[..end].strip_prefix('/').unwrap_or(&tail[..end]);
    [
        "b", "i", "em", "strong", "s", "u", "code", "sub", "sup", "mark", "span",
    ]
    .into_iter()
    .any(|tag| name.eq_ignore_ascii_case(tag))
    .then_some(end + 1)
}

fn label_text(text: &str) -> Cow<'_, str> {
    let mut end = 0;
    let mut output = String::new();
    for (index, marker) in text.match_indices(['<', '#', '&']) {
        if index < end {
            continue;
        }
        let tail = &text[index + 1..];
        if marker == "<" {
            if let Some(bytes) = formatting_tag(tail) {
                output.push_str(&text[end..index]);
                end = index + 1 + bytes;
            }
        } else if let Some((replacement, bytes)) = label_entity(tail, marker == "#") {
            output.push_str(&text[end..index]);
            output.push(replacement);
            end = index + 1 + bytes;
        }
    }
    if end == 0 {
        Cow::Borrowed(text)
    } else {
        output.push_str(&text[end..]);
        Cow::Owned(output)
    }
}

fn label_entity(tail: &str, mermaid: bool) -> Option<(char, usize)> {
    let end = tail
        .bytes()
        .take(16)
        .position(|byte| !(byte.is_ascii_alphanumeric() || byte == b'#'))?;
    if tail.as_bytes()[end] != b';' {
        return None;
    }
    let name = tail[..end].strip_prefix('#').unwrap_or(&tail[..end]);
    let value = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "ndash" => '–',
        "mdash" => '—',
        "hellip" => '…',
        _ if mermaid || tail.starts_with('#') => {
            let code = if let Some(hex) = name.strip_prefix(['x', 'X']) {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                name.parse::<u32>().ok()?
            };
            char::from_u32(code)?
        }
        _ => return None,
    };
    Some((value, end + 1))
}

#[derive(Clone, Copy)]
struct Edge<'a> {
    from: usize,
    to: usize,
    label: Option<Label<'a>>,
    link: Link,
    style: Style,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
    Invisible,
}

#[derive(Clone, Copy)]
enum Tip {
    Open,
    Arrow,
    Circle,
    Cross,
}

#[derive(Clone, Copy)]
struct Link {
    style: EdgeStyle,
    start: Tip,
    end: Tip,
}

struct ParsedLink<'a> {
    id: Option<&'a str>,
    label: Option<Label<'a>>,
    link: Link,
}

impl Link {
    fn arrow(style: EdgeStyle) -> Self {
        Self {
            style,
            start: Tip::Open,
            end: Tip::Arrow,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Outline {
    Rectangle,
    FixedRectangle,
    Decision,
    Rounded,
    Circle,
    Database,
    Subroutine,
    Stadium,
    Flag,
    Hexagon,
    SlantedRight,
    SlantedLeft,
    Trapezoid,
    TrapezoidInverted,
    DoubleCircle,
    Text,
    Fork,
    Initial,
    Final,
    Synchronization { vertical: bool },
    Note,
    Brace { left: bool, right: bool },
    Triangle { down: bool },
    Document { lined: bool },
}

#[derive(Clone, Copy)]
struct Shape {
    outline: Outline,
    note: Option<&'static str>,
}

impl Shape {
    const RECTANGLE: Self = Self::new(Outline::Rectangle);

    const fn new(outline: Outline) -> Self {
        Self {
            outline,
            note: None,
        }
    }

    fn annotated(outline: Outline, note: &'static str) -> Self {
        Self {
            outline,
            note: Some(note),
        }
    }
}

#[derive(Clone, Copy)]
struct Node<'a> {
    label: Label<'a>,
    shape: Shape,
    body: Option<Panel<'a>>,
    port_name: Option<Label<'a>>,
    style: Style,
}

struct NodeDeclaration<'a> {
    label: Option<Label<'a>>,
    shape: Option<Shape>,
}

#[derive(Clone, Copy)]
struct Panel<'a> {
    drawing: &'a layout::Drawing,
    columns: usize,
    rows: usize,
}

impl<'a> Panel<'a> {
    fn new(drawing: &'a layout::Drawing) -> Self {
        Self {
            drawing,
            columns: drawing
                .plain
                .lines()
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or_default(),
            rows: drawing.plain.lines().count(),
        }
    }
}

impl<'a> Node<'a> {
    fn new(label: Label<'a>, shape: Shape) -> Self {
        Self {
            label,
            shape,
            body: None,
            port_name: None,
            style: Style::default(),
        }
    }

    fn dimensions(self) -> (usize, usize) {
        let port = self.port_name.map_or(0, |name| name.columns);
        let note = self.shape.note.map_or(0, str::len);
        let rows = self.label.rows + usize::from(self.port_name.is_some()) + usize::from(note > 0);
        if let Some(body) = self.body {
            return (
                self.label.columns.max(port).max(body.columns) + 4,
                rows + body.rows + 4,
            );
        }
        let (side, extra_rows) = match self.shape.outline {
            Outline::Circle | Outline::DoubleCircle => (6, 2),
            Outline::Stadium | Outline::Subroutine | Outline::Triangle { .. } => (6, 0),
            Outline::Database | Outline::Document { lined: true } => (4, 1),
            _ => (4, 0),
        };
        (
            self.label.columns.max(port).max(note) + side,
            rows + extra_rows + 2,
        )
    }
}

#[derive(Default)]
pub(crate) struct Graph<'a> {
    nodes: Vec<Node<'a>>,
    edges: Vec<Edge<'a>>,
}

impl<'a> Graph<'a> {
    pub(crate) fn from_parts(
        nodes: impl IntoIterator<Item = (&'a str, bool)>,
        edges: impl IntoIterator<Item = (usize, usize, Option<&'a str>)>,
    ) -> Result<Self, String> {
        let nodes = nodes
            .into_iter()
            .map(|(label, configurable_corners)| {
                let shape = if configurable_corners {
                    Shape::RECTANGLE
                } else {
                    Shape::new(Outline::FixedRectangle)
                };
                Ok(Node::new(Label::new(label)?, shape))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let edges = edges
            .into_iter()
            .map(|(from, to, label)| {
                if from >= nodes.len() || to >= nodes.len() {
                    return Err("graph edge refers to an unknown node".into());
                }
                Ok(Edge {
                    from,
                    to,
                    label: label.map(Label::new).transpose()?,
                    link: Link::arrow(EdgeStyle::Solid),
                    style: Style::default(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self { nodes, edges })
    }

    pub(crate) fn set_node_style(&mut self, index: usize, style: Style) {
        self.nodes[index].style = style;
    }

    pub(crate) fn render(
        &self,
        direction: Direction,
        options: &Options,
        max_branch_gap: usize,
    ) -> Result<String, String> {
        if self.nodes.is_empty() {
            return Err("graph has no nodes".into());
        }
        let topology = Topology::new(self);
        layout::render(self, &topology, direction, options, max_branch_gap)
    }
}

fn parse(source: &str) -> Result<(Direction, groups::Document<'_>), String> {
    let (first, rest) = source.split_once('\n').unwrap_or((source, ""));
    let (header, body, first_line) = match first.split_once(';') {
        Some((header, _)) => (header, &source[header.len() + 1..], 1),
        None => (first, rest, 2),
    };
    let mut words = header.split_whitespace();
    if !matches!(
        words.next(),
        Some("flowchart" | "graph" | "flowchart-v2" | "flowchart-elk")
    ) {
        return Err("expected a flowchart or graph header".into());
    }
    let direction = parse_direction(words.next().unwrap_or("TB"))?;
    if words.next().is_some() {
        return Err("unexpected text after flowchart direction".into());
    }
    let mut graph = groups::Document::default();
    let (mut start, mut end, mut line_number) = (0, 0, first_line);
    let mut quoted = false;
    let mut attributes = 0usize;
    let mut physical_lines = body.split_inclusive('\n').enumerate().peekable();
    while let Some((index, line)) = physical_lines.next() {
        end += line.len();
        let mut chars = line.as_bytes().iter().copied().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                b'"' => quoted = !quoted,
                b'%' if !quoted && chars.peek() == Some(&b'%') => break,
                b'@' if !quoted && chars.peek() == Some(&b'{') => {
                    chars.next();
                    attributes += 1;
                }
                b'{' if !quoted && attributes > 0 => attributes += 1,
                b'}' if !quoted && attributes > 0 => attributes -= 1,
                _ => {}
            }
        }
        let continues = physical_lines
            .peek()
            .is_some_and(|(_, next)| link_operator(next.trim_start()).is_some());
        if !quoted && attributes == 0 && !continues {
            parse_line(&body[start..end], &mut graph)
                .map_err(|err| format!("flowchart line {line_number}: {err}"))?;
            start = end;
            line_number = first_line + index + 1;
        }
    }
    if quoted {
        return Err(format!(
            "flowchart line {line_number}: unterminated quoted label"
        ));
    }
    if attributes != 0 {
        return Err(format!(
            "flowchart line {line_number}: unterminated attributes"
        ));
    }
    graph.finish()?;
    Ok((direction, graph))
}

fn parse_direction(word: &str) -> Result<Direction, String> {
    match word {
        "TD" | "TB" | "v" => Ok(Direction::Down),
        "BT" | "^" => Ok(Direction::Up),
        "LR" | ">" => Ok(Direction::Right),
        "RL" | "<" => Ok(Direction::Left),
        _ => Err("expected flowchart direction LR, RL, TD, TB or BT".into()),
    }
}

fn parse_line<'a>(line: &'a str, graph: &mut groups::Document<'a>) -> Result<(), String> {
    let mut input = Line(line);
    while !input.rest().is_empty() && !input.rest().starts_with("%%") {
        if input.take(";") {
            continue;
        }
        if input.keyword("subgraph") {
            let (id, title) = input.group_title()?;
            graph.open(id, title)?;
        } else if input.keyword("end") {
            graph.close()?;
        } else if input.keyword("direction") {
            let rest = input.rest();
            let end = rest
                .find(|ch: char| ch.is_whitespace() || ch == ';')
                .unwrap_or(rest.len());
            input.0 = &rest[end..];
            graph.direction(parse_direction(&rest[..end])?)?;
        } else if input.keyword("style") {
            let targets = input.names("expected a style target")?;
            graph.style_nodes(&targets, Style::css(input.statement_text()))?;
        } else if input.keyword("classDef") {
            let names = input.names("expected a class name")?;
            graph.define_classes(&names, Style::css(input.statement_text()));
        } else if input.keyword("class") {
            let targets = input.names("expected a class target")?;
            let classes = input.names("expected a class name")?;
            for target in targets {
                graph.assign_classes(target, classes.iter().copied());
            }
        } else if input.keyword("linkStyle") {
            let targets = input.names("expected a link index or default")?;
            if input.keyword("interpolate") {
                input.id();
            }
            let style = Style::css(input.statement_text());
            for target in targets {
                let index = if target == "default" {
                    None
                } else {
                    Some(
                        target
                            .parse()
                            .map_err(|_| "expected a link index or default")?,
                    )
                };
                graph.style_edge(index, style);
            }
        } else if input.keyword("click") || input.take("accTitle:") || input.take("accDescr:") {
            input.statement_text();
        } else if !input.edge_metadata(graph)? {
            let mut from = input.nodes(graph, true)?;
            while let Some(link) = input.edge()? {
                let to = input.nodes(graph, false)?;
                graph.connect(&from, &to, link)?;
                from = to;
            }
        }
        if !input.at_statement_end() {
            return Err("expected a supported edge or a statement separator".into());
        }
    }
    Ok(())
}

struct Line<'a>(&'a str);

impl<'a> Line<'a> {
    fn rest(&self) -> &'a str {
        self.0.trim_start()
    }

    fn take(&mut self, token: &str) -> bool {
        match self.rest().strip_prefix(token) {
            Some(rest) => {
                self.0 = rest;
                true
            }
            None => false,
        }
    }

    fn keyword(&mut self, keyword: &str) -> bool {
        if let Some(rest) = self.rest().strip_prefix(keyword)
            && (rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with(';'))
        {
            self.0 = rest;
            return true;
        }
        false
    }

    fn at_statement_end(&self) -> bool {
        let tail = self.rest();
        tail.is_empty() || tail.starts_with(';') || tail.starts_with("%%")
    }

    fn statement_text(&mut self) -> &'a str {
        let text = self.rest();
        let mut quote = None;
        let mut brackets = 0usize;
        let mut escaped = false;
        for (index, ch) in text.char_indices() {
            if std::mem::take(&mut escaped) {
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if quote.is_some() {
                if quote == Some(ch) {
                    quote = None;
                }
            } else {
                match ch {
                    '"' | '\'' => quote = Some(ch),
                    '(' => brackets += 1,
                    ')' => brackets = brackets.saturating_sub(1),
                    ';' if brackets == 0 => {
                        self.0 = &text[index..];
                        return &text[..index];
                    }
                    '%' if brackets == 0 && text[index..].starts_with("%%") => {
                        self.0 = &text[index..];
                        return &text[..index];
                    }
                    _ => {}
                }
            }
        }
        self.0 = "";
        text
    }

    fn edge_metadata(&mut self, graph: &groups::Document<'_>) -> Result<bool, String> {
        let saved = self.0;
        let id = self.id();
        let known_edge = graph.is_edge_id(id);
        if !id.is_empty() && self.take("@{") {
            loop {
                let (key, _) = self.attribute()?;
                if matches!(key, "shape" | "label")
                    || (!known_edge && !matches!(key, "animate" | "animation" | "curve" | "step"))
                {
                    break;
                }
                if self.take("}") {
                    return Ok(true);
                }
                if !self.attribute_separator() {
                    return Err("expected a comma or } after the edge attribute".into());
                }
            }
        }
        self.0 = saved;
        Ok(false)
    }

    fn edge(&mut self) -> Result<Option<ParsedLink<'a>>, String> {
        use EdgeStyle::{Dotted, Solid, Thick};
        let saved = self.0;
        let id = self.id();
        let id = if !id.is_empty() && self.take("@") {
            Some(id)
        } else {
            self.0 = saved;
            None
        };
        if let Some((link, rest)) = link_operator(self.rest()) {
            self.0 = rest;
            let label = self.label("|", "|")?;
            if link.style == EdgeStyle::Invisible && label.is_some() {
                return Err("an invisible link cannot display a label".into());
            }
            return Ok(Some(ParsedLink { id, label, link }));
        }
        let (start, rest) = edge_start(self.rest());
        self.0 = rest;
        for (opening, marker, style) in
            [("--", '-', Solid), ("-.", '.', Dotted), ("==", '=', Thick)]
        {
            if self.take(opening) {
                let input = self.rest();
                let (text, link, rest) = if let Some(quoted) = input.strip_prefix('"') {
                    let (text, rest) = quoted
                        .split_once('"')
                        .ok_or("unterminated quoted edge label")?;
                    let (link, rest) = inline_link_end(rest.trim_start(), style)
                        .ok_or("expected a connector after the edge label")?;
                    (text, link, rest)
                } else {
                    let (index, link, rest) = input
                        .match_indices(marker)
                        .filter(|&(index, _)| {
                            index == 0 || input.as_bytes()[index - 1] != marker as u8
                        })
                        .find_map(|(index, _)| {
                            inline_link_end(&input[index..], style)
                                .map(|(link, rest)| (index, link, rest))
                        })
                        .ok_or("unterminated edge label")?;
                    let text = &input[..index];
                    if text.contains([';', '"', '|', '[', ']', '{', '}']) {
                        return Err("unsupported edge label syntax".into());
                    }
                    (text, link, rest)
                };
                self.0 = rest;
                let link = Link { start, ..link };
                return Ok(Some(ParsedLink {
                    id,
                    label: Some(plain_label(text)?),
                    link,
                }));
            }
        }
        self.0 = saved;
        Ok(None)
    }

    fn id(&mut self) -> &'a str {
        let input = self.rest();
        let mut chars = input.char_indices().peekable();
        let mut end = 0;
        while let Some((index, ch)) = chars.next() {
            if is_id_char(ch)
                || (matches!(ch, '-' | '.')
                    && index > 0
                    && chars.peek().is_some_and(|&(_, c)| is_id_char(c)))
            {
                end = index + ch.len_utf8();
            } else {
                break;
            }
        }
        let (id, rest) = input.split_at(end);
        self.0 = rest;
        id
    }

    fn names(&mut self, error: &'static str) -> Result<Vec<&'a str>, String> {
        let mut names = Vec::new();
        loop {
            let name = self.id();
            if name.is_empty() {
                return Err(error.into());
            }
            names.push(name);
            if !self.take(",") {
                return Ok(names);
            }
        }
    }

    fn group_title(&mut self) -> Result<(Option<&'a str>, Label<'a>), String> {
        let input = self.rest();
        if let Some(quoted) = input.strip_prefix('"') {
            let (title, tail) = quoted
                .split_once('"')
                .ok_or("unterminated subgraph title")?;
            self.0 = tail;
            let id = (!title.is_empty() && Line(title).id() == title).then_some(title);
            return Ok((id, plain_label(title)?));
        }
        let id = self.id();
        if !id.is_empty() {
            if let Some(title) = self.label("[", "]")? {
                return Ok((Some(id), title));
            }
            if self.at_statement_end() {
                return Ok((Some(id), Label::new(id)?));
            }
        }
        self.0 = input;
        let title = self.statement_text();
        Ok((None, plain_label(title)?))
    }

    fn nodes(
        &mut self,
        graph: &mut groups::Document<'a>,
        standalone: bool,
    ) -> Result<Vec<usize>, String> {
        let mut nodes = vec![self.node(graph, standalone)?];
        while self.take("&") {
            nodes.push(self.node(graph, standalone)?);
        }
        Ok(nodes)
    }

    fn node(
        &mut self,
        graph: &mut groups::Document<'a>,
        standalone: bool,
    ) -> Result<usize, String> {
        let id = self.id();
        if id.is_empty()
            || matches!(
                id,
                "subgraph"
                    | "end"
                    | "direction"
                    | "style"
                    | "class"
                    | "classDef"
                    | "linkStyle"
                    | "click"
                    | "accTitle"
                    | "accDescr"
            )
        {
            return Err("expected a node ID".into());
        }
        let mut node = self.attributes()?;
        if node.is_none() {
            for (open, closings, shapes) in [
                (
                    "[/",
                    ["/]", "\\]"],
                    [Outline::SlantedRight, Outline::Trapezoid],
                ),
                (
                    "[\\",
                    ["\\]", "/]"],
                    [Outline::SlantedLeft, Outline::TrapezoidInverted],
                ),
            ] {
                if let Some((label, delimiter)) = self.label_choices(open, &closings)? {
                    node = Some(NodeDeclaration {
                        label: Some(label),
                        shape: Some(Shape::new(shapes[delimiter])),
                    });
                    break;
                }
            }
        }
        for (open, close, shape) in [
            ("[[", "]]", Outline::Subroutine),
            ("[(", ")]", Outline::Database),
            ("([", "])", Outline::Stadium),
            ("(((", ")))", Outline::DoubleCircle),
            ("((", "))", Outline::Circle),
            ("{{", "}}", Outline::Hexagon),
            ("[", "]", Outline::Rectangle),
            ("(", ")", Outline::Rounded),
            ("{", "}", Outline::Decision),
            (">", "]", Outline::Flag),
        ] {
            if node.is_some() {
                break;
            }
            if let Some(label) = self.label(open, close)? {
                node = Some(NodeDeclaration {
                    label: Some(label),
                    shape: Some(Shape::new(shape)),
                });
                break;
            }
        }
        if let Some(revision) = self.attributes()? {
            let declaration = node.get_or_insert(NodeDeclaration {
                label: None,
                shape: None,
            });
            declaration.label = revision.label.or(declaration.label);
            declaration.shape = revision.shape.or(declaration.shape);
        }
        while self.take(":::") {
            graph.assign_classes(id, self.names("expected Mermaid class name after :::")?);
        }
        graph.node(id, node, standalone && self.at_statement_end())
    }

    fn attributes(&mut self) -> Result<Option<NodeDeclaration<'a>>, String> {
        if !self.take("@{") {
            return Ok(None);
        }
        let mut shape = None;
        let mut label = None;
        loop {
            let (key, value) = self.attribute()?;
            match key {
                "shape" => shape = Some(Shape::from_name(value)?),
                "label" => label = Some(node_label(value)?),
                "icon" => {
                    shape.get_or_insert(Shape::annotated(Outline::Rounded, "[icon]"));
                }
                "img" => {
                    shape.get_or_insert(Shape::annotated(Outline::Rectangle, "[image]"));
                }
                "look" | "other" | "form" | "pos" | "w" | "h" | "constraint" | "labelType" => {}
                _ => return Err(format!("unsupported node attribute {key}")),
            }
            if self.take("}") {
                break;
            }
            if !self.attribute_separator() {
                return Err("expected a comma, newline or } after the node attribute".into());
            }
        }
        Ok(Some(NodeDeclaration { label, shape }))
    }

    fn attribute(&mut self) -> Result<(&'a str, &'a str), String> {
        let leading = &self.0[..self.0.len() - self.rest().len()];
        let indent = leading
            .rsplit_once('\n')
            .map_or(0, |(_, spaces)| spaces.len());
        let key = self.id();
        if key.is_empty() || !self.take(":") {
            return Err("expected a Mermaid attribute and colon".into());
        }
        let text = self.rest();
        let value = if let Some(quote @ ('"' | '\'')) = text.chars().next() {
            let (value, rest) = text[1..]
                .split_once(quote)
                .ok_or("unterminated attribute")?;
            self.0 = rest;
            value
        } else if let Some((marker, block)) = text.split_once('\n')
            && matches!(marker.trim_end(), "|" | "|-" | "|+")
        {
            let mut bytes = 0;
            for line in block.split_inclusive('\n') {
                let trimmed = line.trim_start_matches([' ', '\t']);
                if !trimmed.trim().is_empty() && line.len() - trimmed.len() <= indent {
                    break;
                }
                bytes += line.len();
            }
            if bytes == 0 {
                self.0 = &text[marker.len()..];
                ""
            } else {
                let end = bytes - usize::from(block.as_bytes()[bytes - 1] == b'\n');
                self.0 = &block[end..];
                &block[..end]
            }
        } else {
            let end = text
                .find([',', '}', '\n'])
                .ok_or("unterminated attributes")?;
            let (value, rest) = text.split_at(end);
            self.0 = rest;
            value.trim()
        };
        Ok((key, value))
    }

    fn attribute_separator(&mut self) -> bool {
        self.take(",") || self.0[..self.0.len() - self.rest().len()].contains('\n')
    }

    fn label(&mut self, opening: &str, closing: &str) -> Result<Option<Label<'a>>, String> {
        self.label_choices(opening, &[closing])
            .map(|label| label.map(|(label, _)| label))
    }

    fn label_choices(
        &mut self,
        opening: &str,
        closings: &[&str],
    ) -> Result<Option<(Label<'a>, usize)>, String> {
        if !self.take(opening) {
            return Ok(None);
        }
        let input = self.rest();
        let (text, rest, delimiter) = if let Some(quoted) = input.strip_prefix('"') {
            let (text, rest) = quoted.split_once('"').ok_or("unterminated quoted label")?;
            let rest = rest.trim_start();
            let (delimiter, rest) = closings
                .iter()
                .enumerate()
                .find_map(|(i, closing)| rest.strip_prefix(closing).map(|rest| (i, rest)))
                .ok_or("expected a closing label delimiter")?;
            (text, rest, delimiter)
        } else {
            let (index, delimiter, closing) = closings
                .iter()
                .enumerate()
                .filter_map(|(i, closing)| input.find(closing).map(|at| (at, i, closing)))
                .min()
                .ok_or("unterminated label")?;
            let text = &input[..index];
            if text.contains(['"', '|']) {
                return Err("unsupported label syntax".into());
            }
            (text, &input[index + closing.len()..], delimiter)
        };
        self.0 = rest;
        Ok(Some((node_label(text)?, delimiter)))
    }
}

fn link_operator(input: &str) -> Option<(Link, &str)> {
    if input.starts_with("~~~") {
        return Some((
            Link {
                style: EdgeStyle::Invisible,
                start: Tip::Open,
                end: Tip::Open,
            },
            input.trim_start_matches('~'),
        ));
    }
    let (start, text) = edge_start(input);
    let marker = *text.as_bytes().first()?;
    if marker == b'.' {
        let (end, tail) = edge_tip(dotted_tail(text)?);
        return Some((
            Link {
                style: EdgeStyle::Dotted,
                start,
                end,
            },
            tail,
        ));
    }
    if !matches!(marker, b'-' | b'=') {
        return None;
    }
    let mut end = text.bytes().take_while(|&ch| ch == marker).count();
    let style = if marker == b'=' {
        EdgeStyle::Thick
    } else if end == 1 && text.as_bytes().get(end) == Some(&b'.') {
        end = text.len() - dotted_tail(&text[end..])?.len();
        EdgeStyle::Dotted
    } else {
        EdgeStyle::Solid
    };
    let (tip, rest) = edge_tip(&text[end..]);
    if !matches!(style, EdgeStyle::Dotted)
        && end
            < if matches!((start, tip), (Tip::Open, Tip::Open)) {
                3
            } else {
                2
            }
    {
        return None;
    }
    Some((
        Link {
            style,
            start,
            end: tip,
        },
        rest,
    ))
}

fn edge_start(text: &str) -> (Tip, &str) {
    match text.as_bytes().first() {
        Some(b'<') => (Tip::Arrow, &text[1..]),
        Some(b'o') => (Tip::Circle, &text[1..]),
        Some(b'x') => (Tip::Cross, &text[1..]),
        _ => (Tip::Open, text),
    }
}

fn dotted_tail(text: &str) -> Option<&str> {
    let dots = text.bytes().take_while(|&ch| ch == b'.').count();
    let dashes = text[dots..].bytes().take_while(|&ch| ch == b'-').count();
    (dots > 0 && dashes > 0).then_some(&text[dots + dashes..])
}

fn edge_tip(text: &str) -> (Tip, &str) {
    match text.as_bytes().first() {
        Some(b'>') => (Tip::Arrow, &text[1..]),
        Some(b'o') => (Tip::Circle, &text[1..]),
        Some(b'x') => (Tip::Cross, &text[1..]),
        _ => (Tip::Open, text),
    }
}

fn inline_link_end(text: &str, style: EdgeStyle) -> Option<(Link, &str)> {
    if style == EdgeStyle::Dotted {
        let (end, rest) = edge_tip(dotted_tail(text)?);
        Some((
            Link {
                style,
                start: Tip::Open,
                end,
            },
            rest,
        ))
    } else {
        let (link, rest) = link_operator(text)?;
        (link.style == style && matches!(link.start, Tip::Open)).then_some((link, rest))
    }
}

impl Shape {
    fn from_name(name: &str) -> Result<Self, String> {
        use Outline::*;
        let outline = match name {
            "rect" | "rectangle" | "process" | "proc" | "square" | "squareRect" => Rectangle,
            "rounded" | "rounded-rectangle" | "roundedRect" | "event" => Rounded,
            "stadium" | "pill" | "terminal" => Stadium,
            "circ" | "circle" | "start" | "sm-circ" | "small-circle" => Circle,
            "dbl-circ" | "double-circle" | "doublecircle" | "stop" | "fr-circ"
            | "framed-circle" => DoubleCircle,
            "cyl" | "cylinder" | "database" | "db" | "disk" => Database,
            "subproc" | "subprocess" | "subroutine" | "fr-rect" | "framed-rectangle"
            | "lin-rect" | "lin-proc" | "lined-rectangle" | "lined-process" => Subroutine,
            "diamond" | "diam" | "decision" | "question" | "choice" => Decision,
            "hex" | "hexagon" | "prepare" => Hexagon,
            "flag" | "asymmetric" | "odd" => Flag,
            "lean-r" | "lean-right" | "lean_right" | "in-out" => SlantedRight,
            "lean-l" | "lean-left" | "lean_left" | "out-in" => SlantedLeft,
            "trap-b" | "trapezoid-bottom" | "trapezoid" => Trapezoid,
            "trap-t" | "trapezoid-top" | "inv-trapezoid" | "inv_trapezoid" => TrapezoidInverted,
            "text" | "labelRect" => Text,
            "brace" | "brace-l" | "comment" => Brace {
                left: true,
                right: false,
            },
            "brace-r" => Brace {
                left: false,
                right: true,
            },
            "braces" => Brace {
                left: true,
                right: true,
            },
            "tri" | "triangle" | "extract" => Triangle { down: false },
            "flip-tri" | "flipped-triangle" | "manual-file" => Triangle { down: true },
            "doc" | "document" => Document { lined: false },
            "lin-doc" | "lined-document" => Document { lined: true },
            _ => {
                let (outline, note) = match name {
                    "fork" | "forkJoin" => (Fork, "[fork]"),
                    "join" => (Fork, "[join]"),
                    "anchor" => (Text, "[anchor]"),
                    "tag-rect" | "tagged-rectangle" | "tag-proc" | "tagged-process" => {
                        (Rectangle, "[tagged]")
                    }
                    "tag-doc" | "tagged-document" => (Document { lined: false }, "[tagged]"),
                    "docs" | "st-doc" | "stacked-document" | "documents" => {
                        (Document { lined: true }, "[stacked]")
                    }
                    "st-rect" | "stacked-rectangle" | "processes" | "procs" => {
                        (Subroutine, "[stacked]")
                    }
                    "curv-trap" | "curved-trapezoid" | "display" => (Trapezoid, "[display]"),
                    "h-cyl" | "horizontal-cylinder" | "das" | "direct-access-storage" => {
                        (Database, "[horizontal]")
                    }
                    "lin-cyl" | "lined-cylinder" => (Database, "[lined]"),
                    "data-store" | "datastore" | "stored-data" => (Database, "[stored data]"),
                    "half-rounded-rectangle" | "delay" => (Rounded, "[delay]"),
                    "f-circ" | "filled-circle" | "junction" => (Circle, "[filled]"),
                    "cross-circ" | "crossed-circle" | "summary" => (Circle, "[crossed]"),
                    "card" | "notch-rect" | "notched-rectangle" => (Flag, "[card]"),
                    "loop-limit" | "notch-pent" | "notched-pentagon" => (Hexagon, "[loop limit]"),
                    "sl-rect" | "sloped-rectangle" | "manual-input" => (SlantedLeft, "[input]"),
                    "manual" | "manual-operation" => (Trapezoid, "[manual]"),
                    "div-proc" | "div-rect" | "divided-process" | "divided-rectangle" => {
                        (Subroutine, "[divided]")
                    }
                    "internal-storage" => (Subroutine, "[storage]"),
                    "paper-tape" => (Document { lined: false }, "[paper tape]"),
                    "note" => (Document { lined: false }, "[note]"),
                    "folder" | "directory" => (Rectangle, "[folder]"),
                    "browser" | "win-pane" | "window-pane" => (Rectangle, "[window]"),
                    "console" => (Rectangle, "[console]"),
                    "bucket" => (TrapezoidInverted, "[bucket]"),
                    "bow-rect" | "bow-tie-rectangle" => (Rectangle, "[bow tie]"),
                    "collate" | "hourglass" => (Decision, "[hourglass]"),
                    "priority" => (Flag, "[priority]"),
                    "shaded-process" => (Rectangle, "[shaded]"),
                    "cloud" => (Rounded, "[cloud]"),
                    "person" => (Rounded, "[person]"),
                    "bolt" | "lightning-bolt" | "com-link" => (Flag, "[lightning]"),
                    "bang" => (Circle, "[bang]"),
                    "icon" | "iconCircle" | "iconRounded" | "iconSquare" => (Rounded, "[icon]"),
                    "image" | "img" | "imageSquare" => (Rectangle, "[image]"),
                    _ => return Err(format!("unsupported flowchart shape {name}")),
                };
                return Ok(Self::annotated(outline, note));
            }
        };
        Ok(Self::new(outline))
    }
}

fn plain_label(text: &str) -> Result<Label<'_>, String> {
    let text = text.trim();
    let text = text
        .strip_prefix('`')
        .and_then(|text| text.strip_suffix('`'))
        .unwrap_or(text);
    if text.is_empty() {
        return Err("expected a nonempty label".into());
    }
    if text.split('<').skip(1).any(|tail| {
        line_break(tail).is_none()
            && formatting_tag(tail).is_none()
            && tail.split_once('>').is_some_and(|(tag, _)| {
                tag.trim_start_matches('/')
                    .starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '!')
            })
    }) {
        return Err("unsupported label syntax".into());
    }
    Label::measure(text, true)
}

fn node_label(text: &str) -> Result<Label<'_>, String> {
    if text.trim().is_empty() {
        Label::new(" ")
    } else {
        plain_label(text)
    }
}

fn is_id_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

struct Topology {
    ranks: Vec<usize>,
    outgoing: Vec<Vec<usize>>,
    components: Vec<Vec<usize>>,
}

impl Topology {
    fn new(graph: &Graph<'_>) -> Self {
        Self::from_endpoints(graph.nodes.len(), graph.edges.len(), |index| {
            let edge = &graph.edges[index];
            (edge.from, edge.to)
        })
    }

    fn from_endpoints(
        count: usize,
        edge_count: usize,
        endpoints: impl Fn(usize) -> (usize, usize),
    ) -> Self {
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for index in 0..edge_count {
            let (from, to) = endpoints(index);
            outgoing[from].push(index);
            incoming[to].push(index);
        }

        let mut pending: Vec<_> = incoming.iter().map(Vec::len).collect();
        let mut ready: VecDeque<_> = (0..count).filter(|&node| pending[node] == 0).collect();
        let mut ranks = vec![0; count];
        let mut ranked = vec![false; count];
        let mut roots = 0..count;
        while let Some(from) = ready
            .pop_front()
            .or_else(|| roots.find(|&node| !ranked[node]))
        {
            // When a cycle stalls the queue, rank one node to resume forward traversal.
            ranked[from] = true;
            for &index in &outgoing[from] {
                let (_, to) = endpoints(index);
                if ranked[to] {
                    continue;
                }
                ranks[to] = ranks[to].max(ranks[from] + 1);
                pending[to] -= 1;
                if pending[to] == 0 {
                    ready.push_back(to);
                }
            }
        }
        let mut seen = vec![false; count];
        let mut components = Vec::new();
        for start in 0..count {
            if std::mem::replace(&mut seen[start], true) {
                continue;
            }
            let mut component = vec![start];
            let mut next = 0;
            while let Some(&node) = component.get(next) {
                next += 1;
                for neighbor in outgoing[node]
                    .iter()
                    .map(|&edge| endpoints(edge).1)
                    .chain(incoming[node].iter().map(|&edge| endpoints(edge).0))
                {
                    if !std::mem::replace(&mut seen[neighbor], true) {
                        component.push(neighbor);
                    }
                }
            }
            component.sort_unstable();
            components.push(component);
        }
        Self {
            ranks,
            outgoing,
            components,
        }
    }
}
