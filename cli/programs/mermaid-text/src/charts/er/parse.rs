use super::Renderer;
use crate::charts::shared::cards::{
    End, Link, Mark, Node,
    parse::{
        Grammar, Parsed, Parser, inline_style, keyword, link, name, nonempty, phrase, quoted, take,
    },
};

impl Grammar for Renderer {
    const HEADERS: &'static [&'static str] = &["erDiagram"];
    const IDENTIFIER_QUOTE: char = '"';
    const MEMBER_QUOTES: &'static [char] = &['"', '`'];
    const DIRECTIONAL_LABELS: bool = true;

    fn statement<'a>(parser: &mut Parser<'a>, mut text: &'a str, line: usize) -> Parsed<()> {
        let keyword = keyword(text);
        if keyword == "class" {
            return parser.assign_style::<Self>(text[keyword.len()..].trim_start());
        }
        if parser.common_statement::<Self>(keyword, text)? {
            return Ok(());
        }
        let first = name::<Self>(&mut text)?;
        if text.is_empty() || text.starts_with(['[', '{']) || text.starts_with(":::") {
            let mut rest = text;
            let _ = inline_style::<Self>(&mut rest)?;
            if rest.is_empty() || rest.starts_with(['[', '{']) {
                return parser.declaration::<Self>(first, text, line);
            }
        }
        let mut rest = text;
        if let Ok(second) = name::<Self>(&mut rest)
            && rest.starts_with('[')
        {
            parser.declare(first);
            return parser.declaration::<Self>(second, rest, line);
        }
        parser.relationship::<Self>(first, None, text)
    }

    fn extra_identifier_character(c: char) -> bool {
        !c.is_ascii() && !c.is_whitespace() || c == '*'
    }

    fn unquoted_alias<'a>(text: &mut &'a str) -> Parsed<&'a str> {
        let (value, rest) = text
            .split_once(']')
            .ok_or("expected ']' after the entity alias")?;
        *text = rest.trim_start();
        Ok(value)
    }

    fn member<'a>(node: &mut Node<'a>, mut text: &'a str) -> Parsed<()> {
        while !text.is_empty() {
            let start = text;
            attribute_word(&mut text, false).map_err(|_| "expected an ER attribute type")?;
            attribute_word(&mut text, true)
                .map_err(|_| "expected an ER attribute name after its type")?;
            let key = |text: &mut &str| {
                ["PK", "FK", "UK"]
                    .into_iter()
                    .any(|word| phrase(text, word))
            };
            if key(&mut text) {
                while take(&mut text, ",") {
                    if !key(&mut text) {
                        return Err("expected PK, FK or UK after ','");
                    }
                }
            }
            quoted(&mut text, '"')?;
            node.fields
                .push(start[..start.len() - text.len()].trim_end());
        }
        Ok(())
    }

    fn connector<'a>(text: &mut &'a str) -> Parsed<(Link, [End<'a>; 2])> {
        let left_count =
            cardinality(text).ok_or("expected a left ER cardinality (||, |o, }| or }o)")?;
        let link = if phrase(text, "optionally to") || take(text, ".-") || take(text, "-.") {
            Link::Dashed
        } else if phrase(text, "to") {
            Link::Solid
        } else {
            link(text).ok_or("expected an ER relationship line ('--' or '..')")?
        };
        let right_count =
            cardinality(text).ok_or("expected a right ER cardinality (||, o|, |{ or o{)")?;
        Ok((
            link,
            [left_count, right_count].map(|count| End {
                mark: Mark::None,
                count: Some(count),
            }),
        ))
    }
}

fn attribute_word(text: &mut &str, is_name: bool) -> Parsed<()> {
    if let Some(word) = quoted(text, '`')? {
        nonempty(word, "empty attribute token")?;
        return Ok(());
    }
    let first = text.chars().next().ok_or("missing attribute token")?;
    if !(first.is_alphabetic() || matches!(first, '_' | '*')) {
        return Err("invalid attribute token");
    }
    let mut depth = 0_usize;
    let mut length = 0;
    for (index, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            '?' if !is_name && depth == 0 => {
                length = index + 1;
                break;
            }
            ',' if depth > 0 => {}
            c if c.is_alphanumeric() || matches!(c, '_' | '-' | '[' | ']' | '.' | '~') => {}
            '*' => {}
            c if c.is_whitespace() && depth > 0 => {}
            _ => break,
        }
        length = index + c.len_utf8();
    }
    if depth != 0 || &text[..length] == "*" {
        return Err("invalid attribute token");
    }
    let rest = &text[length..];
    if rest
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace() && c != '"')
    {
        return Err("invalid character in ER attribute");
    }
    *text = rest.trim_start();
    Ok(())
}

fn cardinality(text: &mut &str) -> Option<&'static str> {
    for (symbol, count) in [
        ("||", "1"),
        ("|o", "0..1"),
        ("o|", "0..1"),
        ("}|", "1..*"),
        ("|{", "1..*"),
        ("}o", "0..*"),
        ("o{", "0..*"),
    ] {
        if take(text, symbol) {
            return Some(count);
        }
    }
    for (aliases, count) in [
        (&["one or zero", "zero or one"][..], "0..1"),
        (&["one or more", "one or many", "many(1)", "1+"][..], "1..*"),
        (
            &["zero or more", "zero or many", "many(0)", "0+", "many"][..],
            "0..*",
        ),
        (&["only one", "one", "1"][..], "1"),
        (&["u"][..], "parent"),
    ] {
        if aliases.iter().any(|alias| phrase(text, alias)) {
            return Some(count);
        }
    }
    None
}
