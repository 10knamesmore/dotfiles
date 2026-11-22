use super::{Direction, Palette, Period, label_parts, parse};
use crate::{
    Options,
    charts::shared::history::{IMAGE_PAD, ImageLabel},
    color::{Rgb, Style},
    graphics::Scene,
};

const GAP: f32 = 28.0;
const SECTION_GAP: f32 = 44.0;
const CARD_PAD: f32 = 10.0;
const BRANCH: f32 = 20.0;
const EVENT_GAP: f32 = 8.0;
const LANE: f32 = 34.0;
const ACCENTS: [Rgb; 6] = [
    Rgb::from_rgba(91, 164, 231, 255),
    Rgb::from_rgba(231, 160, 83, 255),
    Rgb::from_rgba(91, 183, 158, 255),
    Rgb::from_rgba(204, 143, 197, 255),
    Rgb::from_rgba(190, 173, 89, 255),
    Rgb::from_rgba(149, 164, 226, 255),
];

fn label(scene: &Scene, text: &str) -> ImageLabel {
    ImageLabel::wrapped(scene, &label_parts(text).collect::<Vec<_>>().join("\n"), 32)
}

struct Card {
    period: ImageLabel,
    events: Vec<ImageLabel>,
    style: Style,
    width: f32,
    height: f32,
}

impl Card {
    fn new(scene: &Scene, period: &Period<'_>, style: Style) -> Self {
        let label = label(scene, period.label);
        let events: Vec<_> = period
            .events
            .iter()
            .map(|event| self::label(scene, event))
            .collect();
        let width = events
            .iter()
            .map(|label| label.width + BRANCH)
            .fold(label.width.max(96.0), f32::max)
            + 2.0 * CARD_PAD;
        let height = label.height()
            + 2.0 * CARD_PAD
            + events
                .iter()
                .map(|label| label.height() + 2.0 * CARD_PAD + EVENT_GAP)
                .sum::<f32>();
        Self {
            period: label,
            events,
            style,
            width,
            height,
        }
    }

    fn draw(&self, scene: &mut Scene, x: f32, y: f32) {
        let color = self.style.stroke;
        let title_height = self.period.height() + 2.0 * CARD_PAD;
        scene.rect(x, y, self.width, title_height, 5.0, self.style.fill, color);
        self.period
            .draw(scene, x + CARD_PAD, y + CARD_PAD, self.style.text);

        let mut event_y = y + title_height + EVENT_GAP;
        let spine = x + BRANCH / 2.0;
        let mut previous_y = y + title_height;
        for event in &self.events {
            let height = event.height() + 2.0 * CARD_PAD;
            let center_y = event_y + height / 2.0;
            scene.path(
                &format!("M {spine} {previous_y} V {center_y} H {}", x + BRANCH),
                None,
                color,
                1.3,
            );
            scene.rect(
                x + BRANCH,
                event_y,
                self.width - BRANCH,
                height,
                5.0,
                self.style.fill,
                color,
            );
            event.draw(
                scene,
                x + BRANCH + CARD_PAD,
                event_y + CARD_PAD,
                self.style.text,
            );
            previous_y = center_y;
            event_y += height + EVENT_GAP;
        }
    }
}

struct Group {
    heading: Option<ImageLabel>,
    cards: Vec<Card>,
    accent: Option<Rgb>,
}

impl Group {
    fn heading_height(&self) -> f32 {
        self.heading
            .as_ref()
            .map_or(0.0, |label| label.height() + 12.0)
    }

    fn size(&self, horizontal: bool) -> (f32, f32) {
        let heading_width = self
            .heading
            .as_ref()
            .map_or(0.0, |label| label.width + 16.0);
        if horizontal {
            let width = self.cards.iter().map(|card| card.width + GAP).sum::<f32>() - GAP;
            let height = self
                .cards
                .iter()
                .map(|card| card.height)
                .fold(0.0, f32::max);
            (heading_width.max(width), height)
        } else {
            let width = self.cards.iter().map(|card| card.width).fold(0.0, f32::max);
            let height = self.cards.iter().map(|card| card.height + GAP).sum::<f32>();
            (
                heading_width.max(width) + LANE,
                self.heading_height() + height,
            )
        }
    }

    fn draw_heading(&self, scene: &mut Scene, x: f32, y: f32) {
        if let Some(heading) = &self.heading {
            scene.rect(
                x,
                y + 2.0,
                4.0,
                heading.height() - 4.0,
                2.0,
                self.accent,
                None,
            );
            heading.draw(scene, x + 14.0, y, None);
        }
    }
}

fn draw_groups(scene: &mut Scene, groups: &[Group], top: f32, size: (f32, f32), horizontal: bool) {
    let (width, height) = size;
    let has_periods = groups.iter().any(|group| !group.cards.is_empty());
    let ink = Some(scene.ink());
    if horizontal {
        let heading_height = groups.iter().map(Group::heading_height).fold(0.0, f32::max);
        let axis = top + heading_height + 9.0;
        if has_periods {
            let right = IMAGE_PAD + width;
            scene.path(
                &format!("M {IMAGE_PAD} {axis} H {right} m -8 -4 l 8 4 -8 4"),
                None,
                ink,
                1.5,
            );
        }
        let mut left = IMAGE_PAD;
        for group in groups {
            group.draw_heading(scene, left, top);
            let mut x = left;
            for card in &group.cards {
                let cx = x + card.width / 2.0;
                let y = axis + LANE - 9.0;
                scene.path(
                    &format!("M {cx} {axis} V {y}"),
                    None,
                    card.style.stroke,
                    1.6,
                );
                scene.rect(
                    cx - 5.0,
                    axis - 5.0,
                    10.0,
                    10.0,
                    5.0,
                    card.style.stroke,
                    None,
                );
                card.draw(scene, x, y);
                x += card.width + GAP;
            }
            left += group.size(true).0 + SECTION_GAP;
        }
    } else {
        let axis = IMAGE_PAD + 9.0;
        let left = IMAGE_PAD + LANE;
        let mut y = top;
        if has_periods {
            let end = top + height;
            scene.path(
                &format!("M {axis} {top} V {end} m -4 -8 l 4 8 4 -8"),
                None,
                ink,
                1.5,
            );
        }
        for group in groups {
            group.draw_heading(scene, left, y);
            y += group.heading_height();
            for card in &group.cards {
                let cy = y + CARD_PAD + card.period.height() / 2.0;
                scene.path(
                    &format!("M {axis} {cy} H {left}"),
                    None,
                    card.style.stroke,
                    1.6,
                );
                scene.rect(
                    axis - 5.0,
                    cy - 5.0,
                    10.0,
                    10.0,
                    5.0,
                    card.style.stroke,
                    None,
                );
                card.draw(scene, left, y);
                y += card.height + GAP;
            }
        }
    }
}

fn size(scene: &Scene, groups: &[Group], horizontal: bool) -> (f32, f32) {
    if groups.is_empty() {
        return (scene.measure("No periods"), 22.0);
    }
    let (width, height) = groups.iter().map(|group| group.size(horizontal)).fold(
        (0.0_f32, 0.0_f32),
        |(width, height), (w, h)| {
            if horizontal {
                (width + w, height.max(h))
            } else {
                (width.max(w), height + h)
            }
        },
    );
    if horizontal {
        let width = width + (groups.len() - 1) as f32 * SECTION_GAP;
        let headings = groups.iter().map(Group::heading_height).fold(0.0, f32::max);
        if height > 0.0 {
            (width + GAP, headings + LANE + height)
        } else {
            (width, headings)
        }
    } else {
        (width, height)
    }
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let timeline = parse(source)?;
    let palette = Palette::new(options);
    let style = |i: usize| {
        let mut style = palette.slot(i);
        let accent = if options.color {
            ACCENTS[i % ACCENTS.len()]
        } else {
            scene.ink()
        };
        let stroke = style.stroke.unwrap_or(accent);
        let [r, g, b, _] = stroke.rgba();
        style.stroke = Some(stroke);
        style.fill = style.fill.or(Some(Rgb::from_rgba(r, g, b, 30)));
        style
    };
    let mut groups = Vec::new();
    if !timeline.periods.is_empty() {
        groups.push(Group {
            heading: None,
            cards: timeline
                .periods
                .iter()
                .enumerate()
                .map(|(i, period)| Card::new(scene, period, style(i)))
                .collect(),
            accent: None,
        });
    }
    for (i, section) in timeline.sections.iter().enumerate() {
        let style = style(i);
        groups.push(Group {
            heading: Some(label(scene, section.label)),
            cards: section
                .periods
                .iter()
                .map(|period| Card::new(scene, period, style))
                .collect(),
            accent: style.stroke,
        });
    }

    let title = timeline.title.map(|text| label(scene, text));
    let horizontal = matches!(timeline.direction, Direction::LeftToRight);
    let (width, height) = size(scene, &groups, horizontal);
    let title_width = title.as_ref().map_or(0.0, |label| label.width);
    let title_height = title.as_ref().map_or(0.0, |label| label.height() + 12.0);
    scene.resize(
        width.max(title_width) + 2.0 * IMAGE_PAD,
        height + title_height + 2.0 * IMAGE_PAD,
    )?;
    if let Some(title) = title {
        let color = options
            .color
            .then(|| options.config.theme_color("titleColor"))
            .flatten();
        title.draw(scene, IMAGE_PAD, IMAGE_PAD, color);
    }
    let y = IMAGE_PAD + title_height;
    if groups.is_empty() {
        scene.text(IMAGE_PAD, y, "No periods", None);
    } else {
        draw_groups(scene, &groups, y, (width, height), horizontal);
    }
    Ok(())
}
