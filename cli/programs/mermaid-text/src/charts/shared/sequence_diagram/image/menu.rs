use std::borrow::Cow;

use crate::color::Style;
use crate::graphics::Scene;

use super::{Diagram, HELPER_GAP, LINE_HEIGHT, MARGIN, Text, stroke};

pub(super) struct Footer {
    cards: Vec<(Text<'static>, Text<'static>)>,
    width: f32,
}

impl Footer {
    pub(super) fn new(diagram: &Diagram<'_>, scene: &Scene) -> Self {
        let mut width: f32 = 0.0;
        let cards = diagram
            .menus
            .iter()
            .filter_map(|(&id, menu)| menu.text(&diagram.participants[id].label))
            .map(|menu| {
                let heading = Text::measure(scene, std::iter::once(Cow::Owned(menu.heading)));
                let entries = Text::measure(scene, menu.entries.into_iter().map(Cow::Owned));
                width = width.max(heading.width.max(entries.width) + 2.0 * HELPER_GAP);
                (heading, entries)
            })
            .collect();
        Self { cards, width }
    }

    pub(super) fn canvas_width(&self) -> f32 {
        self.width + 2.0 * MARGIN
    }

    pub(super) fn height(&self) -> f32 {
        self.cards
            .iter()
            .map(|(_, entries)| entries.height() + LINE_HEIGHT + 5.0 * HELPER_GAP)
            .sum()
    }

    pub(super) fn draw(self, scene: &mut Scene, mut top: f32, radius: f32, actor: Style) {
        let border = actor.stroke.unwrap_or(scene.ink());
        let heading_height = LINE_HEIGHT + 2.0 * HELPER_GAP;
        for (heading, entries) in self.cards {
            top += HELPER_GAP;
            let height = heading_height + entries.height() + 2.0 * HELPER_GAP;
            let divider = top + heading_height;
            scene.rect(
                MARGIN,
                top,
                self.width,
                height,
                radius,
                actor.fill.or(Some(scene.paper())),
                Some(border),
            );
            stroke(
                scene,
                &[(MARGIN, divider), (MARGIN + self.width, divider)],
                false,
                border,
            );
            let left = MARGIN + HELPER_GAP;
            heading.draw(scene, left, top + HELPER_GAP, false, actor.text);
            entries.draw(scene, left, divider + HELPER_GAP, false, actor.text);
            top += height;
        }
    }
}
