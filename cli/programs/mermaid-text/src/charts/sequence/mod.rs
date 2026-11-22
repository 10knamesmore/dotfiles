use crate::Options;
use crate::graphics::Scene;

mod mermaid;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(render_image);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        mermaid::parse(source)?.render_text(options)
    }
}

fn render_image(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    mermaid::parse(source)?.render_image(options, scene)
}
