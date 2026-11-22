use std::borrow::Cow;

use crate::Options;
use crate::charts::shared::Direction;
use crate::color::Style;
use crate::graphics::Scene;

mod draw;
mod image;
mod parse;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(render_image);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let diagram = parse::parse(source)?;
        draw::draw(&diagram, options).map_err(|error| format!("state diagram: {error}"))
    }
}

fn render_image(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let diagram = parse::parse(source)?;
    image::render(&diagram, options, scene).map_err(|error| format!("state diagram: {error}"))
}

fn expand_tabs(text: &str) -> Cow<'_, str> {
    if text.contains('\t') {
        Cow::Owned(text.replace('\t', "    "))
    } else {
        Cow::Borrowed(text)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum StateId<'a> {
    Named(&'a str),
    Start(usize),
    End(usize),
}

#[derive(Clone, Copy, Default)]
enum Kind {
    #[default]
    Ordinary,
    Composite(usize),
    Choice,
    Fork,
    Join,
    FloatingNote,
}

struct State<'a> {
    id: StateId<'a>,
    region: usize,
    kind: Kind,
    descriptions: Vec<&'a str>,
    notes: Vec<Note<'a>>,
    style: Style,
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

struct Note<'a> {
    side: Side,
    lines: Vec<&'a str>,
}

struct Edge<'a> {
    from: usize,
    to: usize,
    label: Option<&'a str>,
}

struct Region<'a> {
    direction: Direction,
    states: Vec<usize>,
    edges: Vec<Edge<'a>>,
}

struct Scope {
    owner: Option<usize>,
    regions: Vec<usize>,
}

struct Diagram<'a> {
    scopes: Vec<Scope>,
    regions: Vec<Region<'a>>,
    states: Vec<State<'a>>,
    cross_region: Vec<Edge<'a>>,
}
