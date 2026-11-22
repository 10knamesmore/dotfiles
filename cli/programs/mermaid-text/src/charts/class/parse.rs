use super::Renderer;
use crate::charts::shared::cards::{
    End, Link, Mark, Node,
    parse::{Grammar, Parsed, Parser, finish, keyword, link, name, nonempty, phrase, quoted, take},
};

impl Grammar for Renderer {
    const HEADERS: &'static [&'static str] = &["classDiagram", "classDiagram-v2"];
    const IDENTIFIER_QUOTE: char = '`';

    fn statement<'a>(parser: &mut Parser<'a>, mut text: &'a str, line: usize) -> Parsed<()> {
        while text.split_whitespace().next() == Some("namespace") {
            text = namespace(parser, text["namespace".len()..].trim_start(), line)?;
        }
        let keyword = keyword(text);
        if parser.common_statement::<Self>(keyword, text)? {
            return Ok(());
        }
        match keyword {
            "class" => {
                text = text[keyword.len()..].trim_start();
                let name = name::<Self>(&mut text)?;
                return parser.declaration::<Self>(name, text, line);
            }
            "note" => return note(parser, text[keyword.len()..].trim_start()),
            "click" | "link" | "callback" => {
                return interaction(keyword, text[keyword.len()..].trim_start());
            }
            _ => {}
        }
        if let Some(annotation) = Self::annotation(&mut text)? {
            let name = name::<Self>(&mut text)?;
            let argument = Self::generic(&mut text)?;
            finish(text)?;
            let node = parser.node_for(name);
            node.annotations.push(annotation);
            if let Some(argument) = argument {
                node.generic = Some(argument);
            }
            return Ok(());
        }

        let first = name::<Self>(&mut text)?;
        let argument = Self::generic(&mut text)?;
        if !text.starts_with(":::") && take(&mut text, ":") {
            if text.is_empty() {
                return Err("expected a class member after ':'");
            }
            let node = parser.node_for(first);
            if let Some(argument) = argument {
                node.generic = Some(argument);
            }
            Self::member(node, text)
        } else {
            parser.relationship::<Self>(first, argument, text)
        }
    }

    fn open_quote(mut text: &str, mut open: Option<char>) -> Option<char> {
        while let Some(c) = text.chars().next() {
            text = &text[c.len_utf8()..];
            match (open, c) {
                (Some(delimiter), c) if delimiter == c => open = None,
                (Some(_), _) => {}
                (None, '"' | '`') => open = Some(c),
                (None, '{') => break,
                (None, ':') if text.starts_with("::") => text = &text[2..],
                (None, ':') if !text.trim_start().starts_with('"') => break,
                _ => {}
            }
        }
        open
    }

    fn generic<'a>(text: &mut &'a str) -> Parsed<Option<&'a str>> {
        if !take(text, "~") {
            return Ok(None);
        }
        let (value, rest) = text
            .split_once('~')
            .ok_or("expected '~' after the class type parameter")?;
        *text = rest.trim_start();
        Ok(Some(nonempty(value, "empty class type parameter")?))
    }

    fn annotation<'a>(text: &mut &'a str) -> Parsed<Option<&'a str>> {
        if !take(text, "<<") {
            return Ok(None);
        }
        let (value, rest) = text
            .split_once(">>")
            .ok_or("expected '>>' after the class annotation")?;
        *text = rest.trim_start();
        Ok(Some(nonempty(value, "empty class annotation")?))
    }

    fn unquoted_alias<'a>(_text: &mut &'a str) -> Parsed<&'a str> {
        Err("expected a quoted class label")
    }

    fn member<'a>(node: &mut Node<'a>, member: &'a str) -> Parsed<()> {
        let mut text = member;
        if let Some(annotation) = Self::annotation(&mut text)? {
            finish(text)?;
            node.annotations.push(annotation);
        } else if member
            .split_once('(')
            .is_some_and(|(_, tail)| tail.contains(')'))
        {
            node.methods.push(member);
        } else {
            node.fields.push(member);
        }
        Ok(())
    }

    fn connector<'a>(text: &mut &'a str) -> Parsed<(Link, [End<'a>; 2])> {
        let left_count = quoted(text, '"')?;
        let left_mark = ["<|", "<", "*", "o", "()"]
            .into_iter()
            .find(|mark| take(text, mark))
            .unwrap_or("");
        let link = link(text)
            .ok_or("expected a class relationship operator (such as '<|--', '-->' or '--')")?;
        let right_mark = ["|>", ">", "*", "()"]
            .into_iter()
            .find(|mark| take(text, mark))
            .or_else(|| {
                let mut rest = *text;
                if phrase(&mut rest, "o") && !rest.is_empty() && !rest.starts_with(':') {
                    *text = rest;
                    Some("o")
                } else {
                    None
                }
            })
            .unwrap_or("");
        let right_count = quoted(text, '"')?;
        let marker = |mark| match mark {
            "<|" | "|>" => Mark::Triangle,
            "<" | ">" => Mark::Arrow,
            "*" => Mark::Diamond,
            "o" => Mark::HollowDiamond,
            "()" => Mark::Circle,
            _ => Mark::None,
        };
        Ok((
            link,
            [
                End {
                    mark: marker(left_mark),
                    count: left_count,
                },
                End {
                    mark: marker(right_mark),
                    count: right_count,
                },
            ],
        ))
    }
}

fn namespace<'a>(parser: &mut Parser<'a>, mut text: &'a str, line: usize) -> Parsed<&'a str> {
    let name = name::<Renderer>(&mut text)?;
    let label = if take(&mut text, "[") {
        let label = quoted(&mut text, '"')?.ok_or("expected a quoted namespace label")?;
        if !take(&mut text, "]") {
            return Err("expected ']' after the namespace label");
        }
        Some(nonempty(label, "empty namespace label")?)
    } else {
        None
    };
    if !take(&mut text, "{") {
        return Err("expected '{' after the namespace name");
    }
    parser.open_namespace(name, label, line)?;
    Ok(text)
}

fn note<'a>(parser: &mut Parser<'a>, mut text: &'a str) -> Parsed<()> {
    let target = if phrase(&mut text, "for") {
        Some(name::<Renderer>(&mut text)?)
    } else {
        None
    };
    let (note, tail) = text
        .strip_prefix('"')
        .and_then(|body| body.rsplit_once('"'))
        .ok_or("expected a quoted note")?;
    finish(tail.trim())?;
    if let Some(target) = target {
        parser.node_for(target).notes.push(note);
    } else {
        parser.note(note);
    }
    Ok(())
}

fn interaction(kind: &str, mut text: &str) -> Parsed<()> {
    name::<Renderer>(&mut text)?;
    Renderer::generic(&mut text)?;
    let href = kind == "click" && phrase(&mut text, "href");
    let call = kind == "click" && phrase(&mut text, "call");
    if call || (kind == "click" && !href && !text.starts_with('"')) {
        name::<Renderer>(&mut text)?;
        if take(&mut text, "(") {
            let mut quote = None;
            let mut depth = 1;
            for (index, c) in text.char_indices() {
                match (quote, c) {
                    (Some(open), c) if open == c => quote = None,
                    (None, '"' | '\'' | '`') => quote = Some(c),
                    (None, '(') => depth += 1,
                    (None, ')') => {
                        depth -= 1;
                        if depth == 0 {
                            text = text[index + 1..].trim_start();
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth != 0 {
                return Err("unclosed callback arguments");
            }
        }
    } else {
        quoted(&mut text, '"')?.ok_or("expected a quoted link or callback")?;
    }
    quoted(&mut text, '"')?;
    if !text.is_empty() && !text.starts_with(';') {
        let target = name::<Renderer>(&mut text)?.id;
        if !matches!(target, "_self" | "_blank" | "_top" | "_parent") {
            return Err("unknown link target");
        }
    }
    finish(text.trim_end_matches(';').trim_end())
}
