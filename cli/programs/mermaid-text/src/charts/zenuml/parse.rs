use std::collections::BTreeMap;

use crate::charts::shared::sequence_diagram::{
    Arrow, Diagram, Event, Participant, ParticipantKind, Placement, declare_participant,
    first_word, is_id, label::Label, plain_text, register_participant,
};

const OUTSIDE: usize = usize::MAX;

pub(super) fn parse(source: &str) -> Result<Diagram<'_>, String> {
    let mut lines = source.lines().map(str::trim);
    if lines.next() != Some("zenuml") {
        return Err("expected a sequenceDiagram header".into());
    }
    let mut parser = Parser::default();
    let mut lines = lines.enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        let result = if let Some(tail) = line.strip_prefix('}') {
            let mut tail = tail.trim_start();
            if tail == ";" {
                tail = "";
            }
            if tail.is_empty()
                && lines.peek().is_some_and(|(_, next)| is_branch(next))
                && let Some((_, next)) = lines.next()
            {
                tail = next;
            }
            parser.close_scope(tail)
        } else {
            parser.line(line)
        };
        result.map_err(|error| format!("zenuml line {}: {error}", index + 2))?;
    }
    if !parser.scopes.is_empty() {
        return Err("zenuml has an unclosed block".into());
    }
    if parser.return_annotation {
        return Err("zenuml `@return` requires an arrow message".into());
    }
    if parser.uses_outside {
        add_caller(&mut parser.diagram);
    }
    Ok(parser.diagram)
}

enum Scope<'a> {
    Call {
        from: usize,
        to: usize,
        parent: Option<usize>,
        assignment: Option<Label<'a>>,
        returned: bool,
    },
    Frame(Frame),
}

#[derive(Clone, Copy)]
enum Frame {
    If { has_else: bool },
    Try { has_finally: bool },
    Other,
}

#[derive(Default)]
struct Parser<'a> {
    diagram: Diagram<'a>,
    ids: BTreeMap<&'a str, usize>,
    scopes: Vec<Scope<'a>>,
    current_call: Option<usize>,
    frame_depth: usize,
    comments: Vec<Label<'a>>,
    starter: Option<usize>,
    uses_outside: bool,
    return_annotation: bool,
}

impl<'a> Parser<'a> {
    fn line(&mut self, line: &'a str) -> Result<(), String> {
        if let Some(comment) = line.strip_prefix("//") {
            self.comments.push(plain_text(comment)?);
            return Ok(());
        }
        if line.is_empty() {
            return Ok(());
        }
        let line = line.strip_suffix(';').unwrap_or(line).trim_end();
        let (keyword, rest) = first_word(line);
        if self.return_annotation || keyword == "@return" {
            let message = if self.return_annotation { line } else { rest };
            self.return_annotation = message.is_empty();
            if self.return_annotation {
                return Ok(());
            }
            return self.arrow(message, true);
        }
        if keyword == "return" {
            return self.return_value(rest);
        }
        if let ("@Starter", target) = keyword_prefix(line) {
            if self.starter.is_some() || !self.diagram.events.is_empty() {
                return Err("expected a single `@Starter` before the diagram body".into());
            }
            let target = condition_text(target)?;
            self.starter = Some(if target.is_empty() {
                OUTSIDE
            } else {
                self.id(target)?
            });
            self.comments.clear();
            return Ok(());
        }

        if direct_route(line).is_some() {
            return self.arrow(line, false);
        }
        if keyword == "title" {
            if rest.is_empty()
                || self.diagram.title.is_some()
                || !self.diagram.events.is_empty()
                || !self.diagram.participants.is_empty()
            {
                return Err("expected a single `title TEXT` before the diagram body".into());
            }
            self.diagram.title = Some(plain_text(rest)?);
            self.comments.clear();
            return Ok(());
        }

        let (statement, block) = opening(line);
        if block && self.open_frame(statement)? {
            return Ok(());
        }
        if let Some((assignment, call)) = split_assignment(statement)? {
            return self.call(call, Some(assignment), block);
        }
        if keyword == "new" || statement.contains(['.', '(']) {
            return self.call(statement, None, block);
        }
        if block || !self.scopes.is_empty() {
            return Err(format!("unsupported ZenUML statement `{line}`"));
        }

        let (kind, declaration) = match keyword {
            "@Actor" => (ParticipantKind::Actor, rest),
            "@Database" => (ParticipantKind::Database, rest),
            _ => (ParticipantKind::Participant, line),
        };
        declare_participant(&mut self.diagram, &mut self.ids, declaration, kind, false)?;
        self.comments.clear();
        Ok(())
    }

    fn open_frame(&mut self, statement: &'a str) -> Result<bool, String> {
        let (keyword, condition) = keyword_prefix(statement);
        let (keyword, frame, title) = match keyword {
            "if" => (
                "if",
                Frame::If { has_else: false },
                condition_text(condition)?,
            ),
            "while" => ("while", Frame::Other, condition_text(condition)?),
            "for" => ("for", Frame::Other, condition_text(condition)?),
            "foreach" => ("foreach", Frame::Other, condition_text(condition)?),
            "opt" if condition.is_empty() => ("opt", Frame::Other, ""),
            "par" if condition.is_empty() => ("par", Frame::Other, ""),
            "try" if condition.is_empty() => ("try", Frame::Try { has_finally: false }, ""),
            _ => return Ok(false),
        };
        self.scopes.push(Scope::Frame(frame));
        self.frame_depth += 1;
        self.diagram.max_depth = self.diagram.max_depth.max(self.frame_depth);
        self.diagram.events.push(Event::Start {
            keyword,
            label: plain_text(title)?,
            fill: None,
        });
        Ok(true)
    }

    fn close_scope(&mut self, tail: &'a str) -> Result<(), String> {
        if self.return_annotation {
            return Err("`@return` requires an arrow message".into());
        }
        if !tail.is_empty() {
            return self.branch(tail);
        }
        match self.scopes.pop().ok_or("unmatched `}`")? {
            Scope::Frame(_) => {
                self.frame_depth -= 1;
                self.diagram.events.push(Event::End);
            }
            Scope::Call {
                from,
                to,
                parent,
                assignment,
                returned,
            } => {
                if let Some(label) = assignment.filter(|_| !returned) {
                    self.message(to, from, Arrow::REPLY, label);
                }
                self.activation(to, false);
                self.current_call = parent;
            }
        }
        self.comments.clear();
        Ok(())
    }

    fn branch(&mut self, tail: &'a str) -> Result<(), String> {
        let (statement, block) = opening(tail);
        if !block {
            return Err("expected a braced branch after `}`".into());
        }
        let (prefix, condition) = keyword_prefix(statement);
        let Some(Scope::Frame(frame)) = self.scopes.last_mut() else {
            return Err("branch outside a fragment".into());
        };
        let (keyword, label) = match (&mut *frame, prefix) {
            (Frame::If { has_else: false }, "else") => {
                if condition.is_empty() {
                    *frame = Frame::If { has_else: true };
                    ("else", "")
                } else {
                    let ("if", rest) = keyword_prefix(condition) else {
                        return Err("expected `else` or `else if (condition)`".into());
                    };
                    ("else if", condition_text(rest)?)
                }
            }
            (Frame::Try { has_finally: false }, "catch") => (
                "catch",
                if condition.is_empty() {
                    ""
                } else {
                    condition_text(condition)?
                },
            ),
            (Frame::Try { has_finally: false }, "finally") if condition.is_empty() => {
                *frame = Frame::Try { has_finally: true };
                ("finally", "")
            }
            _ => return Err(format!("unexpected ZenUML branch `{statement}`")),
        };
        self.diagram
            .events
            .push(Event::Branch(keyword, plain_text(label)?));
        Ok(())
    }

    fn arrow(&mut self, line: &'a str, reply: bool) -> Result<(), String> {
        let (sender, receiver, text, dotted) =
            direct_route(line).ok_or("expected `A->B: message`")?;
        let from = if sender.is_empty() {
            self.caller()
        } else {
            self.id(sender)?
        };
        let to = if receiver.is_empty() {
            self.caller()
        } else {
            self.id(receiver)?
        };
        if from == OUTSIDE && to == OUTSIDE {
            return Err("an arrow needs at least one participant".into());
        }
        if (dotted || reply)
            && let Some(index) = self.current_call
            && let Scope::Call {
                from: caller,
                to: callee,
                returned,
                ..
            } = &mut self.scopes[index]
            && (*callee, *caller) == (from, to)
        {
            *returned = true;
        }
        self.message(
            from,
            to,
            if dotted || reply {
                Arrow::REPLY
            } else {
                Arrow::ASYNC
            },
            plain_text(text)?,
        );
        Ok(())
    }

    fn call(
        &mut self,
        statement: &'a str,
        assignment: Option<Label<'a>>,
        block: bool,
    ) -> Result<(), String> {
        let (keyword, rest) = first_word(statement);
        let (from, to, label, creating) = if keyword == "new" {
            let (name, _) = invocation(rest)?;
            let to = declare_participant(
                &mut self.diagram,
                &mut self.ids,
                name,
                ParticipantKind::Participant,
                true,
            )?;
            (self.caller(), to, plain_text(statement)?, true)
        } else {
            let (path, signature) = invocation(statement)?;
            let (from, to, label) = if let Some((route, method)) = path.rsplit_once('.') {
                let (from, to) = if let Some((sender, receiver)) = route.split_once("->") {
                    (self.id(sender.trim())?, self.id(receiver.trim())?)
                } else {
                    (self.caller(), self.id(route.trim())?)
                };
                if !is_id(method) {
                    return Err("expected a method name".into());
                }
                let offset = path.len() - method.len();
                (from, to, plain_text(&statement[offset..])?)
            } else {
                if signature.is_none() || self.current_call.is_none() || !is_id(path) {
                    return Err(format!(
                        "expected `[participant.]method(arguments)` in `{statement}`"
                    ));
                }
                (self.caller(), self.caller(), plain_text(statement)?)
            };
            (from, to, label, false)
        };
        if creating {
            self.emit_comments(from);
            self.diagram.events.push(Event::Create(to));
        }
        self.message(from, to, Arrow::CALL, label);
        if block || !creating {
            self.activation(to, true);
        }
        if block {
            let parent = self.current_call.replace(self.scopes.len());
            self.scopes.push(Scope::Call {
                from,
                to,
                parent,
                assignment,
                returned: false,
            });
        } else {
            if let Some(label) = assignment {
                self.message(to, from, Arrow::REPLY, label);
            }
            if !creating {
                self.activation(to, false);
            }
        }
        Ok(())
    }

    fn return_value(&mut self, text: &'a str) -> Result<(), String> {
        let Some(index) = self.current_call else {
            return Err("`return` outside a synchronous call".into());
        };
        let Scope::Call {
            from, to, returned, ..
        } = &mut self.scopes[index]
        else {
            unreachable!("current call points to a call scope");
        };
        *returned = true;
        let (from, to) = (*from, *to);
        self.message(to, from, Arrow::REPLY, plain_text(text)?);
        Ok(())
    }

    fn caller(&self) -> usize {
        self.current_call
            .map_or(self.starter.unwrap_or(OUTSIDE), |index| {
                match self.scopes[index] {
                    Scope::Call { to, .. } => to,
                    Scope::Frame(_) => unreachable!("current call points to a call scope"),
                }
            })
    }

    fn id(&mut self, id: &'a str) -> Result<usize, String> {
        register_participant(&mut self.diagram, &mut self.ids, id)
    }

    fn activation(&mut self, participant: usize, start: bool) {
        self.diagram
            .events
            .push(Event::Activation { participant, start });
    }

    fn emit_comments(&mut self, speaker: usize) {
        for label in self.comments.drain(..) {
            self.diagram.events.push(Event::Note {
                placement: Placement::Over(speaker),
                label,
            });
        }
    }

    fn message(&mut self, from: usize, to: usize, arrow: Arrow, label: Label<'a>) {
        self.uses_outside |= from == OUTSIDE || to == OUTSIDE;
        self.emit_comments(from);
        self.diagram.events.push(Event::Message {
            from,
            to,
            arrow,
            label,
            number: None,
        });
    }
}

fn add_caller(diagram: &mut Diagram<'_>) {
    diagram.participants.insert(
        0,
        Participant {
            label: Label::new("Caller"),
            declaration: Some(ParticipantKind::Actor),
            deferred: false,
        },
    );
    let shift = |index: &mut usize| *index = if *index == OUTSIDE { 0 } else { *index + 1 };
    for event in &mut diagram.events {
        match event {
            Event::Message { from, to, .. } => {
                shift(from);
                shift(to);
            }
            Event::Note { placement, .. } => match placement {
                Placement::Left(index) | Placement::Right(index) | Placement::Over(index) => {
                    shift(index)
                }
                Placement::Between(left, right) => {
                    shift(left);
                    shift(right);
                }
            },
            Event::Activation { participant, .. }
            | Event::Create(participant)
            | Event::Destroy(participant) => shift(participant),
            Event::Start { .. } | Event::Branch(..) | Event::End => {}
        }
    }
}

fn direct_route(line: &str) -> Option<(&str, &str, &str, bool)> {
    let (route, text) = line.split_once(':').unwrap_or((line, ""));
    for (token, dotted) in [("-->", true), ("->", false)] {
        if let Some((from, to)) = route.split_once(token) {
            let (from, to) = (from.trim(), to.trim());
            if (from.is_empty() || is_id(from)) && (to.is_empty() || is_id(to)) {
                return Some((from, to, text, dotted));
            }
        }
    }
    if line.contains(':') && is_id(route.trim()) {
        return Some(("", route.trim(), text, false));
    }
    None
}

fn split_assignment(text: &str) -> Result<Option<(Label<'_>, &str)>, String> {
    let Some((left, right)) = text.split_once('=') else {
        return Ok(None);
    };
    if left.contains('(') {
        return Ok(None);
    }
    let left = left.trim_end();
    if !left.split_whitespace().last().is_some_and(is_id) || right.trim().is_empty() {
        return Err("expected a variable and call on either side of `=`".into());
    }
    Ok(Some((plain_text(left)?, right.trim_start())))
}

fn opening(line: &str) -> (&str, bool) {
    line.strip_suffix('{')
        .map_or((line, false), |line| (line.trim_end(), true))
}

fn keyword_prefix(text: &str) -> (&str, &str) {
    text.split_once(|ch: char| ch.is_whitespace() || ch == '(')
        .map_or((text, ""), |(prefix, _)| {
            (prefix, text[prefix.len()..].trim_start())
        })
}

fn is_branch(line: &str) -> bool {
    matches!(keyword_prefix(line).0, "else" | "catch" | "finally")
}

fn invocation(text: &str) -> Result<(&str, Option<&str>), String> {
    if let Some(index) = text.find('(') {
        let path = text[..index].trim_end();
        let args = &text[index..];
        condition_text(args)?;
        Ok((path, Some(args)))
    } else {
        Ok((text, None))
    }
}

fn condition_text(text: &str) -> Result<&str, String> {
    let inner = text
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(')'))
        .ok_or("expected balanced parentheses")?;
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for ch in inner.chars() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == delimiter {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => depth += 1,
            ')' if depth == 0 => return Err("expected balanced parentheses".into()),
            ')' => depth -= 1,
            _ => {}
        }
    }
    if depth != 0 || quote.is_some() {
        return Err("expected balanced parentheses and quoted text".into());
    }
    Ok(inner.trim())
}
