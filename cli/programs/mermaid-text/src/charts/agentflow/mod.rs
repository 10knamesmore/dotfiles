use super::shared::lane::{self, Family, Syntax, parse::Parser};
use crate::Options;

pub(super) struct Renderer;

const SYNTAX: Syntax = Syntax {
    family: Family::Agentflow,
    headers: &["agentflow", "agentflow-beta"],
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
    match word {
        "global" if rest.is_empty() => parser.open_global(),
        "flow" => parser.open_group(rest, None)?,
        "connector" => parser.connector(rest)?,
        _ => return Ok(false),
    }
    Ok(true)
}
