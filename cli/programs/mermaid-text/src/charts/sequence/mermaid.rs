use std::collections::{BTreeMap, BTreeSet};

use crate::color::Rgb;

use crate::charts::shared::sequence_diagram::{
    Arrow, ArrowHead, Diagram, Event, Group, ParticipantKind, ParticipantMenu, Placement,
    define_participant, first_word, is_id,
    label::{Label, decode_entities, entity_at},
    plain_text, register_participant,
};

pub(super) fn parse(source: &str) -> Result<Diagram<'_>, String> {
    let mut lines = source.lines().map(str::trim);
    let header_tail = lines
        .next()
        .unwrap_or_default()
        .strip_prefix("sequenceDiagram")
        .map(str::trim_start)
        .filter(|tail| tail.is_empty() || tail.starts_with(';'))
        .ok_or("expected a sequenceDiagram header")?;
    let mut parser = Parser::default();
    for (index, line) in std::iter::once(header_tail).chain(lines).enumerate() {
        let result = parser.line(line);
        result.map_err(|error| format!("sequenceDiagram line {}: {error}", index + 1))?;
    }
    if !parser.fragments.is_empty() || parser.group.is_some() {
        return Err("sequenceDiagram has a block without `end`".into());
    }
    if parser.pending.is_some() {
        return Err("create or destroy directive is missing its message".into());
    }
    parser
        .diagram
        .groups
        .sort_unstable_by_key(|group| group.first);
    if parser
        .diagram
        .groups
        .windows(2)
        .any(|pair| pair[0].last >= pair[1].first)
    {
        return Err("a participant cannot belong to multiple boxes".into());
    }
    Ok(parser.diagram)
}

enum Pending {
    Create(usize),
    Destroy(usize),
}

struct OpenGroup<'a> {
    members: BTreeSet<usize>,
    label: Label<'a>,
    fill: Option<Rgb>,
}

#[derive(Default)]
struct Parser<'a> {
    diagram: Diagram<'a>,
    ids: BTreeMap<&'a str, usize>,
    fragments: Vec<&'static str>,
    group: Option<OpenGroup<'a>>,
    pending: Option<Pending>,
    numbering: Numbering,
}

impl<'a> Parser<'a> {
    fn line(&mut self, mut line: &'a str) -> Result<(), String> {
        while !line.is_empty() {
            let (statement, remainder) = next_statement(line)?;
            let statement = statement.trim();
            if !statement.is_empty() {
                self.statement(statement)?;
            }
            line = remainder;
        }
        Ok(())
    }

    fn statement(&mut self, line: &'a str) -> Result<(), String> {
        if let Some(lifecycle) = self.pending.take() {
            return self.message(line, Some(lifecycle));
        }

        let (keyword, rest) = first_word(line);
        let keyword_lower = keyword.to_ascii_lowercase();
        let keyword = keyword_lower.as_str();
        if self.group.is_some() {
            return self.group_statement(keyword, rest);
        }
        let event = match keyword {
            "participant" | "actor" => {
                self.declaration(keyword, rest, false)?;
                return Ok(());
            }
            "box" => {
                if !self.fragments.is_empty() {
                    return Err("participant boxes cannot be inside a fragment".into());
                }
                let (fill, caption) = color_prefix(rest).unwrap_or((None, rest));
                self.group = Some(OpenGroup {
                    members: BTreeSet::new(),
                    label: plain_text(caption)?,
                    fill,
                });
                return Ok(());
            }
            "create" => {
                let (kind, declaration) = first_word(rest);
                let id = self.declaration(&kind.to_ascii_lowercase(), declaration, true)?;
                self.pending = Some(Pending::Create(id));
                return Ok(());
            }
            "destroy" => {
                self.pending = Some(Pending::Destroy(self.id(rest)?));
                return Ok(());
            }
            "autonumber" => return self.numbering.configure(rest),
            "title" | "title:" => {
                if self.diagram.title.is_some() || rest.is_empty() {
                    return Err("expected a single diagram title".into());
                }
                self.diagram.title = Some(plain_text(rest)?);
                return Ok(());
            }
            "acctitle:" | "accdescr:" | "accdescr" => {
                return Err("accessibility metadata must precede sequence statements".into());
            }
            "link" | "links" | "properties" | "details"
                if let Some((target, payload)) = menu_payload(rest) =>
            {
                return self.menu(keyword, target, payload);
            }
            "activate" | "deactivate" => Event::Activation {
                participant: self.id(rest)?,
                start: keyword == "activate",
            },
            "else" | "and" | "option" => self.branch(keyword, rest)?,
            "end" if rest.is_empty() => {
                self.fragments
                    .pop()
                    .ok_or("`end` without an open fragment")?;
                Event::End
            }
            "end" => return Err("expected `end` on its own statement".into()),
            "note" => self.note(rest)?,
            _ => {
                const FRAGMENTS: [&str; 8] = [
                    "alt", "loop", "opt", "par", "par_over", "critical", "break", "rect",
                ];
                let Some(fragment) = FRAGMENTS.into_iter().find(|&fragment| fragment == keyword)
                else {
                    return self.message(line, None);
                };
                self.fragments.push(fragment);
                self.diagram.max_depth = self.diagram.max_depth.max(self.fragments.len());
                let (fill, caption) = if fragment == "rect" {
                    color_prefix(rest).ok_or("expected `rect COLOR`")?
                } else {
                    (None, rest)
                };
                Event::Start {
                    keyword: fragment,
                    label: plain_text(caption)?,
                    fill,
                }
            }
        };
        self.diagram.events.push(event);
        Ok(())
    }

    fn id(&mut self, id: &'a str) -> Result<usize, String> {
        register_participant(&mut self.diagram, &mut self.ids, id)
    }

    fn menu(&mut self, keyword: &str, target: &'a str, payload: &'a str) -> Result<(), String> {
        let id = self.id(target)?;
        let menu = self
            .diagram
            .menus
            .entry(id)
            .or_insert_with(|| ParticipantMenu {
                identifier: target,
                ..ParticipantMenu::default()
            });
        match keyword {
            "link" => {
                let (name, address) = payload
                    .split_once('@')
                    .ok_or("expected `link ID: NAME @ URL`")?;
                let (name, address) = (name.trim(), address.trim());
                if name.is_empty() || address.is_empty() {
                    return Err("expected a link name and URL".into());
                }
                menu.links.insert(
                    decode_entities(name)?.into_owned(),
                    decode_entities(address)?.into_owned(),
                );
            }
            "links" => {
                let links: BTreeMap<String, String> = serde_json::from_str(payload)
                    .map_err(|error| format!("invalid participant links: {error}"))?;
                menu.links.extend(links);
            }
            "properties" => {
                let properties: BTreeMap<String, serde_json::Value> = serde_json::from_str(payload)
                    .map_err(|error| format!("invalid participant properties: {error}"))?;
                menu.properties.extend(properties);
            }
            "details" => {
                if payload.is_empty() {
                    return Err("expected a participant details reference".into());
                }
                menu.details.push(payload);
            }
            _ => unreachable!("participant menu keyword"),
        }
        Ok(())
    }

    fn declaration(
        &mut self,
        kind: &str,
        declaration: &'a str,
        deferred: bool,
    ) -> Result<usize, String> {
        let default_kind = match kind {
            "participant" => ParticipantKind::Participant,
            "actor" => ParticipantKind::Actor,
            _ => return Err("expected `participant` or `actor`".into()),
        };
        let (id, mut rest) = take_id(declaration);
        if !deferred
            && rest.is_empty()
            && let Some(&index) = self.ids.get(id)
            && self.diagram.participants[index].declaration.is_some()
        {
            return Ok(index);
        }
        let mut metadata = Metadata::default();
        if let Some(configuration) = rest.strip_prefix('@').filter(|rest| rest.starts_with('{')) {
            let length = braced_length(configuration)?;
            metadata = yaml_serde::from_str(&configuration[..length])
                .map_err(|error| format!("invalid participant configuration: {error}"))?;
            rest = configuration[length..].trim_start();
        }

        let label = if !rest.is_empty() {
            let (alias_keyword, text) = first_word(rest);
            if !alias_keyword.eq_ignore_ascii_case("as") || text.is_empty() {
                return Err("expected an ID optionally followed by `as LABEL`".into());
            }
            plain_text(text)?
        } else if let Some(alias) = metadata.alias {
            Label::parse(alias)?
        } else {
            Label::new(id)
        };
        define_participant(
            &mut self.diagram,
            &mut self.ids,
            id,
            label,
            metadata.kind.unwrap_or(default_kind),
            deferred,
        )
    }

    fn group_statement(&mut self, keyword: &str, rest: &'a str) -> Result<(), String> {
        match keyword {
            "participant" | "actor" => {
                let participant = self.declaration(keyword, rest, false)?;
                if self.diagram.participants[participant].deferred {
                    return Err(
                        "dynamically created participants cannot be assigned to a participant box"
                            .into(),
                    );
                }
                self.group.as_mut().unwrap().members.insert(participant);
            }
            "end" if rest.is_empty() => {
                let group = self.group.take().unwrap();
                let (&first, &last) = group
                    .members
                    .first()
                    .zip(group.members.last())
                    .ok_or("participant box has no participants")?;
                if last - first + 1 != group.members.len() {
                    return Err("participant box members must occupy contiguous columns".into());
                }
                self.diagram.groups.push(Group {
                    first,
                    last,
                    label: group.label,
                    fill: group.fill,
                });
            }
            _ => {
                return Err(
                    "participant boxes may only contain participant or actor declarations".into(),
                );
            }
        }
        Ok(())
    }

    fn branch(&self, branch: &str, text: &'a str) -> Result<Event<'a>, String> {
        let (branch, allowed) = match branch {
            "else" => ("else", &["alt"][..]),
            "and" => ("and", &["par", "par_over"][..]),
            _ => ("option", &["critical"][..]),
        };
        if !self
            .fragments
            .last()
            .is_some_and(|fragment| allowed.contains(fragment))
        {
            return Err(format!(
                "`{branch}` must be inside a `{}` fragment",
                allowed[0]
            ));
        }
        Ok(Event::Branch(branch, plain_text(text)?))
    }

    fn note(&mut self, rest: &'a str) -> Result<Event<'a>, String> {
        let (position, text) = rest.split_once(':').ok_or("expected `Note ...: text`")?;
        let (keyword, target) = first_word(position.trim_end());
        let placement = match keyword.to_ascii_lowercase().as_str() {
            "left" | "right" => {
                let (of, id) = first_word(target);
                if !of.eq_ignore_ascii_case("of") {
                    return Err("expected `Note left of ID` or `Note right of ID`".into());
                }
                let participant = self.id(id)?;
                if keyword.eq_ignore_ascii_case("left") {
                    Placement::Left(participant)
                } else {
                    Placement::Right(participant)
                }
            }
            "over" => {
                if let Some((from, to)) = target.split_once(',') {
                    let from = self.id(from.trim())?;
                    let to = self.id(to.trim())?;
                    if from == to {
                        Placement::Over(from)
                    } else {
                        Placement::Between(from.min(to), from.max(to))
                    }
                } else {
                    Placement::Over(self.id(target)?)
                }
            }
            _ => return Err("expected a Note placement: `left of`, `right of`, or `over`".into()),
        };
        Ok(Event::Note {
            placement,
            label: plain_text(text)?,
        })
    }

    fn message(&mut self, line: &'a str, pending: Option<Pending>) -> Result<(), String> {
        let (route, text) = line.split_once(':').ok_or_else(|| {
            format!("unsupported statement `{line}`; expected a declaration or `A->>B: message`")
        })?;
        let (from, receiver, mut arrow) =
            arrow(route).ok_or_else(|| format!("unsupported message arrow in `{route}`"))?;
        let receiver = receiver.trim_start();
        let (modifier, to) = match receiver.as_bytes().first() {
            Some(b'+') => (Some(true), &receiver[1..]),
            Some(b'-') => (Some(false), &receiver[1..]),
            _ => (None, receiver),
        };
        let (from, to) = (from.trim(), to.trim());
        let central_source = from.strip_suffix("()");
        let central_target = to.strip_prefix("()");
        arrow.central_source = central_source.is_some();
        arrow.central_target = central_target.is_some();
        let from = self.id(central_source.unwrap_or(from).trim_end())?;
        let to = self.id(central_target.unwrap_or(to).trim_start())?;
        if let Some(Pending::Create(participant)) = pending {
            if to != participant || from == to {
                return Err("the next message must create its receiver".into());
            }
            self.diagram.events.push(Event::Create(participant));
        }
        self.diagram.events.push(Event::Message {
            from,
            to,
            arrow,
            label: plain_text(text)?,
            number: self.numbering.next()?,
        });
        if let Some(start) = modifier {
            self.diagram.events.push(Event::Activation {
                participant: if start { to } else { from },
                start,
            });
        }
        if let Some(Pending::Destroy(participant)) = pending {
            if from != participant && to != participant {
                return Err("the next message must involve the participant being destroyed".into());
            }
            self.diagram.events.push(Event::Destroy(participant));
        }
        Ok(())
    }
}

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    #[serde(rename = "type")]
    kind: Option<ParticipantKind>,
    alias: Option<String>,
}

fn next_statement(line: &str) -> Result<(&str, &str), String> {
    let (keyword, arguments) = first_word(line.trim_start());
    let mut rest = if keyword.eq_ignore_ascii_case("box") || keyword.eq_ignore_ascii_case("rect") {
        color_prefix(arguments).map_or(line, |(_, rest)| rest)
    } else {
        line
    };
    let braced_start = braced_payload(line).map(|payload| line.len() - payload.len());
    while let Some(index) = rest.find([';', '#', '%', '{']) {
        let tail = &rest[index..];
        let end = line.len() - tail.len();
        match tail.as_bytes()[0] {
            b';' => return Ok((&line[..end], &tail[1..])),
            b'#' => {
                if let Some((_, after)) = entity_at(tail) {
                    rest = after;
                } else {
                    return Ok((&line[..end], ""));
                }
            }
            b'%' if tail.starts_with("%%") && line[..end].trim().is_empty() => {
                return Ok(("", ""));
            }
            b'{' if Some(end) == braced_start => {
                rest = &tail[braced_length(tail)?..];
            }
            _ => rest = &tail[1..],
        }
    }
    Ok((line, ""))
}

fn color_prefix(text: &str) -> Option<(Option<Rgb>, &str)> {
    let end = text
        .find(|ch: char| ch.is_whitespace() || matches!(ch, ';' | '('))
        .unwrap_or(text.len());
    let token = &text[..end];
    let remainder = &text[end..];
    if let Some(arguments) = remainder.trim_start().strip_prefix('(')
        && ["rgb", "rgba", "hsl", "hsla"]
            .iter()
            .any(|name| token.eq_ignore_ascii_case(name))
    {
        let end = text.len() - arguments.len() + arguments.find(')')? + 1;
        let (color, remainder) = text.split_at(end);
        if remainder.is_empty() || remainder.starts_with(|ch: char| ch.is_whitespace() || ch == ';')
        {
            return Some((Some(Rgb::parse(color)?), remainder.trim_start()));
        }
        return None;
    }
    if token.eq_ignore_ascii_case("transparent") {
        return Some((None, remainder.trim_start()));
    }
    if let Some(hex) = token.strip_prefix('#') {
        return (matches!(hex.len(), 3 | 4 | 6 | 8)
            && hex.bytes().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| (Rgb::parse(token), remainder.trim_start()));
    }
    // Bare hexadecimal words remain part of the caption.
    if token.bytes().all(|ch| ch.is_ascii_hexdigit()) || remainder.starts_with('(') {
        return None;
    }
    Rgb::parse(token).map(|color| (Some(color), remainder.trim_start()))
}

fn braced_payload(line: &str) -> Option<&str> {
    let (mut keyword, mut declaration) = first_word(line.trim_start());
    if keyword.eq_ignore_ascii_case("links") || keyword.eq_ignore_ascii_case("properties") {
        let (_, payload) = menu_payload(declaration)?;
        return payload.starts_with('{').then_some(payload);
    }
    if keyword.eq_ignore_ascii_case("create") {
        (keyword, declaration) = first_word(declaration);
    }
    if !keyword.eq_ignore_ascii_case("participant") && !keyword.eq_ignore_ascii_case("actor") {
        return None;
    }
    let (_, rest) = take_id(declaration);
    rest.strip_prefix('@').filter(|rest| rest.starts_with('{'))
}

fn menu_payload(rest: &str) -> Option<(&str, &str)> {
    let (target, payload) = rest.split_once(':')?;
    let target = target.trim();
    is_id(target).then_some((target, payload.trim()))
}

fn take_id(declaration: &str) -> (&str, &str) {
    let end = declaration
        .find(|ch: char| ch.is_whitespace() || ch == '@')
        .unwrap_or(declaration.len());
    (&declaration[..end], declaration[end..].trim_start())
}

fn braced_length(config: &str) -> Result<usize, String> {
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0;
    let mut allowed_to_quote = true;
    let mut chars = config.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' && delimiter == '"' {
                escaped = true;
            } else if ch == '\''
                && delimiter == '\''
                && chars.peek().is_some_and(|&(_, ch)| ch == '\'')
            {
                chars.next();
            } else if ch == delimiter {
                quote = None;
            }
        } else {
            match ch {
                '\'' | '"' if allowed_to_quote => quote = Some(ch),
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(index + 1);
                    }
                }
                _ => {}
            }
            if !ch.is_whitespace() {
                allowed_to_quote = matches!(ch, ':' | ',' | '{' | '[');
            }
        }
    }
    Err("unclosed participant configuration or menu object".into())
}

fn arrow(route: &str) -> Option<(&str, &str, Arrow)> {
    use ArrowHead::{Both, Cross, Filled, None, Open};
    const ARROWS: [(&str, bool, ArrowHead); 10] = [
        ("<<-->>", true, Both),
        ("<<->>", false, Both),
        ("-->>", true, Filled),
        ("->>", false, Filled),
        ("-->", true, None),
        ("->", false, None),
        ("--)", true, Open),
        ("-)", false, Open),
        ("--x", true, Cross),
        ("-x", false, Cross),
    ];
    ARROWS.iter().find_map(|&(token, dotted, head)| {
        route.split_once(token).map(|(from, to)| {
            (
                from,
                to,
                Arrow {
                    dotted,
                    head,
                    ..Arrow::CALL
                },
            )
        })
    })
}

struct Numbering {
    value: u128,
    step: u128,
    visible: bool,
}

impl Default for Numbering {
    fn default() -> Self {
        Self {
            value: 100,
            step: 100,
            visible: false,
        }
    }
}

impl Numbering {
    fn configure(&mut self, rest: &str) -> Result<(), String> {
        let mut numbers = rest.split_whitespace();
        self.visible = true;
        if let Some(first) = numbers.next() {
            if first.eq_ignore_ascii_case("off") {
                self.visible = false;
            } else {
                self.value = hundredths(first)?;
                self.step = numbers.next().map_or(Ok(100), hundredths)?;
            }
        }
        if numbers.next().is_some() {
            return Err("expected `autonumber [START [STEP]]` or `autonumber off`".into());
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Option<String>, String> {
        let (whole, fraction) = (self.value / 100, self.value % 100);
        let label = self.visible.then(|| match fraction {
            0 => format!("[{whole}]"),
            fraction if fraction % 10 == 0 => format!("[{whole}.{}]", fraction / 10),
            fraction => format!("[{whole}.{fraction:02}]"),
        });
        self.value = self
            .value
            .checked_add(self.step)
            .ok_or("autonumber overflow")?;
        Ok(label)
    }
}

fn hundredths(value: &str) -> Result<u128, String> {
    let invalid =
        || format!("invalid autonumber value `{value}` (expected up to two decimal places)");
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if (!whole.bytes().all(|byte| byte.is_ascii_digit()))
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || (value.contains('.') && fraction.is_empty())
    {
        return Err(invalid());
    }
    let whole = if whole.is_empty() {
        0
    } else {
        whole.parse::<u128>().map_err(|_| invalid())?
    };
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u128>().map_err(|_| invalid())? * 10,
        _ => fraction.parse().map_err(|_| invalid())?,
    };
    whole
        .checked_mul(100)
        .and_then(|whole| whole.checked_add(fraction))
        .ok_or_else(invalid)
}
