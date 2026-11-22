use std::collections::{BTreeMap, BTreeSet};

use super::shared::model_header;
use crate::Options;

mod image;
mod layout;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::draw);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        layout::draw(&parse(source)?, options)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Actor,
    Ellipse,
    Rectangle,
    Json,
    Boundary,
}

struct Node {
    id: String,
    label: String,
    kind: Kind,
    group: Option<usize>,
    stereotype: String,
    attr: BTreeMap<String, String>,
    rows: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tip {
    None,
    Arrow,
    Circle,
    Cross,
    Triangle,
}

struct Edge {
    id: Option<String>,
    from: String,
    to: String,
    ends: (Tip, Tip),
    label: String,
    dotted: bool,
}

#[derive(Default)]
struct Diagram {
    styles: crate::node_styles::NodeStyles,
    nodes: Vec<Node>,
    ids: BTreeMap<String, usize>,
    edges: Vec<Edge>,
    edge_ids: BTreeSet<String>,
    notes: Vec<(String, String)>,
    direction: String,
    headings: Vec<String>,
}

fn ident(s: &str) -> (&str, &str) {
    s.split_at(
        s.bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count(),
    )
}

fn entity_text(s: &str) -> String {
    let mut out = String::new();
    let mut tail = s;
    while let Some((left, rest)) = tail.split_once('#') {
        out.push_str(left);
        if let Some(i) = rest.bytes().take(9).position(|c| c == b';') {
            let (code, right) = (&rest[..i], &rest[i + 1..]);
            let c = match code {
                "quot" => Some('"'),
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                _ => code.parse().ok().and_then(char::from_u32),
            };
            if let Some(c) = c.filter(|c| !c.is_control()) {
                out.push(c);
                tail = right;
                continue;
            }
        }
        out.push('#');
        tail = rest;
    }
    out.push_str(tail);
    out
}

fn quoted(s: &str) -> Result<(String, &str), String> {
    let (text, tail) = if let Some(s) = s.strip_prefix("\"`") {
        let (value, tail) = s.split_once("`\"").ok_or("unclosed Markdown string")?;
        (value.replace("**", "").replace("__", ""), tail)
    } else {
        let (value, tail) = s
            .strip_prefix('"')
            .and_then(|s| s.split_once('"'))
            .ok_or("expected quoted label")?;
        if value.contains('\n') {
            return Err("newline in plain string".into());
        }
        (value.into(), tail)
    };
    Ok((entity_text(&text), tail.trim_start()))
}

fn node_spec(s: &str, base: Kind, group: Option<usize>) -> Result<(Node, &str), String> {
    let (id, mut tail) = ident(s);
    let mut kind = base;
    let (id, label) = if id.is_empty() {
        let (text, t) = quoted(s)?;
        tail = t;
        (
            text.chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect(),
            text,
        )
    } else {
        tail = tail.trim_start();
        let text = if tail.starts_with(['[', '(']) {
            let close = if tail.starts_with('[') { ']' } else { ')' };
            if close == ']' && base == Kind::Ellipse {
                kind = Kind::Rectangle;
            }
            tail = tail[1..].trim_start();
            if tail.starts_with('"') {
                let (text, t) = quoted(tail)?;
                tail = t.strip_prefix(close).ok_or("unclosed label delimiter")?;
                text
            } else {
                let (text, t) = tail.split_once(close).ok_or("unclosed label delimiter")?;
                if text.contains(['(', ')', '[', ']', '{', '}', '\'', '"']) {
                    return Err("quote labels containing delimiters".into());
                }
                tail = t;
                entity_text(text.trim())
            }
        } else {
            id.into()
        };
        (id.into(), text)
    };
    if label.trim().is_empty() {
        return Err("expected visible label".into());
    }
    Ok((
        Node {
            id,
            label,
            kind,
            group,
            stereotype: String::new(),
            attr: BTreeMap::new(),
            rows: Vec::new(),
        },
        tail.trim_start(),
    ))
}

fn metadata(s: &str) -> Result<(BTreeMap<String, String>, &str), String> {
    let (body, end) = s
        .strip_prefix("@{")
        .and_then(|s| s.split_once('}'))
        .ok_or("unclosed metadata")?;
    let mut data = BTreeMap::new();
    for part in body.split(',') {
        let (k, v) = part.split_once(':').ok_or("expected metadata key: value")?;
        let v = v.trim();
        let value = if v.starts_with('"') {
            let (v, rest) = quoted(v)?;
            if !rest.is_empty() {
                return Err("unexpected metadata value suffix".into());
            }
            v
        } else {
            v.into()
        };
        data.insert(k.trim().into(), value);
    }
    Ok((data, end.trim_start()))
}

fn classes(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Ok(());
    }
    if let Some(names) = s.strip_prefix(":::")
        && !names.is_empty()
        && names
            .split(',')
            .all(|n| !n.is_empty() && ident(n).1.is_empty())
    {
        return Ok(());
    }
    Err(format!("unexpected declaration suffix: {s}"))
}

fn json_rows(s: &str, path: &str, rows: &mut Vec<(String, String)>) -> Result<usize, String> {
    // Iterate object members in source order; serde_json supplies the scalar decoder.
    let array = s.starts_with('[');
    let close = if array { ']' } else { '}' };
    let mut index = 0;
    let mut fields = Vec::<Vec<(String, String)>>::new();
    let mut keys = BTreeMap::new();
    let mut tail = s[1..].trim_start();
    while !tail.starts_with(close) {
        let name = if array {
            format!("{path}[{index}]")
        } else {
            let mut iter = serde_json::Deserializer::from_str(tail).into_iter::<String>();
            let k = iter
                .next()
                .ok_or("missing JSON key")?
                .map_err(|e| e.to_string())?;
            tail = tail[iter.byte_offset()..]
                .trim_start()
                .strip_prefix(':')
                .ok_or("expected :")?
                .trim_start();
            if path.is_empty() {
                k
            } else {
                format!("{path}.{k}")
            }
        };
        let pos = if array {
            fields.len()
        } else {
            *keys.entry(name.clone()).or_insert(fields.len())
        };
        if pos == fields.len() {
            fields.push(Vec::new());
        }
        let field = &mut fields[pos];
        field.clear();
        let n = if tail.starts_with(['[', '{']) {
            json_rows(tail, &name, field)?
        } else {
            let mut it = serde_json::Deserializer::from_str(tail).into_iter::<serde_json::Value>();
            let val = it
                .next()
                .ok_or("missing JSON value")?
                .map_err(|e| e.to_string())?;
            let text = match val {
                serde_json::Value::String(s) => s
                    .replace('\t', " ")
                    .chars()
                    .map(|c| {
                        if c.is_control() && c != '\n' {
                            '�'
                        } else {
                            c
                        }
                    })
                    .collect(),
                other => other.to_string(),
            };
            field.push((name, text));
            it.byte_offset()
        };
        tail = tail[n..].trim_start();
        index += 1;
        if let Some(t) = tail.strip_prefix(',') {
            tail = t.trim_start();
        } else {
            break;
        }
    }
    if !tail.starts_with(close) {
        return Err("unclosed JSON container".into());
    }
    if index == 0 && !path.is_empty() {
        rows.push((path.into(), if array { "[]" } else { "{}" }.into()));
    }
    rows.extend(fields.into_iter().flatten());
    Ok(1 + s.len() - tail.len())
}

fn operator(s: &str) -> Option<((Tip, Tip), &str)> {
    use Tip::*;
    for (token, ends) in [
        ("--|>", (None, Triangle)),
        ("--o", (None, Circle)),
        ("o--", (Circle, None)),
        ("--x", (None, Cross)),
        ("x--", (Cross, None)),
    ] {
        if let Some(t) = s.strip_prefix(token) {
            return Some((ends, t));
        }
    }
    let (rev, t) = s.strip_prefix('<').map_or((false, s), |t| (true, t));
    let n = t.bytes().take_while(|b| *b == b'-').count();
    if n < 2 {
        return Option::None;
    }
    let t = &t[n..];
    Some(if rev {
        ((Arrow, None), t)
    } else if let Some(t) = t.strip_prefix('>') {
        ((None, Arrow), t)
    } else {
        ((None, None), t)
    })
}

fn edge(left: &str, mut s: &str) -> Result<Edge, String> {
    let (candidate, body) = ident(s);
    let id = if let Some(t) = body.strip_prefix('@') {
        if candidate.is_empty() {
            return Err("empty edge id".into());
        }
        s = t;
        Some(candidate.into())
    } else {
        None
    };
    let (ends, label, to, dotted) = if let Some(t) = s.strip_prefix("..>") {
        let (kind, to) = t
            .trim_start()
            .strip_prefix(':')
            .ok_or("expected : include or : extend")?
            .trim()
            .split_once(char::is_whitespace)
            .ok_or("expected relationship target")?;
        if !matches!(kind, "include" | "extend") {
            return Err(format!("unknown relationship: {kind}"));
        }
        ((Tip::None, Tip::Arrow), kind.to_string(), to.trim(), true)
    } else {
        let (mut ends, t) = operator(s).ok_or("unknown relationship operator")?;
        let mut t = t.trim();
        let mut label = String::new();
        if !ident(t).1.is_empty() || t.is_empty() {
            if t.starts_with('"') {
                (label, t) = quoted(t)?;
            } else {
                let i = t
                    .char_indices()
                    .find(|(i, _)| operator(&t[*i..]).is_some())
                    .ok_or("expected closing association operator and target")?
                    .0;
                label = entity_text(t[..i].trim());
                t = &t[i..];
            }
            let (last, right) = operator(t).ok_or("expected association operator after label")?;
            if ends.1 != Tip::None || last.0 != Tip::None {
                return Err("invalid labelled association".into());
            }
            ends.1 = last.1;
            t = right.trim();
        }
        (ends, label, t, false)
    };
    if to.is_empty() || !ident(to).1.is_empty() {
        return Err("expected a single target identifier".into());
    }
    Ok(Edge {
        id,
        from: left.into(),
        to: to.into(),
        ends,
        label,
        dotted,
    })
}

impl Diagram {
    fn add(&mut self, n: Node) -> Result<usize, String> {
        let i = self.nodes.len();
        if self.edge_ids.contains(&n.id) || self.ids.insert(n.id.clone(), i).is_some() {
            return Err(format!("duplicate identifier: {}", n.id));
        }
        self.nodes.push(n);
        Ok(i)
    }

    fn statement(
        &mut self,
        line: &str,
        group: &mut Option<usize>,
        later: &mut Vec<(String, BTreeMap<String, String>)>,
    ) -> Result<(), String> {
        let (kw, rest) = line
            .split_once(char::is_whitespace)
            .map_or((line, ""), |(k, r)| (k, r.trim()));
        if line == "end" {
            return group.take().map(|_| ()).ok_or("unmatched end".into());
        }
        if group.is_some()
            && matches!(
                kw,
                "direction"
                    | "systemBoundary"
                    | "note"
                    | "json"
                    | "class"
                    | "classDef"
                    | "style"
                    | "accDescr"
                    | "accDescr:"
                    | "accTitle:"
            )
        {
            return Err("boundary contents must be actor or use case declarations".into());
        }
        match kw {
            "direction" => {
                if !matches!(rest, "LR" | "RL" | "TD" | "TB" | "BT") {
                    return Err("expected LR, RL, TD, TB or BT".into());
                }
                self.direction = rest.into();
            }
            "accTitle:" | "accDescr:" => self.headings.push(rest.into()),
            "accDescr" => {
                self.headings.push(
                    rest.strip_prefix('{')
                        .and_then(|s| s.strip_suffix('}'))
                        .ok_or("expected accDescr { ... }")?
                        .trim()
                        .to_string(),
                );
            }
            "classDef" | "class" | "style" => {
                let (ids, style) = rest
                    .split_once(char::is_whitespace)
                    .ok_or("incomplete style statement")?;
                if ids.is_empty()
                    || style.is_empty()
                    || style.contains(';')
                    || kw != "class" && !style.contains(':')
                {
                    return Err("invalid style statement".into());
                }
                match kw {
                    "classDef" => self.styles.define(ids, crate::color::Style::css(style)),
                    "class" => self.styles.assign(ids, style),
                    _ => self.styles.set(ids, crate::color::Style::css(style)),
                }
            }
            "note" => {
                let t = rest
                    .strip_prefix("for")
                    .filter(|s| s.starts_with(char::is_whitespace))
                    .ok_or("expected note for ID \"text\"")?;
                let (id, t) = ident(t.trim_start());
                if id.is_empty() {
                    return Err("missing note target".into());
                }
                let (text, t) = quoted(t.trim_start())?;
                if !t.is_empty() {
                    return Err("unexpected text after note".into());
                }
                self.notes.push((id.into(), text));
            }
            "json" => {
                let (mut n, t) = node_spec(rest, Kind::Json, None)?;
                let t = t
                    .strip_prefix('@')
                    .filter(|t| t.starts_with('{'))
                    .ok_or("expected JSON object after @")?;
                let mut it = serde_json::Deserializer::from_str(t).into_iter::<serde_json::Value>();
                it.next()
                    .ok_or("empty JSON object declaration")?
                    .map_err(|e| e.to_string())?;
                let consumed = it.byte_offset();
                classes(t[consumed..].trim())?;
                if let Some(names) = t[consumed..].trim().strip_prefix(":::") {
                    self.styles.assign(&n.id, names);
                }
                let mut entries = Vec::new();
                json_rows(&t[..consumed], "", &mut entries)?;
                n.rows = entries
                    .into_iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect();
                self.add(n)?;
            }
            _ => {
                let base = match kw {
                    "actor" => Kind::Actor,
                    "systemBoundary" => Kind::Boundary,
                    _ => Kind::Ellipse,
                };
                let raw = if base == Kind::Ellipse { line } else { rest };
                let (id, tail) = ident(raw);
                let t = tail.trim_start();
                if base == Kind::Ellipse
                    && !id.is_empty()
                    && !t.is_empty()
                    && !t.starts_with(['(', '['])
                    && !t.starts_with(":::")
                    && !t.starts_with("<<")
                    && !t.starts_with("@{")
                {
                    if group.is_some() {
                        return Err("relationships must be outside the boundary".into());
                    }
                    let rel = edge(id, t)?;
                    if let Some(id) = &rel.id
                        && (self.ids.contains_key(id) || !self.edge_ids.insert(id.clone()))
                    {
                        return Err(format!("duplicate identifier: {id}"));
                    }
                    self.edges.push(rel);
                    return Ok(());
                }
                if base == Kind::Ellipse && !id.is_empty() && t.starts_with("@{") && group.is_none()
                {
                    let (attrs, rest) = metadata(t)?;
                    if rest.is_empty() {
                        later.push((id.into(), attrs));
                        return Ok(());
                    }
                }
                let (mut n, mut t) = node_spec(raw, base, *group)?;
                if t.starts_with("@{") {
                    (n.attr, t) = metadata(t)?;
                }
                if let Some(tail) = t.strip_prefix("<<") {
                    let (s, rest) = tail.split_once(">>").ok_or("unclosed stereotype")?;
                    n.stereotype = s.trim().into();
                    t = rest.trim_start();
                }
                classes(t)?;
                if let Some(names) = t.strip_prefix(":::") {
                    self.styles.assign(&n.id, names);
                }
                let idx = self.add(n)?;
                if base == Kind::Boundary {
                    *group = Some(idx);
                }
            }
        }
        Ok(())
    }
}

fn parse(source: &str) -> Result<Diagram, String> {
    let lines = model_header::body(source, "usecase-beta")?;
    let mut d = Diagram::default();
    let mut group = None;
    let mut later = Vec::new();
    let mut lines = lines.enumerate();
    while let Some((i, s)) = lines.next() {
        if s.trim().is_empty() || s.trim_start().starts_with("%%") {
            continue;
        }
        let mut line = String::new();
        let mut part = s.trim_start();
        let json = part.starts_with("json") && part[4..].starts_with(char::is_whitespace);
        let (mut depth, mut string, mut markdown) = (0usize, false, false);
        loop {
            let mut chars = part.chars().peekable();
            while let Some(ch) = chars.next() {
                if markdown {
                    if ch == '`' && chars.next_if_eq(&'"').is_some() {
                        markdown = false;
                    }
                } else if string {
                    if ch == '\\' && json {
                        chars.next();
                    } else if ch == '"' {
                        string = false;
                    }
                } else {
                    match ch {
                        '"' if !json && chars.next_if_eq(&'`').is_some() => markdown = true,
                        '"' => string = true,
                        '{' => depth += 1,
                        '}' => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
            }
            line.push_str(part);
            if depth == 0 && !markdown && !string {
                break;
            }
            line.push('\n');
            part = lines
                .next()
                .ok_or_else(|| format!("use case line {}: unclosed string or block", i + 2))?
                .1;
        }
        d.statement(line.trim_end(), &mut group, &mut later)
            .map_err(|e| format!("use case line {}: {e}", i + 2))?;
    }
    if group.is_some() {
        return Err("unclosed systemBoundary".into());
    }
    let endpoints: Vec<_> = d
        .edges
        .iter()
        .flat_map(|e| [e.from.clone(), e.to.clone()])
        .collect();
    for id in endpoints {
        if !d.ids.contains_key(&id) {
            d.add(node_spec(&id, Kind::Ellipse, None)?.0)?;
        }
    }
    for (id, attrs) in later {
        if let Some(i) = d.ids.get(&id) {
            d.nodes[*i].attr.extend(attrs);
        } else if d.edge_ids.contains(&id) {
            for (k, v) in attrs {
                if !matches!(
                    (k.as_str(), v.as_str()),
                    ("animate", "true" | "false") | ("animation", "fast" | "slow")
                ) {
                    return Err(format!("invalid edge metadata on {id}: {k}: {v}"));
                }
            }
        } else if attrs.contains_key("business") {
            let mut n = node_spec(&id, Kind::Ellipse, None)?.0;
            n.attr = attrs;
            d.add(n)?;
        } else {
            return Err(format!("metadata references unknown element: {id}"));
        }
    }
    for (id, text) in &d.notes {
        let i = *d
            .ids
            .get(id)
            .ok_or_else(|| format!("unknown note target: {id}"))?;
        if matches!(d.nodes[i].kind, Kind::Json | Kind::Boundary) {
            return Err(format!("invalid note target: {id}"));
        }
        d.nodes[i].rows.push(format!("Note: {text}"));
    }
    for e in &d.edges {
        let a = d.nodes[d.ids[&e.from]].kind;
        let b = d.nodes[d.ids[&e.to]].kind;
        let case = |k| matches!(k, Kind::Ellipse | Kind::Rectangle);
        if a == Kind::Boundary
            || b == Kind::Boundary
            || e.dotted && !(case(a) && case(b))
            || e.ends.1 == Tip::Triangle
                && !(a == Kind::Actor && b == Kind::Actor || case(a) && case(b))
            || (a == Kind::Json || b == Kind::Json)
                && [e.ends.0, e.ends.1]
                    .iter()
                    .any(|t| !matches!(t, Tip::None | Tip::Arrow))
        {
            return Err(format!(
                "invalid relationship endpoints: {} and {}",
                e.from, e.to
            ));
        }
    }
    for n in &d.nodes {
        if n.kind == Kind::Actor {
            let variant = n.attr.get("type").map(String::as_str).unwrap_or("normal");
            let business = n.attr.get("business").is_some_and(|v| v == "true");
            if n.attr.contains_key("icon") && (variant != "normal" || business)
                || business && variant == "awesome"
            {
                return Err(format!("incompatible actor metadata on {}", n.id));
            }
        }
        for (k, v) in &n.attr {
            let valid = match (n.kind, k.as_str()) {
                (Kind::Actor, "type") => matches!(v.as_str(), "normal" | "hollow" | "awesome"),
                (Kind::Boundary, "type") => matches!(v.as_str(), "rect" | "package"),
                (Kind::Actor, "icon") => !v.is_empty(),
                (Kind::Actor | Kind::Ellipse, "business") => matches!(v.as_str(), "true" | "false"),
                _ => false,
            };
            if !valid {
                return Err(format!("invalid metadata on {}: {k}: {v}", n.id));
            }
        }
    }
    if d.nodes.is_empty() {
        return Err("use case diagram has no nodes".into());
    }
    Ok(d)
}
