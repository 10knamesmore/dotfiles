use super::shared::block_layout::{self, DiagramColors};
use crate::{
    Options,
    color::{Paint, Style},
};

mod parse;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(|source, options, scene| {
        let (diagram, grid) = parse::parse(block_layout::body(source))?;
        block_layout::image(&diagram, &grid, options, colors(options), scene)
    });

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (diagram, grid) = parse::parse(block_layout::body(source))?;
        block_layout::render(diagram, grid, options, colors(options))
    }
}

fn colors(options: &Options) -> Option<DiagramColors> {
    DiagramColors::new(options).map(|mut colors| {
        let color = |name| options.config.theme_color(name);
        colors.nodes = colors.nodes.overlay(Style {
            fill: color("requirementBackground"),
            stroke: color("requirementBorderColor"),
            text: color("requirementTextColor"),
        });
        colors.line.fg = color("relationColor").or(colors.line.fg);
        colors.label = Paint {
            fg: color("relationLabelColor").or(colors.nodes.text),
            bg: color("relationLabelBackground"),
        };
        colors
    })
}
