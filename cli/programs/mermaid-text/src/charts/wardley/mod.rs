use std::collections::BTreeMap;

use super::shared::model_header;
use crate::Options;

mod image;
mod plot;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::draw);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        plot::draw(&parse(source)?, options)
    }
}

type Point = (f64, f64); // visibility, evolution

struct Item {
    name: String,
    point: Point,
    kind: String,
    parent: Option<String>,
    offset: Point,
}

struct Link {
    from: String,
    to: String,
    style: &'static str,
    caption: String,
}

struct Map {
    title: String,
    size: Point,
    stages: Vec<(String, f64)>,
    annotations: Option<Point>,
    items: Vec<Item>,
    names: BTreeMap<String, usize>,
    links: Vec<Link>,
    moves: Vec<(String, Option<f64>, f64)>,
}

fn number(s: &str) -> Result<f64, String> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("expected a finite number, got {s:?}"))
}

fn unit(n: f64) -> Result<f64, String> {
    if (0.0..=1.0).contains(&n) {
        Ok(n)
    } else {
        Err(format!("coordinate {n} outside [0, 1]"))
    }
}

fn coords(s: &str, arity: usize) -> Result<(Vec<f64>, &str), String> {
    let (list, rest) = s
        .trim_start()
        .strip_prefix('[')
        .and_then(|s| s.split_once(']'))
        .ok_or("expected [coordinates]")?;
    let vals: Vec<_> = list.split(',').map(number).collect::<Result<_, _>>()?;
    if vals.len() != arity {
        return Err(format!("expected {arity} coordinates"));
    }
    Ok((vals, rest.trim()))
}

fn pair(s: &str) -> Result<(Point, &str), String> {
    let (p, tail) = coords(s, 2)?;
    Ok(((p[0], p[1]), tail))
}

fn name(s: &str) -> Result<String, String> {
    let s = s.trim();
    let value = if let Some(s) = s.strip_prefix('"') {
        s.strip_suffix('"')
            .filter(|s| !s.contains('"'))
            .ok_or("invalid quoted name")?
    } else {
        if s.contains(['"', '[', ']', '{', '}', ';']) {
            return Err(format!("invalid name {s:?}"));
        }
        s
    };
    if value.is_empty() {
        return Err("expected a name".into());
    }
    Ok(value.into())
}

fn before_unquoted(s: &str, stop: impl Fn(&str) -> bool) -> (&str, &str) {
    let mut quoted = false;
    for (i, c) in s.char_indices() {
        if !quoted && stop(&s[i..]) {
            return s.split_at(i);
        }
        if c == '"' {
            quoted = !quoted;
        }
    }
    (s, "")
}

fn end(s: &str) -> Result<(), String> {
    if s.is_empty() {
        Ok(())
    } else {
        Err(format!("unexpected trailing text: {s}"))
    }
}

fn stages(s: &str) -> Result<Vec<(String, f64)>, String> {
    let parts: Vec<_> = s.split("->").collect();
    let mut last = 0.;
    let mut stages = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let (title, upper) = match part.rsplit_once('@') {
            Some((title, upper)) => (title, unit(number(upper)?)?),
            None => (*part, (i + 1) as f64 / parts.len() as f64),
        };
        if title.trim().is_empty() || upper <= last {
            return Err("invalid evolution stage or boundary".into());
        }
        stages.push((title.trim().into(), upper));
        last = upper;
    }
    if last != 1. {
        return Err("last evolution stage must end at 1".into());
    }
    Ok(stages)
}

impl Map {
    fn lookup(&self, s: &str) -> Result<usize, String> {
        self.names
            .get(s)
            .copied()
            .ok_or_else(|| format!("unknown Wardley component: {s}"))
    }

    fn statement(&mut self, line: &str, pipeline: &mut Option<String>) -> Result<(), String> {
        if line == "}" {
            return pipeline.take().map(|_| ()).ok_or("unexpected }".into());
        }
        let (kw, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        if pipeline.is_some() && kw != "component" {
            return Err("expected component or } inside pipeline".into());
        }
        match kw {
            "title" => self.title = name(rest)?,
            "size" => {
                let (size, tail) = pair(rest)?;
                end(tail)?;
                if size.0 <= 0. || size.1 <= 0. {
                    return Err("size must be positive".into());
                }
                self.size = size;
            }
            "evolution" => self.stages = stages(rest)?,
            "pipeline" => {
                *pipeline = Some(name(
                    rest.strip_suffix('{').ok_or("expected pipeline name {")?,
                )?);
            }
            "annotations" => {
                let ((a, b), tail) = pair(rest)?;
                self.annotations = Some((unit(a)?, unit(b)?));
                end(tail)?;
            }
            "annotation" => {
                let (n, rest) = rest
                    .split_once(',')
                    .ok_or("expected annotation number,[position] \"text\"")?;
                let n: usize = n.trim().parse().map_err(|_| "invalid annotation number")?;
                let ((v, e), text) = pair(rest)?;
                if !text.starts_with('"') {
                    return Err("annotation text must be quoted".into());
                }
                self.items.push(Item {
                    name: name(text)?,
                    point: (unit(v)?, unit(e)?),
                    kind: format!("annotation {n}"),
                    parent: None,
                    offset: (0., 0.),
                });
            }
            "component" | "anchor" | "note" | "accelerator" | "deaccelerator" => {
                let (raw, args) = before_unquoted(rest, |s| s.starts_with('['));
                let label = name(raw)?;
                if kw == "note" && !raw.starts_with('"') {
                    return Err("note text must be quoted".into());
                }
                let (vals, mut tail) = coords(args, if pipeline.is_some() { 1 } else { 2 })?;
                let point = if pipeline.is_some() {
                    (0., unit(vals[0])?)
                } else {
                    (unit(vals[0])?, unit(vals[1])?)
                };
                let mut offset = (0., 0.);
                let mut kind = kw.to_string();
                while !tail.is_empty() {
                    if let Some(t) = tail.strip_prefix("label") {
                        (offset, tail) = pair(t)?;
                    } else if let Some((dec, t)) =
                        tail.strip_prefix('(').and_then(|s| s.split_once(')'))
                    {
                        if !matches!(dec, "build" | "buy" | "outsource" | "market" | "inertia") {
                            return Err(format!("unknown decorator: {dec}"));
                        }
                        kind.push(' ');
                        kind.push_str(dec);
                        tail = t.trim_start();
                    } else {
                        return Err(format!("unexpected component modifier: {tail}"));
                    }
                }
                if matches!(kw, "component" | "anchor")
                    && self.names.insert(label.clone(), self.items.len()).is_some()
                {
                    return Err(format!("duplicate component: {label}"));
                }
                self.items.push(Item {
                    name: label,
                    point,
                    kind,
                    parent: pipeline.clone(),
                    offset,
                });
            }
            "evolve" => {
                let (n, e) = rest
                    .rsplit_once(char::is_whitespace)
                    .ok_or("expected evolve name evolution")?;
                self.moves.push((name(n)?, None, unit(number(e)?)?));
            }
            _ => {
                let ops = ["-.->", "-->", "->", "+<>", "+<", "+>", "+'", "-.-"];
                let (from, rest) =
                    before_unquoted(line, |s| ops.iter().any(|op| s.starts_with(op)));
                let op = ops
                    .into_iter()
                    .find(|op| rest.starts_with(op))
                    .ok_or("unknown Wardley statement")?;
                let from = name(from)?;
                let mut tail = rest[op.len()..].trim();
                if op == "-.-" {
                    let (x, y) = tail
                        .strip_prefix('(')
                        .and_then(|s| s.strip_suffix(')'))
                        .and_then(|s| s.split_once(','))
                        .ok_or("expected trend (evolution, visibility)")?;
                    self.moves
                        .push((from, Some(unit(number(y)?)?), unit(number(x)?)?));
                } else {
                    let mut caption = String::new();
                    if op == "+'" {
                        let (text, right) =
                            tail.split_once("'>").ok_or("expected +'flow label'>")?;
                        caption.push_str(text);
                        tail = right;
                    }
                    let (to, label) = before_unquoted(tail, |s| s.starts_with(';'));
                    if let Some(label) = label.strip_prefix(';') {
                        if !caption.is_empty() {
                            caption.push_str(": ");
                        }
                        caption.push_str(label.trim());
                    }
                    self.links.push(Link {
                        from,
                        to: name(to)?,
                        style: op,
                        caption,
                    });
                }
            }
        }
        Ok(())
    }
}

fn parse(source: &str) -> Result<Map, String> {
    let lines = model_header::body(source, "wardley-beta")?;
    let mut map = Map {
        title: String::new(),
        size: (1100., 600.),
        stages: stages("Genesis -> Custom Built -> Product -> Commodity")?,
        annotations: None,
        items: Vec::new(),
        names: BTreeMap::new(),
        links: Vec::new(),
        moves: Vec::new(),
    };
    let mut pipeline = None;
    for (i, line) in lines.enumerate() {
        let (line, _) = before_unquoted(line.trim(), |s| s.starts_with("%%"));
        if line.trim().is_empty() {
            continue;
        }
        map.statement(line.trim(), &mut pipeline)
            .map_err(|e| format!("Wardley line {}: {e}", i + 2))?;
    }
    if pipeline.is_some() {
        return Err("unclosed Wardley pipeline".into());
    }
    for i in 0..map.items.len() {
        if let Some(p) = &map.items[i].parent {
            let parent = map.lookup(p)?;
            if map.items[parent].parent.is_some() {
                return Err("pipeline parent must be a top-level component".into());
            }
            map.items[i].point.0 = map.items[parent].point.0;
        }
    }
    Ok(map)
}
