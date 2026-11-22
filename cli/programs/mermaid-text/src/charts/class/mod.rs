use super::shared::cards;
use crate::Options;

mod parse;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(cards::image::<Self>);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        cards::render::<Self>(source, options)
    }
}
