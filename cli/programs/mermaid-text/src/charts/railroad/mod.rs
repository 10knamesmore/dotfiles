mod image;
mod layout;
mod parser;

use crate::Options;
use crate::color::{Paint, Style};
use crate::graphics::Scene;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Dialect {
    Ir,
    Abnf,
    Ebnf,
    Peg,
}

struct Grammar<'a> {
    title: Option<String>,
    rules: Vec<(&'a str, Expr)>,
}

enum Expr {
    Atom(String, Atom),
    Sequence(Vec<Self>),
    Choice(Vec<Self>, bool),
    Repeat(Box<Self>, usize, Option<usize>),
    Scope(Box<Self>, &'static str, Option<Box<Self>>),
}

impl Expr {
    fn repeat(self, min: usize, max: Option<usize>) -> Self {
        Self::Repeat(Box::new(self), min, max)
    }

    fn scope(self, caption: &'static str, exclusion: Option<Self>) -> Self {
        Self::Scope(Box::new(self), caption, exclusion.map(Box::new))
    }
}

#[derive(Clone, Copy)]
enum Atom {
    Terminal,
    Numeric,
    Nonterminal,
    Special,
    Any,
}

struct Colors {
    terminal: Style,
    nonterminal: Style,
    special: Style,
    line: Paint,
    rule_name: Paint,
}

impl Colors {
    fn new(options: &Options) -> Self {
        let color = |key| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let node = options.theme_style();
        let line = options.theme_line();
        Self {
            terminal: Style {
                fill: color("secondBkg").or(color("secondaryColor")),
                stroke: color("secondaryBorderColor").or(line),
                text: color("secondaryTextColor"),
            },
            nonterminal: Style {
                fill: color("mainBkg").or(node.fill),
                stroke: node.stroke.or(line),
                text: node.text,
            },
            special: Style {
                fill: color("tertiaryColor").or(color("secondaryColor")),
                stroke: color("tertiaryBorderColor").or(color("secondaryBorderColor")),
                text: node.text,
            },
            line: Paint { fg: line, bg: None },
            rule_name: Paint {
                fg: color("titleColor"),
                bg: None,
            },
        }
    }

    fn atom(&self, kind: Atom) -> Style {
        match kind {
            Atom::Terminal | Atom::Numeric => self.terminal,
            Atom::Nonterminal => self.nonterminal,
            Atom::Special | Atom::Any => self.special,
        }
    }
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(render_image);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        layout::render(parse(source)?, options)
    }
}

fn render_image(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    image::render(parse(source)?, options, scene)
}

fn parse(source: &str) -> Result<Grammar<'_>, String> {
    let (header, body) = source.split_once('\n').unwrap_or((source, ""));
    let dialect = match header.split("%%").next().unwrap().trim() {
        "railroad-beta" => Dialect::Ir,
        "railroad-abnf-beta" => Dialect::Abnf,
        "railroad-ebnf-beta" => Dialect::Ebnf,
        "railroad-peg-beta" => Dialect::Peg,
        other => return Err(format!("invalid railroad diagram header: {other}")),
    };
    parser::Parser::new(body, dialect).diagram()
}
