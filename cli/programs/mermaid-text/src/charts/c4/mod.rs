use super::shared::architecture_layout::Palette;
use crate::Options;

mod parse;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(|source, options, scene| {
        parse::parse(source)?.image(options, &Palette::new(options), scene)
    });

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        parse::parse(source)?.render(options, &Palette::new(options))
    }
}
