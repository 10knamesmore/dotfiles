use rustc_hash::FxHashMap;

use crate::color::Style;

use crate::charts::shared::block_layout::{
    Diagram, EAST, Edge, Grid, Kind, Line, NORTH, Node, PaintStatement, SOUTH, Shape, Tip, WEST,
    label, paint_statement, statements,
};

struct Cursor<'a>(&'a str);

impl<'a> Cursor<'a> {
    fn take(&mut self, token: &str) -> bool {
        if let Some(rest) = self.0.strip_prefix(token) {
            self.0 = rest;
            true
        } else {
            false
        }
    }

    fn spaces(&mut self) {
        self.0 = self.0.trim_start();
    }

    fn id(&mut self) -> Result<&'a str, String> {
        let mut end = 0;
        for (i, ch) in self.0.char_indices() {
            if !(ch.is_alphanumeric()
                || matches!(ch, '_' | '-' | '.')
                || (!ch.is_ascii() && !ch.is_whitespace() && !ch.is_control()))
                || self.0[i..].starts_with("--")
                || self.0[i..].starts_with("-.")
            {
                break;
            }
            end = i + ch.len_utf8();
        }
        if end == 0 {
            return Err(format!("expected block identifier near `{}`", self.0));
        }
        let id = &self.0[..end];
        self.0 = &self.0[end..];
        Ok(id)
    }

    fn count(&mut self) -> Result<usize, String> {
        let n = self.0.bytes().take_while(u8::is_ascii_digit).count();
        let count = self.0[..n]
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=4096).contains(n))
            .ok_or("expected a column count from 1 to 4096")?;
        self.0 = &self.0[n..];
        Ok(count)
    }

    fn span(&mut self) -> Result<usize, String> {
        if !self.0.starts_with(":::") && self.take(":") {
            self.count()
        } else {
            Ok(1)
        }
    }

    fn delimited(&mut self, close: &str) -> Result<String, String> {
        let text = if self.take("\"") {
            let (text, rest) = self.0.split_once('"').ok_or("unterminated quote")?;
            self.0 = rest.trim_start();
            if !self.take(close) {
                return Err(format!("expected `{close}` after quoted label"));
            }
            text
        } else {
            let (text, rest) = self
                .0
                .split_once(close)
                .ok_or_else(|| format!("missing `{close}` after block label"))?;
            self.0 = rest;
            text.trim()
        };
        Ok(label(text))
    }

    fn shape(&mut self) -> Result<Option<(Shape, String)>, String> {
        let (open, close, mut shape) = [
            ("(((", ")))", Shape::DoubleCircle),
            ("((", "))", Shape::Circle),
            ("([", "])", Shape::Stadium),
            ("[[", "]]", Shape::Routine),
            ("[(", ")]", Shape::Cylinder),
            ("{{", "}}", Shape::Hexagon),
            ("<[", "]>", Shape::Arrows(0)),
            (">", "]", Shape::Flag),
            ("[", "]", Shape::Box),
            ("(", ")", Shape::Round),
            ("{", "}", Shape::Diamond),
        ]
        .into_iter()
        .find(|(open, _, _)| self.0.starts_with(open))
        .unwrap_or(("", "", Shape::Box));
        if open.is_empty() {
            return Ok(None);
        }
        self.take(open);
        let text = if open == "[" {
            let first = if self.take("/") {
                '/'
            } else if self.take("\\") {
                '\\'
            } else {
                '|'
            };
            let (text, tail) = if self.take("\"") {
                let (text, rest) = self.0.split_once('"').ok_or("missing closing quote")?;
                self.0 = rest.trim_start();
                let tail = if self.take("/") {
                    '/'
                } else if self.take("\\") {
                    '\\'
                } else {
                    '|'
                };
                if !self.take("]") {
                    return Err("expected `]` after block label".into());
                }
                (label(text), tail)
            } else {
                let (text, rest) = self.0.split_once(']').ok_or("missing `]`")?;
                self.0 = rest;
                let tail = text.chars().last().filter(|c| matches!(c, '/' | '\\'));
                (
                    label(
                        if tail.is_some() {
                            &text[..text.len() - 1]
                        } else {
                            text
                        }
                        .trim(),
                    ),
                    tail.unwrap_or('|'),
                )
            };
            if first != '|' || tail != '|' {
                shape = Shape::Slanted(first, tail);
            }
            text
        } else {
            self.delimited(close)?
        };
        if matches!(shape, Shape::Arrows(_)) {
            if !self.take("(") {
                return Err("expected block arrow directions in parentheses".into());
            }
            let (dirs, rest) = self
                .0
                .split_once(')')
                .ok_or("missing `)` after block arrow directions")?;
            self.0 = rest;
            let mut mask = 0;
            for dir in dirs.split(',') {
                mask |= match dir.trim() {
                    "left" => WEST,
                    "right" => EAST,
                    "up" => NORTH,
                    "down" => SOUTH,
                    "x" => EAST | WEST,
                    "y" => NORTH | SOUTH,
                    other => return Err(format!("unknown block arrow direction: {other}")),
                };
            }
            shape = Shape::Arrows(mask);
        }
        Ok(Some((shape, text)))
    }

    fn edge(&mut self) -> Result<Option<(Tip, Tip, Line, String)>, String> {
        self.spaces();
        let mut start = Tip::None;
        for (marker, tip) in [('<', Tip::Arrow), ('o', Tip::Circle), ('x', Tip::Cross)] {
            if let Some(rest) = self.0.strip_prefix(marker)
                && ["--", "==", "-."].iter().any(|s| rest.starts_with(s))
            {
                self.0 = rest;
                start = tip;
                break;
            }
        }
        let (line, tail, extension) = if self.take("--") {
            (Line::Solid, "--", "-")
        } else if self.take("==") {
            (Line::Thick, "==", "=")
        } else if self.take("-.") {
            (Line::Dotted, ".-", ".")
        } else {
            return Ok(None);
        };
        let mut text = String::new();
        let mut open = false;
        if self.0.trim_start().starts_with('"') {
            self.spaces();
            self.take("\"");
            let (s, rest) = self.0.split_once('"').ok_or("unterminated edge label")?;
            text = label(s);
            self.0 = rest.trim_start();
            if !self.take(tail) {
                return Err(format!("expected `{tail}` after edge label"));
            }
            open = matches!(line, Line::Dotted);
        } else if matches!(line, Line::Dotted) {
            while self.take(".") {}
            if !self.take("-") {
                return Err("expected `-` to finish dotted connection".into());
            }
            open = true;
        }
        while self.take(extension) {
            open = true;
        }
        let end = if self.take(">") {
            Tip::Arrow
        } else if self.take("x") {
            Tip::Cross
        } else if self.take("o") {
            Tip::Circle
        } else if open || start != Tip::None {
            Tip::None
        } else {
            return Err("incomplete block connection".into());
        };
        self.spaces();
        if self.take("|") {
            if !text.is_empty() {
                return Err("edge has two labels".into());
            }
            text = self.delimited("|")?;
            self.spaces();
        }
        Ok(Some((start, end, line, text)))
    }
}

struct Parser<'a> {
    diagram: Diagram,
    parents: Vec<usize>,
    // A later explicit placement supersedes an earlier implicit edge endpoint.
    positions: FxHashMap<usize, (usize, usize, bool)>,
    slots: usize,
    style_defs: FxHashMap<&'a str, Style>,
    class_assignments: Vec<(&'a str, &'a str)>,
    direct_styles: FxHashMap<&'a str, Style>,
}

impl<'a> Parser<'a> {
    fn grid(&mut self) -> &mut Grid {
        let Kind::Group(g) = &mut self.diagram.nodes[*self.parents.last().unwrap()].kind else {
            unreachable!()
        };
        g
    }

    fn place(&mut self, node: usize, span: usize, explicit: bool) -> Result<(), String> {
        if let Some(&(_, _, was_explicit)) = self.positions.get(&node)
            && (was_explicit || !explicit)
        {
            return Ok(());
        }
        let pos = (
            *self.parents.last().unwrap(),
            self.grid().slots.len(),
            explicit,
        );
        self.positions.insert(node, pos);
        self.grid().slots.push((Some(node), span));
        self.slots += 1;
        if self.slots > 65536 {
            return Err("block grid exceeds 65536 slots".into());
        }
        Ok(())
    }

    fn node(&mut self, s: &mut Cursor<'a>, id: &'a str) -> Result<(usize, usize, bool), String> {
        s.spaces();
        let shape = s.shape()?;
        s.spaces();
        let span = s.span()?;
        if s.take(":::") {
            loop {
                self.class_assignments.push((id, s.id()?));
                if !s.take(",") {
                    break;
                }
            }
        }
        let shaped = shape.is_some();
        let index = if let Some(&i) = self.diagram.names.get(id) {
            if let Some((shape, label)) = shape {
                self.diagram.nodes[i].label = label;
                match &mut self.diagram.nodes[i].kind {
                    Kind::Block(old) => *old = shape,
                    _ => return Err(format!("cannot change the shape of group `{id}`")),
                }
            }
            i
        } else {
            let (shape, label) = shape.unwrap_or_else(|| (Shape::Box, id.to_owned()));
            self.diagram.insert(
                id,
                Node {
                    label,
                    kind: Kind::Block(shape),
                    style: Style::default(),
                },
            )?
        };
        Ok((index, span, shaped))
    }

    fn statement(&mut self, line: &'a str) -> Result<(), String> {
        if let Some(statement) = paint_statement(line)? {
            match statement {
                PaintStatement::Define { names, style } => {
                    for name in names.split(',').map(str::trim) {
                        let current = self.style_defs.entry(name).or_default();
                        *current = current.overlay(style);
                    }
                }
                PaintStatement::Assign { targets, classes } => {
                    for target in targets.split(',').map(str::trim) {
                        self.class_assignments
                            .extend(classes.split(',').map(|class| (target, class.trim())));
                    }
                }
                PaintStatement::Direct { targets, style } => {
                    for target in targets.split(',').map(str::trim) {
                        let current = self.direct_styles.entry(target).or_default();
                        *current = current.overlay(style);
                    }
                }
            }
            return Ok(());
        }
        let mut s = Cursor(line);
        while !s.0.is_empty() {
            let id = s.id()?;
            match id {
                "columns" => {
                    s.spaces();
                    self.grid().columns = if s.take("auto") {
                        None
                    } else {
                        Some(s.count()?)
                    };
                }
                "space" => {
                    let n = s.span()?;
                    self.slots += n;
                    if self.slots > 65536 {
                        return Err("block grid exceeds 65536 slots".into());
                    }
                    self.grid().slots.extend(std::iter::repeat_n((None, 1), n));
                }
                "block" => {
                    if self.parents.len() >= 64 {
                        return Err("block nesting exceeds 64 levels".into());
                    }
                    let (mut name, mut span) = ("", 1);
                    if s.take(":") {
                        if s.0.starts_with(|c: char| c.is_ascii_digit()) {
                            span = s.count()?;
                        } else {
                            name = s.id()?;
                        }
                    }
                    let title = s
                        .shape()?
                        .map(|(_, t)| t)
                        .unwrap_or_else(|| name.to_owned());
                    if s.0.starts_with(':') {
                        span = s.span()?;
                    }
                    let group = Node {
                        label: title,
                        kind: Kind::Group(Grid::default()),
                        style: Style::default(),
                    };
                    let index = if let Some(&i) = self.diagram.names.get(name) {
                        if self.positions.get(&i).is_some_and(|p| p.2) {
                            return Err(format!("duplicate group `{name}`"));
                        }
                        self.diagram.nodes[i] = group;
                        i
                    } else {
                        self.diagram.insert(name, group)?
                    };
                    self.place(index, span, true)?;
                    self.parents.push(index);
                }
                "end" => {
                    if self.parents.len() == 1 {
                        return Err("unexpected `end` outside a block".into());
                    }
                    self.parents.pop();
                }
                _ => {
                    let (mut from, span, shaped) = self.node(&mut s, id)?;
                    let mut edge = s.edge()?;
                    self.place(from, span, shaped || edge.is_none() || span > 1)?;
                    while let Some((start, end, line, label)) = edge {
                        let name = s.id()?;
                        if matches!(name, "space" | "end" | "block" | "columns") {
                            return Err(format!("invalid edge endpoint: {name}"));
                        }
                        let (to, span, shaped) = self.node(&mut s, name)?;
                        self.place(to, span, shaped || span > 1)?;
                        self.diagram.edges.push(Edge {
                            from,
                            to,
                            label,
                            start,
                            end,
                            line,
                        });
                        from = to;
                        edge = s.edge()?;
                    }
                }
            }
            s.spaces();
        }
        Ok(())
    }

    fn apply_styles(&mut self) {
        if let Some(&style) = self.style_defs.get("default") {
            for node in &mut self.diagram.nodes {
                node.style = style;
            }
        }
        for &(target, class) in &self.class_assignments {
            if let Some(&index) = self.diagram.names.get(target)
                && let Some(&style) = self.style_defs.get(class)
            {
                let node = &mut self.diagram.nodes[index];
                node.style = node.style.overlay(style);
            }
        }
        for (&target, &style) in &self.direct_styles {
            if let Some(&index) = self.diagram.names.get(target) {
                let node = &mut self.diagram.nodes[index];
                node.style = node.style.overlay(style);
            }
        }
    }
}

pub(super) fn parse(body: &str) -> Result<(Diagram, Grid), String> {
    let mut parser = Parser {
        diagram: Diagram::default(),
        parents: vec![0],
        positions: FxHashMap::default(),
        slots: 0,
        style_defs: FxHashMap::default(),
        class_assignments: Vec::new(),
        direct_styles: FxHashMap::default(),
    };
    parser.diagram.insert(
        "",
        Node {
            label: String::new(),
            kind: Kind::Group(Grid::default()),
            style: Style::default(),
        },
    )?;
    for (n, line) in statements(body)? {
        parser
            .statement(line)
            .map_err(|e| format!("block line {n}: {e}"))?;
    }
    if parser.parents.len() != 1 {
        return Err("missing `end` for nested block".into());
    }
    for (parent, node) in parser.diagram.nodes.iter_mut().enumerate() {
        if let Kind::Group(g) = &mut node.kind {
            let mut pos = 0;
            g.slots.retain(|(id, _)| {
                let keep = id.is_none_or(|i| {
                    let (p, s, _) = parser.positions[&i];
                    (p, s) == (parent, pos)
                });
                pos += 1;
                keep
            });
        }
    }
    parser.apply_styles();
    let root = std::mem::take(parser.grid());
    Ok((parser.diagram, root))
}
