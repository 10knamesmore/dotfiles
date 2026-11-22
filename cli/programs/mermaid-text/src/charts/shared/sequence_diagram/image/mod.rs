use std::borrow::Cow;
use std::fmt::Write;

use crate::Options;
use crate::color::Rgb;
use crate::graphics::Scene;

use super::{Arrow, ArrowHead, Diagram, Event, Theme, label::Label};

mod layout;
mod menu;
mod participant;
use layout::{BAR_HALF, BAR_OFFSET, HELPER_GAP, Row, SELF_DROP, SELF_REACH};
use participant::Header;

const MARGIN: f32 = 18.0;
const FRAME_INSET: f32 = 12.0;
const LINE_HEIGHT: f32 = 22.0;
const STROKE: f32 = 1.5;

#[derive(Default)]
struct Text<'a> {
    lines: Vec<(Cow<'a, str>, f32)>,
    width: f32,
}

impl<'a> Text<'a> {
    fn measure(scene: &Scene, lines: impl Iterator<Item = Cow<'a, str>>) -> Self {
        let lines: Vec<_> = lines
            .map(|line| {
                let width = scene.measure(&line);
                (line, width)
            })
            .collect();
        let width = lines.iter().map(|(_, width)| *width).fold(0.0, f32::max);
        Self { lines, width }
    }

    fn label(scene: &Scene, label: &'a Label<'_>) -> Self {
        Self::with_prefix(scene, label, None)
    }

    fn with_prefix(scene: &Scene, label: &'a Label<'_>, prefix: Option<&'a str>) -> Self {
        let mut lines = label
            .lines(usize::MAX)
            .into_iter()
            .filter(|_| !label.text.is_empty())
            .map(|line| line.text);
        let first = prefix.map(|prefix| match lines.next() {
            Some(line) if !line.is_empty() => Cow::Owned(format!("{prefix}  {line}")),
            _ => Cow::Borrowed(prefix),
        });
        Self::measure(scene, first.into_iter().chain(lines.map(Cow::Borrowed)))
    }

    fn height(&self) -> f32 {
        self.lines.len() as f32 * LINE_HEIGHT
    }

    fn draw(&self, scene: &mut Scene, x: f32, top: f32, centered: bool, color: Option<Rgb>) {
        for (row, (line, width)) in self.lines.iter().enumerate() {
            let left = if centered { x - width / 2.0 } else { x };
            scene.text(left, top + row as f32 * LINE_HEIGHT, line, color);
        }
    }
}

pub(super) fn render(
    diagram: &Diagram<'_>,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    if diagram.participants.is_empty() {
        return Err("sequence diagram has no participants".into());
    }
    let mut headers: Vec<_> = diagram
        .participants
        .iter()
        .map(|participant| Header::new(scene, participant))
        .collect();
    let title = diagram
        .title
        .as_ref()
        .map_or_else(Text::default, |label| Text::label(scene, label));
    let group_labels: Vec<_> = diagram
        .groups
        .iter()
        .map(|group| Text::label(scene, &group.label))
        .collect();
    let labels: Vec<_> = diagram
        .events
        .iter()
        .map(|event| match event {
            Event::Message { label, number, .. } => {
                Text::with_prefix(scene, label, number.as_deref())
            }
            Event::Note { label, .. } => Text::label(scene, label),
            Event::Start {
                keyword: "rect",
                label,
                ..
            } => Text::label(scene, label),
            Event::Start { keyword, label, .. } | Event::Branch(keyword, label) => {
                Text::with_prefix(scene, label, Some(keyword))
            }
            _ => Text::default(),
        })
        .collect();
    let columns = layout::columns(diagram, &mut headers, &group_labels, &labels, &title);
    let group_top = MARGIN + title.height() + if title.lines.is_empty() { 0.0 } else { MARGIN };
    let header_top = group_labels
        .iter()
        .map(Text::height)
        .reduce(f32::max)
        .map_or(group_top, |height| group_top + height + 2.0 * HELPER_GAP);
    let body = layout::body(diagram, &headers, &labels, &columns, header_top)?;
    let menus = menu::Footer::new(diagram, scene);
    scene.resize(
        columns.width.max(menus.canvas_width()),
        body.bottom + menus.height() + MARGIN,
    )?;

    let theme = Theme::new(options);
    let color = |key| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let ink = scene.ink();
    let paper = scene.paper();
    let signal = theme.signal.unwrap_or(ink);
    let group_ink = color("actorBorder").unwrap_or(ink);
    let rounded = options.corners.resolve(false, ['┌', '┐', '└', '┘']) == ['╭', '╮', '╰', '╯'];
    let radius = if rounded { 6.0 } else { 0.0 };
    title.draw(
        scene,
        columns.width / 2.0,
        MARGIN,
        true,
        color("titleColor"),
    );
    for (group, text) in diagram.groups.iter().zip(&group_labels) {
        let (left, right) = columns.group_bounds(&headers, group.first, group.last);
        scene.rect(
            left,
            group_top,
            right - left,
            body.bottom - group_top,
            radius,
            group.fill.filter(|_| options.color),
            Some(group_ink),
        );
        text.draw(
            scene,
            (left + right) / 2.0,
            group_top + HELPER_GAP,
            true,
            color("actorTextColor"),
        );
    }
    for frame in &body.frames {
        if let Event::Start {
            fill: Some(fill), ..
        } = diagram.events[frame.event]
            && options.color
        {
            let (left, right) = frame_bounds(frame.depth, columns.width);
            scene.rect(
                left,
                frame.top,
                right - left,
                frame.bottom - frame.top,
                0.0,
                Some(fill),
                None,
            );
        }
    }

    let lifeline_ink = color("actorLineColor")
        .or(theme.actor.stroke)
        .unwrap_or(ink);
    for (&x, life) in columns.centers.iter().zip(&body.lifelines) {
        if let Some(life) = life {
            stroke(
                scene,
                &[(x, life.start), (x, life.end.unwrap_or(body.bottom))],
                true,
                lifeline_ink,
            );
        }
    }
    let activation_fill = color("activationBkgColor")
        .or(theme.actor.fill)
        .or(Some(paper));
    let activation_border = color("activationBorderColor").or(Some(lifeline_ink));
    for bar in &body.bars {
        let x = columns.centers[bar.participant] + bar.level as f32 * BAR_OFFSET;
        scene.rect(
            x - BAR_HALF,
            bar.top,
            2.0 * BAR_HALF,
            bar.bottom - bar.top,
            0.0,
            activation_fill,
            activation_border,
        );
    }
    for ((header, &x), life) in headers.iter().zip(&columns.centers).zip(&body.lifelines) {
        if let Some(life) = life {
            header.draw(scene, x, life.header_top, radius, theme.actor);
        }
    }

    let frame_ink = color("labelBoxBorderColor").or(theme.signal).unwrap_or(ink);
    let heading_ink = color("labelTextColor");
    let heading_fill = color("labelBoxBkgColor").unwrap_or(paper);
    for frame in &body.frames {
        let Event::Start { keyword, .. } = diagram.events[frame.event] else {
            unreachable!()
        };
        let (left, right) = frame_bounds(frame.depth, columns.width);
        if keyword != "rect" {
            scene.rect(
                left,
                frame.top,
                right - left,
                frame.bottom - frame.top,
                radius,
                None,
                Some(frame_ink),
            );
        }
        let text = &labels[frame.event];
        if !text.lines.is_empty() {
            if keyword != "rect" {
                scene.rect(
                    left,
                    frame.top,
                    text.width + 2.0 * HELPER_GAP,
                    text.height() + 4.0,
                    0.0,
                    Some(heading_fill),
                    Some(frame_ink),
                );
            }
            text.draw(
                scene,
                left + HELPER_GAP,
                frame.top + 2.0,
                false,
                heading_ink,
            );
        }
    }
    for ((event, row), text) in diagram.events.iter().zip(&body.rows).zip(&labels) {
        match (event, row) {
            (
                Event::Message {
                    arrow, from, to, ..
                },
                &Row::Message {
                    start,
                    end,
                    label_top,
                    line_y,
                },
            ) => {
                let self_message = from == to;
                let center = if self_message {
                    start.max(end) + HELPER_GAP + text.width / 2.0
                } else {
                    (start + end) / 2.0
                };
                text.draw(scene, center, label_top, true, theme.message_text);
                draw_message(scene, *arrow, start, end, line_y, self_message, signal);
            }
            (Event::Note { .. }, &Row::Note { left, right, top }) => {
                scene.rect(
                    left,
                    top,
                    right - left,
                    text.height() + 2.0 * HELPER_GAP,
                    radius,
                    theme.note.fill.or(Some(paper)),
                    theme.note.stroke.or(Some(ink)),
                );
                text.draw(
                    scene,
                    (left + right) / 2.0,
                    top + HELPER_GAP,
                    true,
                    theme.note.text,
                );
            }
            (Event::Branch(..), &Row::Branch { frame, top }) => {
                let (left, right) = frame_bounds(body.frames[frame].depth, columns.width);
                stroke(scene, &[(left, top), (right, top)], true, frame_ink);
                scene.rect(
                    left + 1.0,
                    top + 1.0,
                    text.width + 2.0 * HELPER_GAP,
                    text.height() + 4.0,
                    0.0,
                    Some(heading_fill),
                    None,
                );
                text.draw(scene, left + HELPER_GAP, top + 2.0, false, heading_ink);
            }
            (Event::Destroy(id), _) => {
                cross(
                    scene,
                    columns.centers[*id],
                    body.lifelines[*id].as_ref().unwrap().end.unwrap(),
                    7.0,
                    signal,
                );
            }
            _ => {}
        }
    }
    menus.draw(scene, body.bottom, radius, theme.actor);
    Ok(())
}

fn frame_bounds(depth: usize, width: f32) -> (f32, f32) {
    let left = MARGIN + depth as f32 * FRAME_INSET;
    (left, width - left)
}

fn draw_message(
    scene: &mut Scene,
    arrow: Arrow,
    start: f32,
    end: f32,
    y: f32,
    self_message: bool,
    color: Rgb,
) {
    let direction = if start <= end { 1.0 } else { -1.0 };
    let (send_dir, recv_dir) = if self_message {
        (1.0, -1.0)
    } else {
        (direction, direction)
    };
    let recv_y = y + if self_message { SELF_DROP } else { 0.0 };
    let start_tip = start
        + if arrow.central_source {
            send_dir * 4.0
        } else {
            0.0
        };
    let end_tip = end
        - if arrow.central_target {
            recv_dir * 4.0
        } else {
            0.0
        };
    if self_message {
        let turn = start.max(end) + SELF_REACH;
        stroke(
            scene,
            &[(start_tip, y), (turn, y), (turn, recv_y), (end_tip, recv_y)],
            arrow.dotted,
            color,
        );
    } else {
        stroke(scene, &[(start_tip, y), (end_tip, y)], arrow.dotted, color);
    }
    let paper = scene.paper();
    if arrow.central_source {
        circle(scene, start, y, 4.0, Some(paper), color);
    }
    if arrow.central_target {
        circle(scene, end, recv_y, 4.0, Some(paper), color);
    }
    arrow_head(scene, arrow.head, end_tip, recv_y, recv_dir, color);
    if matches!(arrow.head, ArrowHead::Both) {
        arrow_head(scene, ArrowHead::Filled, start_tip, y, -send_dir, color);
    }
}

fn arrow_head(scene: &mut Scene, head: ArrowHead, tip: f32, y: f32, direction: f32, color: Rgb) {
    let back = tip - direction * 9.0;
    match head {
        ArrowHead::None => {}
        ArrowHead::Cross => cross(scene, tip, y, 5.0, color),
        ArrowHead::Open => stroke(
            scene,
            &[(back, y - 4.5), (tip, y), (back, y + 4.5)],
            false,
            color,
        ),
        ArrowHead::Filled | ArrowHead::Both => {
            scene.path(
                &format!("M {tip} {y} L {back} {} V {} Z", y - 4.5, y + 4.5),
                Some(color),
                None,
                0.0,
            );
        }
    }
}

fn cross(scene: &mut Scene, x: f32, y: f32, r: f32, color: Rgb) {
    let (left, right, top, bottom) = (x - r, x + r, y - r, y + r);
    scene.path(
        &format!("M {left} {top} L {right} {bottom} M {left} {bottom} L {right} {top}"),
        None,
        Some(color),
        STROKE,
    );
}

fn circle(scene: &mut Scene, x: f32, y: f32, r: f32, fill: Option<Rgb>, stroke: Rgb) {
    let (left, right) = (x - r, x + r);
    scene.path(
        &format!("M {left} {y} A {r} {r} 0 0 1 {right} {y} A {r} {r} 0 0 1 {left} {y} Z"),
        fill,
        Some(stroke),
        STROKE,
    );
}

fn stroke(scene: &mut Scene, points: &[(f32, f32)], dashed: bool, color: Rgb) {
    let mut path = String::new();
    if dashed {
        for pair in points.windows(2) {
            let [(x, y), (end_x, end_y)] = [pair[0], pair[1]];
            let (dx, dy) = (end_x - x, end_y - y);
            let length = dx.hypot(dy);
            let mut offset = 0.0;
            while offset < length {
                let start = offset / length;
                let end = (offset + 4.0_f32).min(length) / length;
                write!(
                    path,
                    "M {} {} L {} {} ",
                    x + dx * start,
                    y + dy * start,
                    x + dx * end,
                    y + dy * end
                )
                .unwrap();
                offset += 8.0;
            }
        }
    } else if let Some((&(x, y), tail)) = points.split_first() {
        write!(path, "M {x} {y} ").unwrap();
        for &(x, y) in tail {
            write!(path, "L {x} {y} ").unwrap();
        }
    }
    if !path.is_empty() {
        scene.path(&path, None, Some(color), STROKE);
    }
}
