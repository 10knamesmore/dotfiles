use std::collections::BTreeMap;

use crate::Options;
use crate::color::{Rgb, Style};
use crate::graphics::Scene;

mod draw;
mod image;
pub mod label;
use label::Label;

#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParticipantKind {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Database,
    Collections,
    Queue,
}

impl ParticipantKind {
    fn stereotype(self) -> &'static str {
        match self {
            Self::Participant | Self::Actor => "",
            Self::Boundary => "[boundary]",
            Self::Control => "[control]",
            Self::Entity => "[entity]",
            Self::Database => "[database]",
            Self::Collections => "[collections]",
            Self::Queue => "[queue]",
        }
    }
}

pub struct Participant<'a> {
    pub label: Label<'a>,
    pub declaration: Option<ParticipantKind>,
    pub deferred: bool,
}

#[derive(Clone, Copy)]
pub struct Arrow {
    pub dotted: bool,
    pub head: ArrowHead,
    pub central_source: bool,
    pub central_target: bool,
}

impl Arrow {
    pub const CALL: Self = Self {
        dotted: false,
        head: ArrowHead::Filled,
        central_source: false,
        central_target: false,
    };
    pub const ASYNC: Self = Self {
        dotted: false,
        head: ArrowHead::Open,
        ..Self::CALL
    };
    pub const REPLY: Self = Self {
        dotted: true,
        ..Self::CALL
    };
}

#[derive(Clone, Copy)]
pub enum ArrowHead {
    None,
    Filled,
    Open,
    Cross,
    Both,
}

#[derive(Clone, Copy)]
pub enum Placement {
    Left(usize),
    Right(usize),
    Over(usize),
    Between(usize, usize),
}

pub enum Event<'a> {
    Message {
        from: usize,
        to: usize,
        arrow: Arrow,
        label: Label<'a>,
        number: Option<String>,
    },
    Note {
        placement: Placement,
        label: Label<'a>,
    },
    Start {
        keyword: &'static str,
        label: Label<'a>,
        fill: Option<Rgb>,
    },
    Branch(&'static str, Label<'a>),
    End,
    Activation {
        participant: usize,
        start: bool,
    },
    Create(usize),
    Destroy(usize),
}

#[derive(Default)]
pub struct Diagram<'a> {
    pub participants: Vec<Participant<'a>>,
    pub groups: Vec<Group<'a>>,
    pub menus: BTreeMap<usize, ParticipantMenu<'a>>,
    pub events: Vec<Event<'a>>,
    pub max_depth: usize,
    pub title: Option<Label<'a>>,
}

impl Diagram<'_> {
    pub fn render_image(&self, options: &Options, scene: &mut Scene) -> Result<(), String> {
        image::render(self, options, scene)
    }

    pub fn render_text(&self, options: &Options) -> Result<String, String> {
        if self.participants.is_empty() {
            return Err("sequence diagram has no participants".into());
        }
        draw::render(self, options)
    }
}

#[derive(Default)]
pub struct ParticipantMenu<'a> {
    pub identifier: &'a str,
    pub links: BTreeMap<String, String>,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub details: Vec<&'a str>,
}

struct MenuText {
    heading: String,
    entries: Vec<String>,
}

impl ParticipantMenu<'_> {
    fn text(&self, label: &Label<'_>) -> Option<MenuText> {
        if self.links.is_empty() && self.properties.is_empty() && self.details.is_empty() {
            return None;
        }
        let mut heading = menu_text(&label.text.replace('\n', " "));
        if label.text.as_ref() != self.identifier {
            heading.push_str(" (");
            heading.push_str(self.identifier);
            heading.push(')');
        }
        let mut entries = Vec::new();
        for (name, address) in &self.links {
            entries.push(format!("Link {}: {}", menu_text(name), menu_text(address)));
        }
        for (name, value) in &self.properties {
            let value = value.as_str().map_or_else(|| value.to_string(), menu_text);
            entries.push(format!("Property {}: {value}", menu_text(name)));
        }
        for details in &self.details {
            entries.push(format!("Details: {}", menu_text(details)));
        }
        Some(MenuText { heading, entries })
    }
}

fn menu_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() {
            output.extend(ch.escape_debug());
        } else {
            output.push(ch);
        }
    }
    output
}

pub struct Group<'a> {
    pub first: usize,
    pub last: usize,
    pub label: Label<'a>,
    pub fill: Option<Rgb>,
}

#[derive(Default)]
pub struct Theme {
    pub actor: Style,
    pub note: Style,
    pub signal: Option<Rgb>,
    pub message_text: Option<Rgb>,
}

impl Theme {
    pub fn new(options: &Options) -> Self {
        if !options.color {
            return Self::default();
        }
        let color = |name| options.config.theme_color(name);
        let base = options.theme_style();
        Self {
            actor: base.overlay(Style {
                fill: color("actorBkg"),
                stroke: color("actorBorder"),
                text: color("actorTextColor"),
            }),
            note: base.overlay(Style {
                fill: color("noteBkgColor"),
                stroke: color("noteBorderColor"),
                text: color("noteTextColor"),
            }),
            signal: color("signalColor").or_else(|| options.theme_line()),
            message_text: color("signalTextColor").or(base.text),
        }
    }
}

pub fn declare_participant<'a>(
    diagram: &mut Diagram<'a>,
    ids: &mut BTreeMap<&'a str, usize>,
    declaration: &'a str,
    kind: ParticipantKind,
    deferred: bool,
) -> Result<usize, String> {
    let (id, alias) = first_word(declaration);
    let label = if alias.is_empty() {
        Label::new(id)
    } else {
        let (keyword, text) = first_word(alias);
        if keyword != "as" || text.is_empty() {
            return Err("expected an ID optionally followed by `as LABEL`".into());
        }
        plain_text(text)?
    };
    define_participant(diagram, ids, id, label, kind, deferred)
}

pub fn first_word(text: &str) -> (&str, &str) {
    let (word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    (word, rest.trim_start())
}

pub fn define_participant<'a>(
    diagram: &mut Diagram<'a>,
    ids: &mut BTreeMap<&'a str, usize>,
    id: &'a str,
    label: Label<'a>,
    kind: ParticipantKind,
    deferred: bool,
) -> Result<usize, String> {
    if deferred && ids.contains_key(id) {
        return Err(format!("cannot create existing participant `{id}`"));
    }
    let index = register_participant(diagram, ids, id)?;
    let participant = &mut diagram.participants[index];
    if let Some(previous_kind) = participant.declaration
        && (previous_kind != kind || participant.label.text != label.text)
    {
        return Err(format!("conflicting declarations for participant `{id}`"));
    }
    participant.label = label;
    participant.declaration = Some(kind);
    participant.deferred |= deferred;
    Ok(index)
}

pub fn register_participant<'a>(
    diagram: &mut Diagram<'a>,
    ids: &mut BTreeMap<&'a str, usize>,
    id: &'a str,
) -> Result<usize, String> {
    if !is_id(id) {
        return Err(format!("unsupported participant ID `{id}`"));
    }
    if let Some(&index) = ids.get(id) {
        return Ok(index);
    }
    let index = diagram.participants.len();
    ids.insert(id, index);
    diagram.participants.push(Participant {
        label: Label::new(id),
        declaration: None,
        deferred: false,
    });
    Ok(index)
}

pub fn is_id(id: &str) -> bool {
    let id = id
        .strip_prefix('(')
        .and_then(|id| id.strip_suffix(')'))
        .unwrap_or(id);
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '=' | '.'))
        })
}

pub fn plain_text(text: &str) -> Result<Label<'_>, String> {
    Label::parse(text)
}
