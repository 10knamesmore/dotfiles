use super::super::{Participant, ParticipantKind};
use crate::color::Style;
use crate::graphics::Scene;

use super::{STROKE, Text, circle};

const ACTOR_HALF_WIDTH: f32 = 14.0;
const ACTOR_ARM_Y: f32 = 22.0;
const ICON_RADIUS: f32 = 15.0;
const ICON_CENTER_Y: f32 = 25.0;
const ICON_LABEL_Y: f32 = 46.0;
const BOUNDARY_BAR: f32 = 35.0;
const DATABASE_HALF_WIDTH: f32 = 21.0;
const DATABASE_TOP: f32 = 14.0;
const DATABASE_BOTTOM: f32 = 38.0;
const DATABASE_RIM: f32 = 6.0;
const COLLECTION_OFFSET: f32 = 6.0;
const QUEUE_RIM: f32 = 8.0;
const BOX_LABEL_Y: f32 = 10.0;

pub(super) struct Header<'a> {
    kind: ParticipantKind,
    text: Text<'a>,
    pub width: f32,
    pub height: f32,
}

impl<'a> Header<'a> {
    pub(super) fn new(scene: &Scene, participant: &'a Participant<'_>) -> Self {
        use ParticipantKind::*;
        let kind = participant.declaration.unwrap_or(Participant);
        let text = Text::label(scene, &participant.label);
        let box_padding = 2.0 * BOX_LABEL_Y;
        let (horizontal, vertical) = match kind {
            Participant => (28.0, box_padding),
            Collections => (28.0 + COLLECTION_OFFSET, box_padding + COLLECTION_OFFSET),
            Queue => (4.0 * QUEUE_RIM + 8.0, box_padding),
            Actor | Boundary | Control | Entity | Database => (28.0, ICON_LABEL_Y + 6.0),
        };
        Self {
            kind,
            width: (text.width + horizontal).max(76.0),
            height: text.height() + vertical,
            text,
        }
    }

    pub(super) fn creation_port(&self, from_left: bool) -> (f32, f32) {
        use ParticipantKind::*;
        let (reach, y) = match self.kind {
            Participant | Collections | Queue => (self.width / 2.0, self.height / 2.0),
            Actor => (ACTOR_HALF_WIDTH, ACTOR_ARM_Y),
            Boundary if from_left => (BOUNDARY_BAR, ICON_CENTER_Y),
            Boundary | Control | Entity => (ICON_RADIUS, ICON_CENTER_Y),
            Database => (DATABASE_HALF_WIDTH, (DATABASE_TOP + DATABASE_BOTTOM) / 2.0),
        };
        (if from_left { -reach } else { reach }, y)
    }

    pub(super) fn draw(&self, scene: &mut Scene, x: f32, y: f32, radius: f32, style: Style) {
        use ParticipantKind::*;
        let ink = style.stroke.unwrap_or(scene.ink());
        let fill = style.fill.or(Some(scene.paper()));
        let left = x - self.width / 2.0;
        let right = x + self.width / 2.0;
        let icon_y = y + ICON_CENTER_Y;
        let (label_x, label_y) = match self.kind {
            Participant => {
                scene.rect(left, y, self.width, self.height, radius, fill, Some(ink));
                (x, BOX_LABEL_Y)
            }
            Actor => {
                circle(scene, x, y + 12.0, 6.0, fill, ink);
                let arms = y + ACTOR_ARM_Y;
                let d = format!(
                    "M {x} {} V {} M {} {arms} H {} M {} {} L {x} {} L {} {}",
                    y + 18.0,
                    y + 33.0,
                    x - ACTOR_HALF_WIDTH,
                    x + ACTOR_HALF_WIDTH,
                    x - 11.0,
                    y + 43.0,
                    y + 33.0,
                    x + 11.0,
                    y + 43.0,
                );
                scene.path(&d, None, Some(ink), STROKE);
                (x, ICON_LABEL_Y)
            }
            Boundary => {
                circle(scene, x, icon_y, ICON_RADIUS, fill, ink);
                let bar = x - BOUNDARY_BAR;
                let d = format!(
                    "M {bar} {} V {} M {bar} {icon_y} H {}",
                    icon_y - 9.0,
                    icon_y + 9.0,
                    x - ICON_RADIUS,
                );
                scene.path(&d, None, Some(ink), STROKE);
                (x, ICON_LABEL_Y)
            }
            Control => {
                circle(scene, x, icon_y, ICON_RADIUS, fill, ink);
                let crown = icon_y - ICON_RADIUS;
                let d = format!(
                    "M {} {crown} L {} {} V {} Z",
                    x - 4.0,
                    x + 4.0,
                    crown - 5.0,
                    crown + 5.0,
                );
                scene.path(&d, Some(ink), None, 0.0);
                (x, ICON_LABEL_Y)
            }
            Entity => {
                circle(scene, x, icon_y, ICON_RADIUS, fill, ink);
                let base = icon_y + ICON_RADIUS;
                let d = format!("M {} {base} H {}", x - ICON_RADIUS, x + ICON_RADIUS);
                scene.path(&d, None, Some(ink), STROKE);
                (x, ICON_LABEL_Y)
            }
            Database => {
                let (rx, ry) = (DATABASE_HALF_WIDTH, DATABASE_RIM);
                let (left, right) = (x - rx, x + rx);
                let (top, bottom) = (y + DATABASE_TOP, y + DATABASE_BOTTOM);
                let d = format!(
                    "M {left} {top} A {rx} {ry} 0 0 1 {right} {top} V {bottom} \
                     A {rx} {ry} 0 0 1 {left} {bottom} Z"
                );
                scene.path(&d, fill, Some(ink), STROKE);
                let rim = format!("M {left} {top} A {rx} {ry} 0 0 0 {right} {top}");
                scene.path(&rim, None, Some(ink), STROKE);
                (x, ICON_LABEL_Y)
            }
            Collections => {
                for (left, top) in [(left + COLLECTION_OFFSET, y), (left, y + COLLECTION_OFFSET)] {
                    scene.rect(
                        left,
                        top,
                        self.width - COLLECTION_OFFSET,
                        self.height - COLLECTION_OFFSET,
                        radius,
                        fill,
                        Some(ink),
                    );
                }
                (x - COLLECTION_OFFSET / 2.0, BOX_LABEL_Y + COLLECTION_OFFSET)
            }
            Queue => {
                let (rx, ry) = (QUEUE_RIM, self.height / 2.0);
                let (left, right) = (left + rx, right - rx);
                let bottom = y + self.height;
                let d = format!(
                    "M {left} {y} A {rx} {ry} 0 0 0 {left} {bottom} H {right} \
                     A {rx} {ry} 0 0 0 {right} {y} Z"
                );
                scene.path(&d, fill, Some(ink), STROKE);
                let rim = format!("M {right} {y} A {rx} {ry} 0 0 0 {right} {bottom}");
                scene.path(&rim, None, Some(ink), STROKE);
                (x, BOX_LABEL_Y)
            }
        };
        self.text
            .draw(scene, label_x, y + label_y, true, style.text);
    }
}
