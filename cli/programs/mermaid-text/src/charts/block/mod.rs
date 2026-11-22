use super::shared::block_layout::{self, DiagramColors};
use crate::Options;

mod parse;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(|source, options, scene| {
        let (diagram, grid) = parse::parse(block_layout::body(source))?;
        block_layout::image(&diagram, &grid, options, DiagramColors::new(options), scene)
    });

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (diagram, grid) = parse::parse(block_layout::body(source))?;
        block_layout::render(diagram, grid, options, DiagramColors::new(options))
    }
}
