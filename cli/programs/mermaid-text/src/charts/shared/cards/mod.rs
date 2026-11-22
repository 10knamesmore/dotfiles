use super::Direction;
use crate::{MAX_SOURCE_BYTES, Options, color::Style, graphics::Scene};

mod card;
mod draw;
pub mod parse;
mod wire;

pub fn render<G: parse::Grammar>(source: &str, options: &Options) -> Result<String, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("diagram exceeds 256 KiB".into());
    }
    draw::render(&parse::parse::<G>(source)?, options)
}

pub fn image<G: parse::Grammar>(
    source: &str,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("diagram exceeds 256 KiB".into());
    }
    draw::image::image(&parse::parse::<G>(source)?, options, scene)
}

#[derive(Default)]
struct Diagram<'a> {
    nodes: Vec<Node<'a>>,
    relationships: Vec<Relationship<'a>>,
    notes: Vec<&'a str>,
    namespaces: Vec<Namespace<'a>>,
    direction: Option<Direction>,
    title: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub struct Name<'a> {
    pub id: &'a str,
    written: &'a str,
}

struct Namespace<'a> {
    name: &'a str,
    label: Option<&'a str>,
    parent: Option<usize>,
}

pub struct Node<'a> {
    name: Name<'a>,
    namespace: Option<usize>,
    alias: Option<&'a str>,
    pub generic: Option<&'a str>,
    pub annotations: Vec<&'a str>,
    pub fields: Vec<&'a str>,
    pub methods: Vec<&'a str>,
    pub notes: Vec<&'a str>,
    style: CardStyle,
}

#[derive(Clone, Copy, Default)]
struct CardStyle {
    heavy: Option<bool>,
    dashed: Option<bool>,
    rounded: Option<bool>,
    color: Style,
}

impl CardStyle {
    fn apply(&mut self, next: Self) {
        self.apply_frame(next);
        self.color = self.color.overlay(next.color);
    }

    fn apply_frame(&mut self, next: Self) {
        self.heavy = next.heavy.or(self.heavy);
        self.dashed = next.dashed.or(self.dashed);
        self.rounded = next.rounded.or(self.rounded);
    }
}

struct Relationship<'a> {
    left: usize,
    right: usize,
    ends: [End<'a>; 2],
    link: Link,
    label: Option<&'a str>,
    directional_label: bool,
}

#[derive(Clone, Copy)]
pub struct End<'a> {
    pub mark: Mark,
    pub count: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub enum Mark {
    None,
    Triangle,
    Arrow,
    Diamond,
    HollowDiamond,
    Circle,
}

#[derive(Clone, Copy)]
pub enum Link {
    Solid,
    Dashed,
}
