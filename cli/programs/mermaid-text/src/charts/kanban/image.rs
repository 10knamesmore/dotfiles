use super::{Card, board, lane_style, parse, theme_color};
use crate::{Options, color::Style, graphics::Scene};
use board::image::{Label, PAD, text_on_fill, tint};

const GUTTER: f32 = 14.0;
const INSET: f32 = 12.0;

struct Tile {
    title: Label,
    details: Vec<Label>,
    width: f32,
    height: f32,
}

impl Tile {
    fn new(scene: &Scene, card: &Card) -> Self {
        let title = Label::new(scene, &card.label);
        let details: Vec<_> = card.details.iter().map(|s| Label::new(scene, s)).collect();
        let width = details.iter().map(|l| l.width).fold(title.width, f32::max) + 2.0 * INSET;
        let mut height = title.height + 2.0 * INSET;
        if !details.is_empty() {
            height += 6.0 + details.iter().map(|l| l.height).sum::<f32>();
        }
        Self {
            title,
            details,
            width,
            height,
        }
    }

    fn draw(&self, scene: &mut Scene, x: f32, y: f32, width: f32, height: f32, style: Style) {
        scene.rect(x, y, width, height, 8.0, style.fill, style.stroke);
        let x = x + INSET;
        let mut y = y + INSET;
        self.title.draw(scene, x, y, style.text);
        y += self.title.height + 6.0;
        for detail in &self.details {
            detail.draw(scene, x, y, style.text);
            y += detail.height;
        }
    }
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let lanes = parse(board::body(source, &["kanban"])?)?;
    let measured: Vec<_> = lanes
        .iter()
        .map(|lane| {
            let heading = Tile::new(scene, &lane.heading);
            let cards: Vec<_> = lane
                .cards
                .iter()
                .map(|card| Tile::new(scene, card))
                .collect();
            let width = cards
                .iter()
                .map(|card| card.width + 2.0 * INSET)
                .fold(heading.width.max(152.0), f32::max);
            (width, heading, cards)
        })
        .collect();
    let width = measured.iter().map(|(w, _, _)| *w + GUTTER).sum::<f32>() - GUTTER;
    let heading_height = measured
        .iter()
        .map(|(_, h, _)| h.height)
        .fold(0.0, f32::max);
    let card_height = measured
        .iter()
        .map(|(_, _, cards)| cards.iter().map(|card| card.height + INSET).sum::<f32>())
        .fold(0.0, f32::max);
    let height = heading_height + card_height + INSET;
    scene.resize(width + 2.0 * PAD, height + 2.0 * PAD)?;

    let ink = scene.ink();
    let card = Style {
        fill: Some(tint(ink, 16)),
        stroke: theme_color(options, "nodeBorder").or(Some(tint(ink, 84))),
        text: None,
    };
    let mut x = PAD;
    for (i, (width, heading, cards)) in measured.into_iter().enumerate() {
        let style = lane_style(options, i);
        let outline = style.stroke.or(Some(tint(ink, 56)));
        scene.rect(x, PAD, width, height, 8.0, Some(tint(ink, 7)), outline);
        heading.draw(
            scene,
            x,
            PAD,
            width,
            heading_height,
            Style {
                fill: style.fill.or(Some(tint(ink, 23))),
                stroke: None,
                text: style
                    .text
                    .or_else(|| text_on_fill(scene, options, style.fill)),
            },
        );
        let mut y = PAD + heading_height + INSET;
        for tile in cards {
            tile.draw(scene, x + INSET, y, width - 2.0 * INSET, tile.height, card);
            y += tile.height + INSET;
        }
        x += width + GUTTER;
    }
    Ok(())
}
