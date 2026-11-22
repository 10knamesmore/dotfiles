use std::{collections::BTreeMap, fmt::Write};

use super::{Caption, Connections, Frame, connections, parse};
use crate::{
    Options,
    charts::shared::{
        plot::image::{self, Point},
        sets::image::Card,
    },
    color::{Rgb, Style},
    graphics::Scene,
};

const MARGIN: f32 = 18.0;
const LANE_HEADING: f32 = 146.0;
const GAP: f32 = 42.0;
const PAD: f32 = 12.0;

struct Placement {
    lane: usize,
    width: f32,
    x: f32,
    card: Card,
}

fn lane_key(frame: &Frame) -> (usize, &str, &str) {
    let (group, namespace) = frame.lane();
    let rank = match group {
        "UI / automation" => 0,
        "Commands / read models" => 1,
        "Events" => 2,
        _ => 3,
    };
    (rank, group, namespace)
}

fn link(scene: &mut Scene, points: &[Point], (stroke, arrow): (Rgb, Rgb)) {
    let mut path = format!("M {} {}", points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        write!(path, " L {x} {y}").unwrap();
    }
    scene.path(&path, None, Some(stroke), 1.5);
    let &(x, y) = points.last().unwrap();
    let base = x - 8.0;
    image::polygon(
        scene,
        &[(x, y), (base, y - 4.0), (base, y + 4.0)],
        Some(arrow),
        None,
        0.0,
    );
}

pub(super) fn draw(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (model, edges) = parse(source)?;
    let Connections {
        adjacent,
        tracks,
        channels,
    } = connections(edges)?;
    let theme = |key| image::theme(options, key);
    let ink = scene.ink();
    let primary = options.theme_style();
    let stroke = theme("emRelationStroke")
        .or_else(|| options.theme_line())
        .unwrap_or(ink);
    let line_colors = (stroke, theme("emArrowhead").unwrap_or(stroke));

    let mut keys: BTreeMap<_, _> = model.frames.iter().map(|f| (lane_key(f), 0)).collect();
    for (i, slot) in keys.values_mut().enumerate() {
        *slot = i;
    }
    let mut lane_heights: Vec<f32> = vec![72.0; keys.len()];
    let mut column = MARGIN + LANE_HEADING + GAP / 2.0;
    let mut frames = Vec::new();
    for frame in &model.frames {
        let lane = keys[&lane_key(frame)];
        let heading = frame.heading(" · ");
        let rows = frame.rows();
        let width = rows
            .iter()
            .chain(std::iter::once(&heading))
            .flat_map(|s| s.lines())
            .map(|line| scene.measure(line) + 28.0)
            .fold(150.0, f32::max)
            .min(245.0);
        let accent = frame
            .stroke_key()
            .and_then(theme)
            .or(primary.stroke)
            .unwrap_or_else(|| {
                image::palette(
                    match frame.kind.as_str() {
                        "Command" => 0,
                        "Event" => 1,
                        "Read model" => 2,
                        "Processor" => 3,
                        "UI" => 4,
                        _ => 5,
                    },
                    options.color,
                )
            });
        let style = Style {
            fill: primary.fill.or_else(|| Some(image::fade(accent, 0.09))),
            stroke: Some(accent),
            text: primary.text,
        };
        let card = Card::new(scene, &heading, &rows, width, style);
        lane_heights[lane] = lane_heights[lane].max(card.height() + 2.0 * PAD);
        frames.push(Placement {
            lane,
            width,
            x: column,
            card,
        });
        column += width + GAP;
    }
    let width = if frames.is_empty() {
        520.0
    } else {
        column - GAP / 2.0 + MARGIN
    }
    .max(520.0);
    let mut y = MARGIN;
    for caption in &model.captions {
        let (text, color) = match caption {
            Caption::Title(text) => (text, theme("titleColor")),
            Caption::Description(text) => (text, None),
        };
        for row in image::wrap(scene, text, width - 2.0 * MARGIN) {
            scene.text(MARGIN, y, &row, color);
            y += 20.0;
        }
    }
    let track_top = y;
    if channels > 0 {
        y += 12.0 * channels as f32 + 8.0;
    }
    let mut lanes = Vec::new();
    for (((_, name, namespace), _), height) in keys.into_iter().zip(lane_heights) {
        let heading = if namespace.is_empty() {
            name.to_owned()
        } else {
            format!("{name}\n{namespace}")
        };
        let label = image::wrap(scene, &heading, LANE_HEADING - 2.0 * PAD);
        let height = height.max(20.0 * label.len() as f32 + 2.0 * PAD);
        let shade = image::fade(ink, 0.045);
        let border = image::fade(ink, 0.28);
        scene.rect(
            MARGIN,
            y,
            width - 2.0 * MARGIN,
            height,
            6.0,
            Some(shade),
            Some(border),
        );
        image::line(
            scene,
            (MARGIN + LANE_HEADING, y),
            (MARGIN + LANE_HEADING, y + height),
            border,
            1.2,
        );
        for (i, row) in label.iter().enumerate() {
            scene.text(MARGIN + PAD, y + PAD + 20.0 * i as f32, row, None);
        }
        lanes.push(y);
        y += height + 6.0;
    }
    let port_y = |i: usize| lanes[frames[i].lane] + PAD + 20.0;
    for a in adjacent {
        let from = &frames[a];
        let to = &frames[a + 1];
        let (x, u) = (from.x + from.width, to.x);
        let (y, v) = (port_y(a), port_y(a + 1));
        let mid = (x + u) / 2.0;
        link(scene, &[(x, y), (mid, y), (mid, v), (u, v)], line_colors);
    }
    for track in tracks {
        let from = &frames[track.source];
        let to = &frames[track.target];
        let (x, u) = (from.x + from.width, to.x);
        let (y, v) = (port_y(track.source) + 12.0, port_y(track.target) - 10.0);
        let shift = (track.channel % 5) as f32 * 2.5;
        let (out, into) = (x + 8.0 + shift, u - 8.0 - shift);
        let above = track_top + 7.0 + 12.0 * track.channel as f32;
        link(
            scene,
            &[
                (x, y),
                (out, y),
                (out, above),
                (into, above),
                (into, v),
                (u, v),
            ],
            line_colors,
        );
    }
    for placement in frames {
        placement
            .card
            .draw(scene, placement.x, lanes[placement.lane] + PAD);
    }

    let mut appendices = Vec::new();
    if !model.entities.is_empty() {
        appendices.push(("Entities".to_owned(), model.entities));
    }
    appendices.extend(
        model
            .data
            .into_iter()
            .map(|(id, fields)| (format!("Data · {id}"), fields)),
    );
    let card_width = 230.0;
    let appendix_style = Style {
        stroke: primary.stroke.or(Some(image::fade(ink, 0.55))),
        ..primary
    };
    let mut x = MARGIN;
    let mut bottom = y;
    if !appendices.is_empty() && !lanes.is_empty() {
        y += 10.0;
    }
    for (heading, rows) in appendices {
        if x + card_width > width - MARGIN {
            x = MARGIN;
            y = bottom + PAD;
        }
        let card = Card::new(scene, &heading, &rows, card_width, appendix_style);
        card.draw(scene, x, y);
        bottom = bottom.max(y + card.height());
        x += card_width + PAD;
    }
    scene.resize(width, bottom + MARGIN)
}
