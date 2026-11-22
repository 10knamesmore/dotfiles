use super::shared::lane::{
    self, Family, Syntax,
    parse::{Parser, id_char},
};
use crate::Options;

pub(super) struct Renderer;

const SYNTAX: Syntax = Syntax {
    family: Family::Swimlane,
    headers: &["swimlane-beta"],
    statement,
};

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> =
        Some(|source, options, scene| lane::render_image(source, options, scene, SYNTAX));

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        lane::render(source, options, SYNTAX)
    }
}

fn statement(parser: &mut Parser, word: &str, rest: &str) -> Result<bool, String> {
    if word != "subgraph" {
        return Ok(false);
    }
    let label = if rest.starts_with('"') && rest.ends_with('"') && rest.len() > 1 {
        Some(&rest[1..rest.len() - 1])
    } else if rest.contains(char::is_whitespace)
        && rest.chars().all(|c| id_char(c) || c.is_whitespace())
    {
        Some(rest)
    } else {
        None
    };
    parser.open_group(rest, label)?;
    Ok(true)
}
