use super::{Caption, Frame, Model};

#[derive(Clone, Copy)]
enum Token<'a> {
    Word(&'a str),
    Payload(&'a str),
    Format(&'a str),
    Ref(&'a str),
    Caption(&'a str),
}

fn qualified(s: &str) -> bool {
    !s.is_empty()
        && s.split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c == '_' || c.is_alphanumeric()))
}

fn quoted_end(s: &str, quote: char) -> Result<usize, String> {
    let mut escaped = false;
    for (i, c) in s[1..].char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            return Ok(i + 2);
        }
    }
    Err("unclosed event model string".into())
}

fn block_end(s: &str) -> Result<usize, String> {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut previous = ' ';
    for (i, c) in s.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
        } else if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Ok(i + 1);
            }
        } else if c == '"' || (c == '\'' && !previous.is_alphanumeric()) {
            quote = Some(c);
        }
        previous = c;
    }
    Err("unclosed event data or description block".into())
}

fn tokens(mut s: &str) -> Result<Vec<Token<'_>>, String> {
    let mut result = Vec::new();
    let mut line_start = true;
    while !s.is_empty() {
        let trimmed = s.trim_start();
        line_start |= s[..s.len() - trimmed.len()].contains('\n');
        s = trimmed;
        if s.is_empty() {
            break;
        }
        if s.starts_with("//") || s.starts_with("%%") {
            s = &s[s.find('\n').unwrap_or(s.len())..];
            continue;
        }
        if let Some(rest) = s.strip_prefix("/*") {
            let (comment, tail) = rest
                .split_once("*/")
                .ok_or("unterminated event model comment")?;
            line_start |= comment.contains('\n');
            s = tail;
            continue;
        }
        if let Some(rest) = s.strip_prefix(';') {
            s = rest;
            line_start = true;
            continue;
        }
        if line_start {
            let key_end = s
                .find(|c: char| c.is_whitespace() || matches!(c, ':' | '{'))
                .unwrap_or(s.len());
            let key = &s[..key_end];
            if matches!(key, "title" | "accTitle" | "accDescr" | "value") {
                let tail = s[key_end..].trim_start_matches([' ', '\t']);
                if key != "accDescr" || !tail.starts_with('{') {
                    let text = if key == "accTitle" || key == "accDescr" {
                        tail.strip_prefix(':')
                            .ok_or_else(|| format!("expected : after {key}"))?
                    } else {
                        tail.strip_prefix(':').unwrap_or(tail)
                    };
                    let end = text.find(['\n', ';']).unwrap_or(text.len());
                    result.extend([Token::Word(key), Token::Caption(text[..end].trim())]);
                    s = &text[end..];
                    continue;
                }
            }
        }
        line_start = false;
        let first = s.chars().next().unwrap();
        let (token, n) = if s.starts_with("->>") {
            (Token::Word("->>"), 3)
        } else if s.starts_with("->") {
            (Token::Word("->"), 2)
        } else if let Some(rest) = s.strip_prefix("[[") {
            let (id, _) = rest
                .split_once("]]")
                .ok_or("unterminated event data reference")?;
            (Token::Ref(id.trim()), id.len() + 4)
        } else if let Some(rest) = s.strip_prefix('`') {
            let (format, _) = rest.split_once('`').ok_or("unclosed event data format")?;
            if !matches!(
                format,
                "json" | "jsobj" | "figma" | "salt" | "uri" | "md" | "html" | "text"
            ) {
                return Err(format!("unknown event data format: {format}"));
            }
            (Token::Format(format), format.len() + 2)
        } else if first == '{' {
            let n = block_end(s)?;
            (Token::Payload(&s[1..n - 1]), n)
        } else if first == '"' || first == '\'' {
            let n = quoted_end(s, first)?;
            (Token::Payload(&s[1..n - 1]), n)
        } else {
            let n = s
                .char_indices()
                .take_while(|(i, c)| {
                    !c.is_whitespace()
                        && !";[]{}'\"`".contains(*c)
                        && !s[*i..].starts_with("->")
                        && !s[*i..].starts_with("//")
                        && !s[*i..].starts_with("/*")
                        && !s[*i..].starts_with("%%")
                })
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8());
            if n == 0 {
                return Err(format!("unexpected event model character: {first}"));
            }
            (Token::Word(&s[..n]), n)
        };
        result.push(token);
        s = &s[n..];
    }
    Ok(result)
}

struct Parser<'a>(std::iter::Peekable<std::vec::IntoIter<Token<'a>>>);

impl<'a> Parser<'a> {
    fn word(&mut self) -> Result<&'a str, String> {
        match self.0.next() {
            Some(Token::Word(s)) => Ok(s),
            _ => Err("expected event model keyword or identifier".into()),
        }
    }

    fn id(&mut self) -> Result<&'a str, String> {
        let id = self.word()?;
        if qualified(id) {
            Ok(id)
        } else {
            Err(format!("invalid event identifier: {id}"))
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if matches!(self.0.peek(),Some(Token::Word(s)) if *s==word) {
            self.0.next();
            true
        } else {
            false
        }
    }

    fn payload(&mut self) -> Result<(Option<&'a str>, &'a str), String> {
        let format = match self.0.peek().copied() {
            Some(Token::Format(s)) => {
                self.0.next();
                Some(s)
            }
            _ => None,
        };
        match self.0.next() {
            Some(Token::Payload(s)) => Ok((format, s.trim())),
            _ => Err("expected event payload".into()),
        }
    }

    fn caption(&mut self) -> Result<&'a str, String> {
        match self.0.next() {
            Some(Token::Caption(s) | Token::Payload(s)) => Ok(s.trim()),
            _ => Err("expected event model text".into()),
        }
    }

    fn clauses(&mut self, group: &str, out: &mut Vec<String>) -> Result<(), String> {
        let before = out.len();
        while let Some(Token::Word(s)) = self.0.peek() {
            let Some(kind) = kind(s) else { break };
            self.0.next();
            out.push(format!("{group}: {kind} {}", self.id()?));
        }
        if out.len() == before {
            Err(format!("empty {group} group in event scenario"))
        } else {
            Ok(())
        }
    }
}

fn kind(s: &str) -> Option<&'static str> {
    Some(match s {
        "ui" => "UI",
        "cmd" | "command" => "Command",
        "evt" | "event" => "Event",
        "rmo" | "readmodel" => "Read model",
        "pcr" | "processor" => "Processor",
        _ => return None,
    })
}

fn fields(label: &str, format: Option<&str>, text: &str) -> Vec<String> {
    let mut result = Vec::new();
    if !label.is_empty() || format.is_some() {
        result.push(match format {
            Some(fmt) if label.is_empty() => format!("[{fmt}]"),
            Some(fmt) => format!("{label} [{fmt}]"),
            None => label.to_owned(),
        });
    }
    result.extend(text.lines().map(|s| s.trim().to_owned()));
    result
}

pub(super) fn parse(source: &str) -> Result<Model, String> {
    let mut p = Parser(tokens(source)?.into_iter().peekable());
    let mut model = Model::default();
    let mut current = None;
    while p.0.peek().is_some() {
        let word = p.word()?;
        if word != "value" {
            current = None;
        }
        match word {
            "title" | "accTitle" => model.captions.push(Caption::Title(p.caption()?.into())),
            "accDescr" => model
                .captions
                .push(Caption::Description(p.caption()?.into())),
            "entity" => model.entities.push(p.id()?.to_owned()),
            "tf" | "timeframe" | "rf" | "resetframe" => {
                if model.frames.len() >= 2048 {
                    return Err("event model exceeds 2048 frames".into());
                }
                let id = p.id()?;
                let type_id = p.id()?;
                let kind = match kind(type_id) {
                    Some(k) => k,
                    None if !id.bytes().all(|b| b.is_ascii_digit()) => type_id,
                    None => {
                        return Err(format!(
                            "unknown entity type for event frame {id}: {type_id}"
                        ));
                    }
                };
                let entity = p.id()?.to_owned();
                let mut sources = Vec::new();
                while p.eat("->>") {
                    sources.push(p.id()?.to_owned());
                }
                let reference = if let Some(Token::Ref(name)) = p.0.peek().copied() {
                    if !qualified(name) {
                        return Err(format!("invalid event data reference: {name}"));
                    }
                    p.0.next();
                    Some(name.into())
                } else {
                    None
                };
                let mut values = Vec::new();
                if matches!(p.0.peek(), Some(Token::Format(_) | Token::Payload(_))) {
                    let (fmt, payload) = p.payload()?;
                    values = fields("Payload", fmt, payload);
                }
                current = Some(model.frames.len());
                model.frames.push(Frame {
                    id: id.into(),
                    reset: matches!(word, "rf" | "resetframe"),
                    kind: kind.into(),
                    entity,
                    sources,
                    reference,
                    fields: values,
                });
            }
            "value" => {
                let idx = current.ok_or("event value must follow a frame")?;
                let text = p.caption()?;
                if text.is_empty() {
                    return Err("empty event frame value".into());
                }
                model.frames[idx].fields.push(text.into());
            }
            "data" | "note" => {
                let name = p.id()?.to_owned();
                let (fmt, payload) = p.payload()?;
                if word == "data" {
                    model.data.push((name, fields("", fmt, payload)));
                } else {
                    model.notes.push((name, fields("Note", fmt, payload)));
                }
            }
            "gwt" => {
                let name = p.id()?.to_owned();
                let mut lines = Vec::new();
                if !p.eat("given") {
                    return Err("event scenario requires given".into());
                }
                p.clauses("Given", &mut lines)?;
                if p.eat("when") {
                    p.clauses("When", &mut lines)?;
                }
                if !p.eat("then") {
                    return Err("event scenario requires then".into());
                }
                p.clauses("Then", &mut lines)?;
                model.notes.push((name, lines));
            }
            from if qualified(from) && p.eat("->") => {
                model.links.push((from.into(), p.id()?.to_owned()))
            }
            unknown => return Err(format!("unknown event model statement: {unknown}")),
        }
    }
    Ok(model)
}
