use std::collections::BTreeMap;

use crate::charts::shared::Direction;
use crate::color::Style;

use super::{Diagram, Edge, Kind, Note, Region, Scope, Side, State, StateId};

pub(super) fn parse(source: &str) -> Result<Diagram<'_>, String> {
    let mut input = Line::new(source.trim_start_matches('\u{feff}'), 1);
    while input.at_end() && !input.rest().starts_with("%%{") {
        input = input
            .next()
            .ok_or("expected a stateDiagram or stateDiagram-v2 header")?;
    }
    if input.rest().starts_with("%%{") {
        return Err("Mermaid directives are not supported".into());
    }
    if !input.word("stateDiagram") && !input.word("stateDiagram-v2") {
        return Err("expected a stateDiagram or stateDiagram-v2 header".into());
    }
    if !input.at_end() && !input.take(";") {
        return Err("expected a newline or semicolon after the state diagram header".into());
    }

    let mut parser = Parser {
        scopes: Vec::new(),
        regions: Vec::new(),
        path: Vec::new(),
        ids: BTreeMap::new(),
        states: Vec::new(),
        edges: Vec::new(),
        class_styles: BTreeMap::new(),
        state_styles: BTreeMap::new(),
        open_note: None,
        pending_composite: None,
    };
    parser.open_scope(None);
    loop {
        let line_number = input.number;
        parser
            .line(&mut input)
            .map_err(|error| format!("state diagram line {line_number}: {error}"))?;
        let Some(next) = input.next() else {
            break;
        };
        input = next;
    }
    if parser.open_note.is_some() {
        return Err("state diagram: expected end note".into());
    }
    if parser.path.len() != 1 {
        return Err("state diagram: expected } to close a composite state".into());
    }
    Ok(parser.finish())
}

struct Parser<'a> {
    scopes: Vec<Scope>,
    regions: Vec<Region<'a>>,
    path: Vec<usize>,
    ids: BTreeMap<StateId<'a>, usize>,
    states: Vec<State<'a>>,
    edges: Vec<Edge<'a>>,
    class_styles: BTreeMap<&'a str, Style>,
    state_styles: BTreeMap<&'a str, StateStyle<'a>>,
    open_note: Option<(usize, Note<'a>)>,
    pending_composite: Option<usize>,
}

#[derive(Default)]
struct StateStyle<'a> {
    classes: Vec<&'a str>,
    direct: Style,
}

impl<'a> Parser<'a> {
    fn scope_index(&self) -> usize {
        self.path[self.path.len() - 1]
    }

    fn region(&self) -> usize {
        let regions = &self.scopes[self.scope_index()].regions;
        regions[regions.len() - 1]
    }

    fn open_scope(&mut self, owner: Option<usize>) {
        self.path.push(self.scopes.len());
        self.scopes.push(Scope {
            owner,
            regions: Vec::new(),
        });
        self.open_region();
    }

    fn open_region(&mut self) {
        let scope = self.scope_index();
        self.scopes[scope].regions.push(self.regions.len());
        self.regions.push(Region {
            direction: Direction::Down,
            states: Vec::new(),
            edges: Vec::new(),
        });
    }

    fn state(&mut self, id: StateId<'a>) -> usize {
        let region = self.region();
        let index = *self.ids.entry(id).or_insert_with(|| {
            let index = self.states.len();
            self.states.push(State {
                id,
                region,
                kind: Kind::Ordinary,
                descriptions: Vec::new(),
                notes: Vec::new(),
                style: Style::default(),
            });
            index
        });
        let state = &mut self.states[index];
        // Root transitions can refer to children. A composite's declaration fixes its owner.
        if self.path.len() > 1 && !matches!(state.kind, Kind::Composite(_)) {
            state.region = region;
        }
        index
    }

    fn finish(mut self) -> Diagram<'a> {
        for (index, state) in self.states.iter_mut().enumerate() {
            if let StateId::Named(id) = state.id
                && let Some(style) = self.state_styles.get(id)
            {
                let classes = style
                    .classes
                    .iter()
                    .filter_map(|class| self.class_styles.get(class).copied());
                state.style = classes
                    .fold(Style::default(), Style::overlay)
                    .overlay(style.direct);
            }
            self.regions[state.region].states.push(index);
        }
        let mut cross_region = Vec::new();
        for edge in self.edges {
            let from = self.states[edge.from].region;
            let to = self.states[edge.to].region;
            if from == to {
                self.regions[from].edges.push(edge);
            } else {
                cross_region.push(edge);
            }
        }
        Diagram {
            scopes: self.scopes,
            regions: self.regions,
            states: self.states,
            cross_region,
        }
    }

    fn line(&mut self, input: &mut Line<'a>) -> Result<(), &'static str> {
        let text = input.rest();
        if text.chars().any(|ch| ch.is_control() && ch != '\t') {
            return Err("unexpected control character");
        }
        if let Some((state, mut note)) = self.open_note.take() {
            if input.take_word_pair("end", "note")
                && (input.at_end() || input.rest().starts_with([';', '}']))
            {
                self.states[state].notes.push(note);
            } else {
                note.lines.push(text.trim());
                self.open_note = Some((state, note));
                return Ok(());
            }
        }
        if !input.at_end()
            && let Some(state) = self.pending_composite.take()
            && input.take("{")
        {
            self.open_composite(state)?;
        }
        while !input.at_end() {
            if input.take(";") {
                continue;
            }
            if input.take("}") {
                if self.path.len() == 1 {
                    return Err("unexpected }");
                }
                self.path.pop();
            } else if input.word("state") {
                self.declaration(input)?;
            } else {
                self.statement(input)?;
                if !input.at_end() && !input.rest().starts_with([';', '}']) {
                    return Err("expected a newline, semicolon or }");
                }
            }
        }
        if input.rest().starts_with("%%{") {
            return Err("Mermaid directives are not supported");
        }
        Ok(())
    }

    fn declaration(&mut self, input: &mut Line<'a>) -> Result<(), &'static str> {
        let description = if input.take("\"") {
            let label = input.quoted_after_open()?;
            if !input.word("as") {
                return Err("expected as after the state label");
            }
            Some(label.trim())
        } else {
            None
        };
        let id = StateId::Named(input.named()?);
        let state = self.state(id);
        if let Some(label) = description.filter(|label| !label.is_empty()) {
            self.states[state].descriptions.push(label);
        }
        self.inline_classes(id, input)?;
        if input.take("{") {
            return self.open_composite(state);
        }
        if input.at_end() && matches!(self.states[state].kind, Kind::Ordinary) {
            self.pending_composite = Some(state);
            return Ok(());
        }
        for (primary, alternate, kind) in [
            ("<<choice>>", "[[choice]]", Kind::Choice),
            ("<<fork>>", "[[fork]]", Kind::Fork),
            ("<<join>>", "[[join]]", Kind::Join),
        ] {
            if input.take(primary) || input.take(alternate) {
                if !matches!(self.states[state].kind, Kind::Ordinary) {
                    return Err("state already has a composite or pseudostate declaration");
                }
                self.states[state].kind = kind;
                break;
            }
        }
        if let Some(description) = input.description()? {
            self.states[state].descriptions.push(description);
        }
        if !input.at_end() && !input.rest().starts_with([';', '}']) {
            return Err("expected a description, composite body or choice, fork, or join");
        }
        Ok(())
    }

    fn open_composite(&mut self, state: usize) -> Result<(), &'static str> {
        if !matches!(self.states[state].kind, Kind::Ordinary) {
            return Err("state already has a composite or pseudostate declaration");
        }
        self.states[state].kind = Kind::Composite(self.scopes.len());
        self.states[state].region = self.region();
        self.open_scope(Some(state));
        Ok(())
    }

    fn statement(&mut self, input: &mut Line<'a>) -> Result<(), &'static str> {
        if input.word("direction") {
            let region = self.region();
            self.regions[region].direction = match input.named()? {
                "TB" | "TD" => Direction::Down,
                "BT" => Direction::Up,
                "LR" => Direction::Right,
                "RL" => Direction::Left,
                _ => return Err("expected direction TB, BT, LR or RL"),
            };
        } else if input.word("note") {
            self.note(input)?;
        } else if input.word("classDef") {
            let names = input.id_list()?;
            let style = input.css()?;
            for name in names {
                let entry = self.class_styles.entry(name).or_default();
                *entry = entry.overlay(style);
            }
        } else if input.word("style") {
            let targets = input.id_list()?;
            let style = input.css()?;
            for target in targets {
                let entry = self.state_styles.entry(target).or_default();
                entry.direct = entry.direct.overlay(style);
            }
        } else if input.word("class") {
            let targets = input.id_list()?;
            let classes = input.id_list()?;
            for target in targets {
                self.state_styles
                    .entry(target)
                    .or_default()
                    .classes
                    .extend_from_slice(&classes);
            }
        } else if input.word("click") {
            self.endpoint(input, StateId::Start(self.region()))?;
            input.word("href");
            input.quoted()?;
            if input.rest().starts_with('"') {
                input.quoted()?;
            }
        } else if input.word("scale") {
            if !input.named()?.bytes().all(|byte| byte.is_ascii_digit()) || !input.word("width") {
                return Err("expected scale <number> width");
            }
        } else if input.take_word_pair("hide", "empty") {
            if !input.word("description") {
                return Err("expected hide empty description");
            }
        } else if input.take("--") {
            self.open_region();
        } else {
            let region = self.region();
            let from = self.endpoint(input, StateId::Start(region))?;
            if input.take("-->") {
                let to = self.endpoint(input, StateId::End(region))?;
                let label = input.description()?;
                let from = self.state(from);
                let to = self.state(to);
                self.edges.push(Edge { from, to, label });
            } else {
                if !matches!(from, StateId::Named(_)) {
                    return Err("[*] must be an endpoint of a transition");
                }
                let state = self.state(from);
                if let Some(description) = input.description()? {
                    self.states[state].descriptions.push(description);
                }
            }
        }
        Ok(())
    }

    fn endpoint(
        &mut self,
        input: &mut Line<'a>,
        wildcard: StateId<'a>,
    ) -> Result<StateId<'a>, &'static str> {
        let id = if input.take("[*]") {
            wildcard
        } else {
            StateId::Named(input.named()?)
        };
        self.inline_classes(id, input)?;
        Ok(id)
    }

    fn inline_classes(
        &mut self,
        id: StateId<'a>,
        input: &mut Line<'a>,
    ) -> Result<(), &'static str> {
        while input.take(":::") {
            let class = input.named()?;
            if let StateId::Named(id) = id {
                self.state_styles.entry(id).or_default().classes.push(class);
            }
        }
        Ok(())
    }

    fn note(&mut self, input: &mut Line<'a>) -> Result<(), &'static str> {
        if input.rest().starts_with('"') {
            let text = input.quoted()?.trim();
            if !input.word("as") {
                return Err("expected as after the floating note");
            }
            let id = input.named()?;
            let index = self.state(StateId::Named(id));
            let state = &mut self.states[index];
            if !matches!(state.kind, Kind::Ordinary | Kind::FloatingNote) {
                return Err("a floating note cannot also declare a composite or pseudostate");
            }
            state.kind = Kind::FloatingNote;
            if !text.is_empty() {
                state.descriptions.push(text);
            }
            return Ok(());
        }
        let side = if input.take_word_pair("left", "of") {
            Side::Left
        } else if input.take_word_pair("right", "of") {
            Side::Right
        } else {
            return Err("expected a floating note or a note left of or right of a state");
        };
        let state = self.state(StateId::Named(input.named()?));
        if input.take(":") {
            self.states[state].notes.push(Note {
                side,
                lines: vec![input.text()?],
            });
        } else if input.at_end() {
            self.open_note = Some((
                state,
                Note {
                    side,
                    lines: Vec::new(),
                },
            ));
        } else {
            return Err("expected note text after : or on the following lines");
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Line<'a> {
    source: &'a str,
    end: usize,
    number: usize,
}

impl<'a> Line<'a> {
    fn new(source: &'a str, number: usize) -> Self {
        Self {
            source,
            end: source.find('\n').unwrap_or(source.len()),
            number,
        }
    }

    fn next(self) -> Option<Self> {
        self.source
            .get(self.end + 1..)
            .map(|source| Self::new(source, self.number + 1))
    }

    fn consume(&mut self, rest: &'a str) {
        self.source = &self.source[self.end - rest.len()..];
        self.end = rest.len();
    }

    fn rest(&self) -> &'a str {
        self.source[..self.end].trim_start()
    }

    fn at_end(&self) -> bool {
        let rest = self.rest();
        rest.is_empty() || rest.starts_with('#') || rest.starts_with("%%")
    }

    fn take(&mut self, token: &str) -> bool {
        let Some(rest) = self.rest().strip_prefix(token) else {
            return false;
        };
        self.consume(rest);
        true
    }

    fn word(&mut self, word: &str) -> bool {
        let Some(rest) = self.rest().strip_prefix(word) else {
            return false;
        };
        if !rest.is_empty()
            && !rest.starts_with(char::is_whitespace)
            && !rest.starts_with([';', '{', '}', '#'])
            && !rest.starts_with("%%")
        {
            return false;
        }
        self.consume(rest);
        true
    }

    fn take_word_pair(&mut self, first: &str, second: &str) -> bool {
        let mut candidate = *self;
        if !candidate.word(first) || !candidate.word(second) {
            return false;
        }
        *self = candidate;
        true
    }

    fn named(&mut self) -> Result<&'a str, &'static str> {
        self.id(false)
    }

    fn id(&mut self, stop_at_comma: bool) -> Result<&'a str, &'static str> {
        if self.at_end() {
            return Err("expected a state or class ID");
        }
        let text = self.rest();
        let end = text
            .char_indices()
            .find(|&(at, ch)| {
                ch.is_whitespace()
                    || matches!(ch, ':' | ';' | '{' | '}' | '"' | '[' | ']' | '<' | '>')
                    || stop_at_comma && ch == ','
                    || ch == '-' && text[at..].starts_with("-->")
                    || ch == '%' && text[at..].starts_with("%%")
            })
            .map_or(text.len(), |(at, _)| at);
        if end == 0 {
            return Err("expected a state ID");
        }
        let (id, rest) = text.split_at(end);
        self.consume(rest);
        Ok(id)
    }

    fn id_list(&mut self) -> Result<Vec<&'a str>, &'static str> {
        let mut ids = Vec::new();
        loop {
            ids.push(self.id(true)?);
            if !self.take(",") {
                return Ok(ids);
            }
        }
    }

    fn quoted(&mut self) -> Result<&'a str, &'static str> {
        if !self.take("\"") {
            return Err("expected quoted text");
        }
        self.quoted_after_open()
    }

    fn quoted_after_open(&mut self) -> Result<&'a str, &'static str> {
        if let Some((text, rest)) = self.source[..self.end].split_once('"') {
            self.consume(rest);
            return Ok(text);
        }
        let close = self.source[self.end..]
            .find('"')
            .map(|index| self.end + index)
            .ok_or("unterminated quoted text")?;
        let (text, rest) = self.source.split_at(close);
        let lines = text.bytes().filter(|&b| b == b'\n').count();
        *self = Self::new(&rest[1..], self.number + lines);
        Ok(text)
    }

    fn css(&mut self) -> Result<Style, &'static str> {
        let text = self.rest();
        let mut quote = None;
        let mut escaped = false;
        let mut parentheses = 0usize;
        let mut end = text.len();
        for (at, ch) in text.char_indices() {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if let Some(delimiter) = quote {
                if ch == delimiter {
                    quote = None;
                }
            } else {
                match ch {
                    '"' | '\'' => quote = Some(ch),
                    '(' => parentheses += 1,
                    ')' => parentheses = parentheses.saturating_sub(1),
                    ';' | '}' if parentheses == 0 => {
                        end = at;
                        break;
                    }
                    '%' if parentheses == 0 && text[at..].starts_with("%%") => {
                        end = at;
                        break;
                    }
                    _ => {}
                }
            }
        }
        let (style, rest) = text.split_at(end);
        if style.trim_end().is_empty() {
            return Err("expected style properties");
        }
        self.consume(rest);
        Ok(Style::css(style))
    }

    fn description(&mut self) -> Result<Option<&'a str>, &'static str> {
        self.take(":").then(|| self.text()).transpose()
    }

    fn text(&mut self) -> Result<&'a str, &'static str> {
        let text = self.rest();
        let mut end = text.len();
        let mut entity_end = None;
        for (at, byte) in text.bytes().enumerate() {
            match byte {
                b'#' | b'&' => {
                    let tail = &text.as_bytes()[at + 1..];
                    let len = tail
                        .iter()
                        .take_while(|byte| byte.is_ascii_alphanumeric())
                        .count();
                    entity_end = (len > 0 && tail.get(len) == Some(&b';')).then_some(at + 1 + len);
                }
                b';' if entity_end != Some(at) => {
                    end = at;
                    break;
                }
                b'%' if text[at..].starts_with("%%") => {
                    end = at;
                    break;
                }
                _ => {}
            }
        }
        let (text, rest) = text.split_at(end);
        self.consume(rest);
        let text = text.trim_end();
        if text.is_empty() {
            return Err("expected text after :");
        }
        Ok(text)
    }
}
