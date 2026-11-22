use crate::Options;
use crate::graphics::Scene;

mod parse;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(render_image);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        parse::parse(source)?.render_text(options)
    }
}

fn render_image(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    parse::parse(source)?.render_image(options, scene)
}
