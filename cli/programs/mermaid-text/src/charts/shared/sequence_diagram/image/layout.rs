use super::super::{Diagram, Event, Placement};
use super::{FRAME_INSET, MARGIN, Text, participant::Header};

const GAP: f32 = 28.0;
const GROUP_PAD: f32 = 12.0;
const ROW_GAP: f32 = 14.0;
pub(super) const BAR_HALF: f32 = 6.0;
pub(super) const BAR_OFFSET: f32 = 5.0;
pub(super) const HELPER_GAP: f32 = 8.0;
pub(super) const SELF_REACH: f32 = 38.0;
pub(super) const SELF_DROP: f32 = 24.0;

pub(super) struct Columns {
    pub centers: Vec<f32>,
    pub width: f32,
}

impl Columns {
    pub(super) fn group_bounds(
        &self,
        headers: &[Header<'_>],
        first: usize,
        last: usize,
    ) -> (f32, f32) {
        (
            self.centers[first] - headers[first].width / 2.0 - GROUP_PAD,
            self.centers[last] + headers[last].width / 2.0 + GROUP_PAD,
        )
    }
}

#[derive(Default, Clone)]
struct Clearance {
    left: f32,
    right: f32,
    group_left: f32,
    group_right: f32,
    spans: Vec<(usize, f32)>,
}

pub(super) fn columns(
    diagram: &Diagram<'_>,
    headers: &mut [Header<'_>],
    group_labels: &[Text<'_>],
    labels: &[Text<'_>],
    title: &Text<'_>,
) -> Columns {
    let mut room = vec![Clearance::default(); headers.len()];
    let mut levels = vec![0usize; headers.len()];
    let mut reach = vec![0.0_f32; headers.len()];
    for event in &diagram.events {
        if let Event::Activation { participant, start } = *event {
            let level = &mut levels[participant];
            if start {
                reach[participant] = reach[participant].max(BAR_HALF + *level as f32 * BAR_OFFSET);
                *level += 1;
            } else {
                *level = level.saturating_sub(1);
            }
        }
    }
    for (clearance, &right) in room.iter_mut().zip(&reach) {
        clearance.right = right;
    }
    let mut depth = 0;
    let mut heading_width = 0.0_f32;
    let mut creating = false;
    for (event, text) in diagram.events.iter().zip(labels) {
        match *event {
            Event::Message { from, to, .. } if from == to => {
                room[from].right = room[from]
                    .right
                    .max(reach[from] + SELF_REACH.max(text.width + HELPER_GAP));
            }
            Event::Message { from, to, .. } => {
                let left = from.min(to);
                let right = from.max(to);
                let port = if std::mem::take(&mut creating) {
                    headers[to].creation_port(from < to).0
                } else {
                    0.0
                };
                let left_edge = reach[left].max(port);
                let right_edge = BAR_HALF.max(-port);
                room[right]
                    .spans
                    .push((left, text.width + GAP + left_edge + right_edge));
            }
            Event::Note { placement, .. } => {
                let box_width = note_width(text);
                match placement {
                    Placement::Left(at) => room[at].left = room[at].left.max(box_width + GAP),
                    Placement::Right(at) => {
                        room[at].right = room[at].right.max(box_width + GAP + reach[at]);
                    }
                    Placement::Over(at) => {
                        room[at].left = room[at].left.max(box_width / 2.0);
                        room[at].right = room[at].right.max(box_width / 2.0);
                    }
                    Placement::Between(left, right) => {
                        room[left].left = room[left].left.max(GROUP_PAD);
                        room[right].right = room[right].right.max(GROUP_PAD);
                        room[right].spans.push((left, box_width - 2.0 * GROUP_PAD));
                    }
                }
            }
            Event::Start { .. } => {
                heading_width = heading_width.max(depth as f32 * FRAME_INSET * 2.0 + text.width);
                depth += 1;
            }
            Event::Branch(..) => {
                heading_width =
                    heading_width.max((depth - 1) as f32 * FRAME_INSET * 2.0 + text.width);
            }
            Event::End => depth -= 1,
            Event::Create(_) => creating = true,
            _ => {}
        }
    }
    for (group, label) in diagram.groups.iter().zip(group_labels) {
        let first = group.first;
        let last = group.last;
        room[first].group_left = GROUP_PAD;
        room[last].group_right = GROUP_PAD;
        if first == last {
            headers[first].width = headers[first].width.max(label.width);
        } else {
            let existing = (headers[first].width + headers[last].width) / 2.0;
            room[last].spans.push((first, label.width - existing));
        }
    }
    let margin = MARGIN + diagram.max_depth as f32 * FRAME_INSET;
    let mut centers = Vec::with_capacity(headers.len());
    let mut header_edge = margin;
    let mut body_edge = margin;
    for (header, room) in headers.iter().zip(&room) {
        let half = header.width / 2.0;
        let mut center = (header_edge + room.group_left + half).max(body_edge + room.left);
        for &(left, distance) in &room.spans {
            center = center.max(centers[left] + distance);
        }
        centers.push(center);
        header_edge = center + half + room.group_right + GAP;
        body_edge = center + room.right + GAP;
    }
    let width = ((header_edge.max(body_edge) - GAP) + margin)
        .max(2.0 * (MARGIN + GROUP_PAD) + heading_width)
        .max(2.0 * MARGIN + title.width);
    Columns { centers, width }
}

pub(super) struct Frame {
    pub event: usize,
    pub depth: usize,
    pub top: f32,
    pub bottom: f32,
}

pub(super) struct Lifeline {
    pub header_top: f32,
    pub start: f32,
    pub end: Option<f32>,
    active: Vec<f32>,
}

impl Lifeline {
    fn new(top: f32, height: f32) -> Self {
        Self {
            header_top: top,
            start: top + height,
            end: None,
            active: Vec::new(),
        }
    }

    fn endpoint(&self, center: f32, direction: f32, central: bool) -> f32 {
        if central || self.active.is_empty() {
            center
        } else if direction < 0.0 {
            center - BAR_HALF
        } else {
            center + BAR_HALF + (self.active.len() - 1) as f32 * BAR_OFFSET
        }
    }
}

pub(super) struct Bar {
    pub participant: usize,
    pub level: usize,
    pub top: f32,
    pub bottom: f32,
}

pub(super) enum Row {
    Message {
        start: f32,
        end: f32,
        label_top: f32,
        line_y: f32,
    },
    Note {
        left: f32,
        right: f32,
        top: f32,
    },
    Branch {
        frame: usize,
        top: f32,
    },
    None,
}

pub(super) struct Body {
    pub rows: Vec<Row>,
    pub lifelines: Vec<Option<Lifeline>>,
    pub bars: Vec<Bar>,
    pub frames: Vec<Frame>,
    pub bottom: f32,
}

pub(super) fn body(
    diagram: &Diagram<'_>,
    headers: &[Header<'_>],
    labels: &[Text<'_>],
    columns: &Columns,
    header_top: f32,
) -> Result<Body, String> {
    let height = diagram
        .participants
        .iter()
        .zip(headers)
        .filter(|(participant, _)| !participant.deferred)
        .map(|(_, header)| header.height)
        .fold(0.0, f32::max);
    let mut lifelines: Vec<_> = diagram
        .participants
        .iter()
        .zip(headers)
        .map(|(participant, header)| {
            (!participant.deferred)
                .then(|| Lifeline::new(header_top + height - header.height, header.height))
        })
        .collect();
    let mut y = header_top + height + ROW_GAP;
    let mut rows = Vec::with_capacity(diagram.events.len());
    let mut frames = Vec::<Frame>::new();
    let mut open = Vec::<usize>::new();
    let mut bars = Vec::<Bar>::new();
    let mut pending_creation = None;
    let mut last_signal = None;
    let mut destruction_y = y;
    let centers = &columns.centers;
    for (event, text) in diagram.events.iter().zip(labels) {
        let event_index = rows.len();
        let previous_signal = last_signal.take();
        let row = match *event {
            Event::Message {
                from, to, arrow, ..
            } => {
                let self_message = from == to;
                let direction = if centers[from] <= centers[to] {
                    1.0
                } else {
                    -1.0
                };
                let start = live(diagram, &mut lifelines, from)?.endpoint(
                    centers[from],
                    direction,
                    arrow.central_source,
                );
                let label_top = y;
                let mut line_y = label_top + text.height() + HELPER_GAP;
                let end = if pending_creation.take() == Some(to) {
                    let header = &headers[to];
                    let (port_x, port_y) = header.creation_port(from < to);
                    lifelines[to] = Some(Lifeline::new(line_y, header.height));
                    line_y += port_y;
                    centers[to] + port_x
                } else {
                    live(diagram, &mut lifelines, to)?.endpoint(
                        centers[to],
                        if self_message { 1.0 } else { -direction },
                        arrow.central_target,
                    )
                };
                let bottom = line_y + if self_message { SELF_DROP } else { 0.0 };
                y = lifelines[to].as_ref().unwrap().start.max(bottom) + ROW_GAP;
                last_signal = Some(bottom);
                destruction_y = bottom;
                Row::Message {
                    start,
                    end,
                    label_top,
                    line_y,
                }
            }
            Event::Note { placement, .. } => {
                let width = note_width(text);
                let (left, right) = match placement {
                    Placement::Left(at) => (centers[at] - GAP - width, centers[at] - GAP),
                    Placement::Right(at) => {
                        let edge = lifelines[at]
                            .as_ref()
                            .map_or(centers[at], |life| life.endpoint(centers[at], 1.0, false))
                            + GAP;
                        (edge, edge + width)
                    }
                    Placement::Over(at) => (centers[at] - width / 2.0, centers[at] + width / 2.0),
                    Placement::Between(left, right) => {
                        (centers[left] - GROUP_PAD, centers[right] + GROUP_PAD)
                    }
                };
                let top = y;
                y += text.height() + 2.0 * HELPER_GAP + ROW_GAP;
                Row::Note { left, right, top }
            }
            Event::Start { .. } => {
                let index = frames.len();
                frames.push(Frame {
                    event: event_index,
                    depth: open.len(),
                    top: y,
                    bottom: y,
                });
                open.push(index);
                y += text.height() + ROW_GAP;
                Row::None
            }
            Event::Branch(..) => {
                let top = y;
                y += text.height() + ROW_GAP;
                Row::Branch {
                    frame: *open.last().unwrap(),
                    top,
                }
            }
            Event::End => {
                frames[open.pop().unwrap()].bottom = y;
                y += ROW_GAP;
                Row::None
            }
            Event::Activation { participant, start } => {
                let life = live(diagram, &mut lifelines, participant)?;
                let at = previous_signal.unwrap_or(y).max(life.start);
                if start {
                    life.active.push(at);
                } else {
                    let top = life.active.pop().ok_or_else(|| {
                        format!(
                            "cannot deactivate inactive participant `{}`",
                            diagram.participants[participant].label.text,
                        )
                    })?;
                    bars.push(Bar {
                        participant,
                        level: life.active.len(),
                        top,
                        bottom: at,
                    });
                }
                if previous_signal.is_none() {
                    y += ROW_GAP;
                }
                Row::None
            }
            Event::Create(id) => {
                pending_creation = Some(id);
                Row::None
            }
            Event::Destroy(participant) => {
                let life = live(diagram, &mut lifelines, participant)?;
                life.end = Some(destruction_y);
                Row::None
            }
        };
        rows.push(row);
    }
    for (participant, life) in lifelines.iter_mut().enumerate() {
        if let Some(life) = life {
            for (level, top) in life.active.drain(..).enumerate() {
                bars.push(Bar {
                    participant,
                    level,
                    top,
                    bottom: life.end.unwrap_or(y),
                });
            }
        }
    }
    bars.sort_unstable_by_key(|bar| bar.level);
    Ok(Body {
        rows,
        lifelines,
        bars,
        frames,
        bottom: y,
    })
}

fn live<'a>(
    diagram: &Diagram<'_>,
    lifelines: &'a mut [Option<Lifeline>],
    id: usize,
) -> Result<&'a mut Lifeline, String> {
    lifelines[id]
        .as_mut()
        .filter(|life| life.end.is_none())
        .ok_or_else(|| {
            format!(
                "participant `{}` has no live lifeline",
                diagram.participants[id].label.text
            )
        })
}

fn note_width(text: &Text<'_>) -> f32 {
    (text.width + GAP).max(48.0)
}
