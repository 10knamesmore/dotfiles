mod image;
mod layout;

use crate::Options;

struct Domain {
    keyword: &'static str,
    heading: &'static str,
    decisions: &'static [&'static str],
    practice: &'static str,
}

const DOMAINS: [Domain; 5] = [
    Domain {
        keyword: "complex",
        heading: "Complex",
        decisions: &["Probe", "Sense", "Respond"],
        practice: "Emergent Practices",
    },
    Domain {
        keyword: "complicated",
        heading: "Complicated",
        decisions: &["Sense", "Analyse", "Respond"],
        practice: "Good Practices",
    },
    Domain {
        keyword: "chaotic",
        heading: "Chaotic",
        decisions: &["Act", "Sense", "Respond"],
        practice: "Novel Practices",
    },
    Domain {
        keyword: "clear",
        heading: "Clear",
        decisions: &["Sense", "Categorise", "Respond"],
        practice: "Best Practices",
    },
    Domain {
        keyword: "confusion",
        heading: "Confusion",
        decisions: &[],
        practice: "Disorder",
    },
];

#[derive(Default)]
struct Region {
    items: Vec<String>,
    outgoing: Vec<(&'static Domain, Option<String>)>,
}

#[derive(Default)]
struct Diagram<'a> {
    title: &'a str,
    accessible_title: &'a str,
    description: Vec<&'a str>,
    regions: [Region; 5],
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let diagram = parse(source)?;
        layout::draw(&diagram, options)
    }
}

fn parse(source: &str) -> Result<Diagram<'_>, String> {
    let mut lines = source.lines().enumerate();
    if lines.next().map(|(_, s)| statement(s)) != Some("cynefin-beta") {
        return Err("expected cynefin-beta on the first line".into());
    }
    let mut diagram = Diagram::default();
    let mut current: Option<usize> = None;
    while let Some((idx, raw)) = lines.next() {
        let s = statement(raw);
        let err = |detail: &str| format!("cynefin line {}: {detail}", idx + 1);
        if s.is_empty() {
            continue;
        } else if let Some(title) = s
            .strip_prefix("title")
            .and_then(|s| s.strip_prefix(char::is_whitespace))
        {
            diagram.title = title.trim();
        } else if let Some(title) = s
            .strip_prefix("accTitle")
            .and_then(|s| s.trim_start().strip_prefix(':'))
        {
            diagram.accessible_title = title.trim();
        } else if let Some(rest) = s.strip_prefix("accDescr") {
            let rest = rest.trim_start();
            diagram.description.clear();
            if let Some(text) = rest.strip_prefix(':') {
                diagram.description.push(text.trim());
            } else if let Some(mut text) = rest.strip_prefix('{') {
                loop {
                    if let Some((last, tail)) = text.split_once('}') {
                        if !statement(tail).is_empty() {
                            return Err(err("unexpected text after accDescr block"));
                        }
                        diagram.description.push(last.trim());
                        break;
                    }
                    diagram.description.push(text.trim());
                    text = lines
                        .next()
                        .ok_or_else(|| err("unterminated accDescr block"))?
                        .1;
                }
            } else {
                return Err(err("expected ':' or '{' after accDescr"));
            }
        } else if s.starts_with('"') {
            let region = current.ok_or_else(|| err("item outside a domain block"))?;
            diagram.regions[region].items.push(label(s).map_err(err)?);
        } else if let Some(region) = domain_index(s) {
            current = Some(region);
        } else if let Some((left, rest)) = s.split_once("-->") {
            let (right, caption) = match rest.split_once(':') {
                Some((right, text)) => (right, Some(label(text.trim()).map_err(err)?)),
                None => (rest, None),
            };
            let from = domain_index(left.trim())
                .ok_or_else(|| err(&format!("unknown transition source: {}", left.trim())))?;
            let to = domain_index(right.trim())
                .ok_or_else(|| err(&format!("unknown transition destination: {}", right.trim())))?;
            if from != to {
                diagram.regions[from].outgoing.push((&DOMAINS[to], caption));
            }
            current = None;
        } else {
            return Err(err(&format!(
                "expected a domain, quoted item, or transition; found {s}"
            )));
        }
    }
    Ok(diagram)
}

fn domain_index(name: &str) -> Option<usize> {
    DOMAINS.iter().position(|domain| domain.keyword == name)
}

fn statement(line: &str) -> &str {
    let mut quoted = false;
    let mut chars = line.char_indices();
    while let Some((idx, c)) = chars.next() {
        match c {
            '\\' if quoted => {
                chars.next();
            }
            '"' => quoted = !quoted,
            '%' if !quoted && line[idx..].starts_with("%%") => return line[..idx].trim(),
            _ => {}
        }
    }
    line.trim()
}

fn label(text: &str) -> Result<String, &'static str> {
    let mut chars = text
        .strip_prefix('"')
        .ok_or("expected double-quoted label")?
        .chars()
        .peekable();
    let mut value = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' if chars.peek().is_none() => return Ok(value),
            '"' => return Err("unexpected text after quoted label"),
            '\\' if matches!(chars.peek(), Some('"' | '\\')) => value.push(chars.next().unwrap()),
            _ => value.push(c),
        }
    }
    Err("unterminated quoted label")
}
