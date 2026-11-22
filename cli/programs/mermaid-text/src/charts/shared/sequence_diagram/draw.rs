use crate::Options;
use crate::color::{Layer, Paint, Rgb, Style};

use super::{
    Arrow, ArrowHead, Diagram, Event, Group, Label, ParticipantKind, Placement, Theme, label::Line,
};

const BOX_PADDING: usize = 4;
const FRAME_INSET: usize = 2;
const SELF_REACH: usize = 4;
const MAX_CELLS: usize = 8 * 1024 * 1024;

pub(super) fn render(diagram: &Diagram<'_>, options: &Options) -> Result<String, String> {
    let mut canvas = Canvas::new(diagram, options);
    let headers: Vec<_> = diagram
        .participants
        .iter()
        .map(|participant| {
            let mut lines = participant.label.lines(canvas.label_width);
            let stereotype = participant
                .declaration
                .map_or("", ParticipantKind::stereotype);
            if !stereotype.is_empty() {
                lines.insert(0, Line::new(stereotype));
            }
            lines
        })
        .collect();
    let initial = diagram
        .participants
        .iter()
        .enumerate()
        .filter_map(|(id, participant)| (!participant.deferred).then_some(id));
    let title = diagram
        .title
        .as_ref()
        .map_or_else(Vec::new, |title| title.lines(canvas.label_width));
    let groups: Vec<_> = diagram
        .groups
        .iter()
        .map(|group| canvas.place_group(group))
        .collect();
    let headers_height = initial
        .clone()
        .map(|id| headers[id].len())
        .max()
        .unwrap_or(0);
    let group_height = groups
        .iter()
        .map(|group| group.labels.len())
        .max()
        .map_or(0, |height| height + 2);
    let mut height = headers_height + 2 + title.len() + group_height;
    let labels: Vec<_> = diagram
        .events
        .iter()
        .map(|event| match *event {
            Event::Message {
                from,
                to,
                ref label,
                ref number,
                ..
            } => {
                let mut lines = if label.text.is_empty() {
                    Vec::new()
                } else {
                    label.lines(canvas.label_width)
                };
                if let Some(number) = number {
                    lines.insert(0, Line::new(number));
                }
                height += lines.len() + if from == to { 3 } else { 1 };
                lines
            }
            Event::Note { ref label, .. } => {
                let lines = label.lines(canvas.label_width);
                height += lines.len() + 2;
                lines
            }
            Event::Start { ref label, .. } | Event::Branch(_, ref label) => {
                let lines = label.lines(canvas.label_width);
                height += lines.len();
                lines
            }
            Event::Create(id) => {
                height += headers[id].len() + 2;
                Vec::new()
            }
            Event::End | Event::Activation { .. } | Event::Destroy(_) => {
                height += 1;
                Vec::new()
            }
        })
        .collect();
    if canvas.width > MAX_CELLS / height {
        return Err(format!("sequenceDiagram layout exceeds {MAX_CELLS} cells"));
    }

    let mut output = Rows::new(options.color);
    for label in title {
        let start = (canvas.width - label.width) / 2;
        output.emit(&canvas.row(0, false), Some((start, label)));
    }
    output.groups = &groups;
    if headers_height != 0 {
        canvas.draw_headers(&mut output, diagram, &headers, initial, (0, false), &groups);
    }
    let mut depth = 0;
    for (event, lines) in diagram.events.iter().zip(&labels) {
        match *event {
            Event::Message {
                from, to, arrow, ..
            } => {
                require_alive(&canvas, diagram, from)?;
                require_alive(&canvas, diagram, to)?;
                canvas.draw_message(&mut output, depth, from, to, arrow, lines);
            }
            Event::Note { placement, .. } => {
                canvas.draw_note(&mut output, depth, placement, lines);
            }
            Event::Start { keyword, fill, .. } => {
                output.start_region(canvas.frame_bounds(depth), fill);
                canvas.draw_frame_heading(
                    &mut output,
                    depth,
                    keyword,
                    lines,
                    canvas.box_corners.top,
                );
                depth += 1;
            }
            Event::Branch(keyword, _) => {
                canvas.draw_frame_heading(
                    &mut output,
                    depth - 1,
                    keyword,
                    lines,
                    (canvas.glyphs.branch_right, canvas.glyphs.branch_left),
                );
            }
            Event::End => {
                depth -= 1;
                canvas.draw_frame_edge(&mut output, depth, " end ", canvas.box_corners.bottom);
                output.end_region(canvas.frame_bounds(depth));
            }
            Event::Activation { participant, start } => {
                let active = require_alive(&canvas, diagram, participant)?;
                let depth_after = if start {
                    active + 1
                } else {
                    active.checked_sub(1).ok_or_else(|| {
                        format!(
                            "cannot deactivate inactive participant `{}`",
                            diagram.participants[participant].label.text,
                        )
                    })?
                };
                canvas.life[participant] = Life::Alive(depth_after);
                let mut row = canvas.row(depth, true);
                row[canvas.centers[participant]] = if start {
                    canvas.glyphs.activation_start
                } else {
                    canvas.glyphs.activation_end
                };
                output.emit(&row, None);
            }
            Event::Create(id) => {
                canvas.draw_headers(
                    &mut output,
                    diagram,
                    &headers,
                    std::iter::once(id),
                    (depth, true),
                    &[],
                );
                canvas.life[id] = Life::Alive(0);
            }
            Event::Destroy(id) => {
                require_alive(&canvas, diagram, id)?;
                canvas.life[id] = Life::Destroyed;
                let mut row = canvas.row(depth, true);
                row[canvas.centers[id]] = canvas.glyphs.cancel;
                output.emit(&row, None);
            }
        }
    }
    let mut text = output.finish();
    draw_menus(&mut text, diagram);
    Ok(text)
}

fn draw_menus(output: &mut String, diagram: &Diagram<'_>) {
    for (&participant, menu) in &diagram.menus {
        let Some(menu) = menu.text(&diagram.participants[participant].label) else {
            continue;
        };
        output.push('\n');
        output.push_str(&menu.heading);
        output.push_str(":\n");
        for entry in &menu.entries {
            output.push_str("  ");
            output.push_str(entry);
            output.push('\n');
        }
    }
}

fn require_alive(canvas: &Canvas, diagram: &Diagram<'_>, id: usize) -> Result<usize, String> {
    if let Life::Alive(active) = canvas.life[id] {
        Ok(active)
    } else {
        Err(format!(
            "participant `{}` has no live lifeline",
            diagram.participants[id].label.text
        ))
    }
}

fn frame_heading(keyword: &str, label: &str) -> String {
    if label.is_empty() {
        format!(" {keyword} ")
    } else {
        format!(" {keyword} {label} ")
    }
}

#[derive(Clone, Copy)]
struct Corners {
    top: (char, char),
    bottom: (char, char),
}

struct Glyphs {
    corners: Corners,
    rounded: Corners,
    horizontal: char,
    vertical: char,
    dotted: char,
    cross: char,
    branch_right: char,
    branch_left: char,
    branch_up: char,
    branch_down: char,
    arrow_left: char,
    arrow_right: char,
    open_left: char,
    open_right: char,
    cancel: char,
    central: char,
    activation: char,
    stacked: char,
    activation_start: char,
    activation_end: char,
    active_junctions: [char; 3],
    stacked_junctions: [char; 3],
}

impl Glyphs {
    const UNICODE: Self = Self {
        corners: Corners {
            top: ('┌', '┐'),
            bottom: ('└', '┘'),
        },
        rounded: Corners {
            top: ('╭', '╮'),
            bottom: ('╰', '╯'),
        },
        horizontal: '─',
        vertical: '│',
        dotted: '┄',
        cross: '┼',
        branch_right: '├',
        branch_left: '┤',
        branch_up: '┴',
        branch_down: '┬',
        arrow_left: '◀',
        arrow_right: '▶',
        open_left: '‹',
        open_right: '›',
        cancel: '×',
        central: '○',
        activation: '┃',
        stacked: '║',
        activation_start: '╽',
        activation_end: '╿',
        active_junctions: ['┠', '┨', '╂'],
        stacked_junctions: ['╟', '╢', '╫'],
    };

    const ASCII: Self = Self {
        corners: Corners {
            top: ('+', '+'),
            bottom: ('+', '+'),
        },
        rounded: Corners {
            top: ('(', ')'),
            bottom: ('(', ')'),
        },
        horizontal: '-',
        vertical: '|',
        dotted: '.',
        cross: '+',
        branch_right: '+',
        branch_left: '+',
        branch_up: '+',
        branch_down: '+',
        arrow_left: '<',
        arrow_right: '>',
        open_left: '<',
        open_right: '>',
        cancel: 'x',
        central: 'o',
        activation: '#',
        stacked: '!',
        activation_start: '[',
        activation_end: ']',
        active_junctions: ['#'; 3],
        stacked_junctions: ['!'; 3],
    };
}

#[derive(Default, Clone)]
struct Clearance {
    left: usize,
    right: usize,
    spans: Vec<(usize, usize)>,
    group_left: usize,
    group_right: usize,
}

struct Geometry {
    centers: Vec<usize>,
    participant_widths: Vec<usize>,
    width: usize,
}

struct GroupBox<'a> {
    left: usize,
    right: usize,
    members: std::ops::RangeInclusive<usize>,
    labels: Vec<Line<'a>>,
    fill: Option<Rgb>,
}

struct Canvas {
    centers: Vec<usize>,
    participant_widths: Vec<usize>,
    width: usize,
    label_width: usize,
    glyphs: Glyphs,
    box_corners: Corners,
    life: Vec<Life>,
    theme: Theme,
}

#[derive(Clone, Copy)]
enum Life {
    NotCreated,
    Alive(usize),
    Destroyed,
}

#[derive(Clone, Copy)]
enum Junction {
    Right,
    Left,
    Cross,
}

impl Canvas {
    fn new(diagram: &Diagram<'_>, options: &Options) -> Self {
        let theme = Theme::new(options);
        let glyphs = if options.ascii {
            Glyphs::ASCII
        } else {
            Glyphs::UNICODE
        };
        let Corners {
            top: (tl, tr),
            bottom: (bl, br),
        } = glyphs.corners;
        let [tl, tr, bl, br] = options.corners.resolve(options.ascii, [tl, tr, bl, br]);
        let box_corners = Corners {
            top: (tl, tr),
            bottom: (bl, br),
        };
        let life = diagram
            .participants
            .iter()
            .map(|participant| {
                if participant.deferred {
                    Life::NotCreated
                } else {
                    Life::Alive(0)
                }
            })
            .collect();
        let Geometry {
            centers,
            participant_widths,
            width,
        } = Self::measure(diagram, usize::MAX, 2);
        if width <= options.width {
            return Self {
                centers,
                participant_widths,
                width,
                label_width: usize::MAX,
                glyphs,
                box_corners,
                life,
                theme,
            };
        }

        let (mut low, target) = if Self::measure(diagram, 1, 1).width > options.width {
            let readable = options.width.clamp(4, 8);
            (readable, Self::measure(diagram, readable, 1).width)
        } else {
            (1, options.width)
        };
        let mut high = target;
        while low < high {
            let candidate = low + (high - low).div_ceil(2);
            if Self::measure(diagram, candidate, 1).width <= target {
                low = candidate;
            } else {
                high = candidate - 1;
            }
        }
        let label_width = low;
        let Geometry {
            centers,
            participant_widths,
            width,
        } = Self::measure(diagram, label_width, 1);
        Self {
            centers,
            participant_widths,
            width,
            label_width,
            glyphs,
            box_corners,
            life,
            theme,
        }
    }

    fn measure(diagram: &Diagram<'_>, label_width: usize, gap: usize) -> Geometry {
        let margin = diagram.max_depth * FRAME_INSET;
        let mut clearance = vec![Clearance::default(); diagram.participants.len()];
        let mut frame_heading_width = 0;
        let mut depth = 0;
        for event in &diagram.events {
            match *event {
                Event::Message {
                    from,
                    to,
                    ref label,
                    ref number,
                    ..
                } if from == to => {
                    clearance[from].right = clearance[from]
                        .right
                        .max(SELF_REACH.max(message_width(label, number.as_deref(), label_width)));
                }
                Event::Message {
                    from,
                    to,
                    ref label,
                    ref number,
                    ..
                } => {
                    clearance[from.max(to)].spans.push((
                        from.min(to),
                        message_width(label, number.as_deref(), label_width) + 1,
                    ));
                }
                Event::Note {
                    placement,
                    ref label,
                } => {
                    let box_width = label.fitted_width(label_width) + BOX_PADDING;
                    match placement {
                        Placement::Left(at) => {
                            clearance[at].left = clearance[at].left.max(box_width + 1);
                        }
                        Placement::Right(at) => {
                            clearance[at].right = clearance[at].right.max(box_width + 1);
                        }
                        Placement::Over(at) => {
                            let half = box_width / 2;
                            clearance[at].left = clearance[at].left.max(half);
                            clearance[at].right = clearance[at].right.max(box_width - half - 1);
                        }
                        Placement::Between(left, right) => {
                            clearance[left].left = clearance[left].left.max(1);
                            clearance[right].right = clearance[right].right.max(1);
                            clearance[right].spans.push((left, box_width - 3));
                        }
                    }
                }
                Event::Start {
                    keyword, ref label, ..
                } => {
                    frame_heading_width = frame_heading_width
                        .max(depth * FRAME_INSET * 2 + heading_width(keyword, label, label_width));
                    depth += 1;
                }
                Event::Branch(keyword, ref label) => {
                    frame_heading_width = frame_heading_width.max(
                        (depth - 1) * FRAME_INSET * 2 + heading_width(keyword, label, label_width),
                    );
                }
                Event::End => depth -= 1,
                Event::Activation { .. } | Event::Create(_) | Event::Destroy(_) => {}
            }
        }

        let mut participant_widths: Vec<_> = diagram
            .participants
            .iter()
            .map(|participant| {
                let stereotype = participant
                    .declaration
                    .map_or("", ParticipantKind::stereotype);
                participant
                    .label
                    .fitted_width(label_width)
                    .max(stereotype.len())
                    + BOX_PADDING
            })
            .collect();
        for group in &diagram.groups {
            clearance[group.first].group_left = 1;
            clearance[group.last].group_right = 1;
            let caption = group.label.fitted_width(label_width);
            if group.first == group.last {
                participant_widths[group.first] = participant_widths[group.first].max(caption + 2);
            } else {
                let edge_widths = participant_widths[group.first] / 2
                    + participant_widths[group.last].div_ceil(2);
                clearance[group.last]
                    .spans
                    .push((group.first, (caption + 2).saturating_sub(edge_widths)));
            }
        }
        let mut centers = Vec::with_capacity(diagram.participants.len());
        let mut header_end = margin;
        let mut body_end = margin;
        for (&box_width, room) in participant_widths.iter().zip(&clearance) {
            let half = box_width / 2;
            let mut center = (header_end + room.group_left + half).max(body_end);
            center = center.max(centers.last().map_or(margin, |previous| previous + 1) + room.left);
            // A label spanning multiple participants can push the right lifeline out.
            for &(left, distance) in &room.spans {
                center = center.max(centers[left] + distance);
            }
            centers.push(center);
            header_end = center + box_width - half + room.group_right + gap;
            body_end = center + room.right + 1;
        }

        let title_width = diagram
            .title
            .as_ref()
            .map_or(0, |title| title.fitted_width(label_width));
        let width = ((header_end - gap).max(body_end) + margin)
            .max(frame_heading_width)
            .max(title_width);
        Geometry {
            centers,
            participant_widths,
            width,
        }
    }

    fn row(&self, depth: usize, lifelines: bool) -> Vec<char> {
        let mut row = vec![' '; self.width];
        for border in 0..depth {
            let (left, right) = self.frame_bounds(border);
            row[left] = self.glyphs.vertical;
            row[right] = self.glyphs.vertical;
        }
        if lifelines {
            for (&column, life) in self.centers.iter().zip(&self.life) {
                row[column] = match life {
                    Life::Alive(0) => self.glyphs.vertical,
                    Life::Alive(1) => self.glyphs.activation,
                    Life::Alive(_) => self.glyphs.stacked,
                    Life::NotCreated | Life::Destroyed => ' ',
                };
            }
        }
        row
    }

    fn frame_bounds(&self, depth: usize) -> (usize, usize) {
        let left = depth * FRAME_INSET;
        (left, self.width - 1 - left)
    }

    fn draw_headers(
        &self,
        output: &mut Rows<'_, '_>,
        diagram: &Diagram<'_>,
        labels: &[Vec<Line<'_>>],
        participants: impl Iterator<Item = usize> + Clone,
        (depth, in_body): (usize, bool),
        groups: &[GroupBox<'_>],
    ) {
        let [mut top, mut middle, mut bottom] = std::array::from_fn(|_| self.row(depth, in_body));
        if !groups.is_empty() {
            self.draw_group_headings(output, groups);
            for group in groups {
                for row in [&mut top, &mut middle, &mut bottom] {
                    row[group.left] = self.glyphs.vertical;
                    row[group.right] = self.glyphs.vertical;
                }
            }
        }
        let bounds = |id: usize| {
            let width = self.participant_widths[id];
            let left = self.centers[id] - width / 2;
            (left, left + width - 1)
        };
        let paint = |output: &mut Rows<'_, '_>, edge| {
            for id in participants.clone() {
                output.paint_box(bounds(id), self.theme.actor, edge);
            }
        };
        for id in participants.clone() {
            let participant = &diagram.participants[id];
            let (left, right) = bounds(id);
            let corners = match participant.declaration {
                Some(ParticipantKind::Actor) => self.glyphs.rounded,
                Some(ParticipantKind::Database) => Corners {
                    top: self.glyphs.rounded.top,
                    bottom: self.glyphs.corners.bottom,
                },
                _ => self.box_corners,
            };
            self.horizontal(&mut top, left, right, corners.top);
            self.horizontal(&mut bottom, left, right, corners.bottom);
            if !diagram.events.is_empty() {
                bottom[self.centers[id]] = self.glyphs.branch_down;
            }
            middle[left] = self.glyphs.vertical;
            middle[right] = self.glyphs.vertical;
        }

        output.emit(&top, None);
        paint(output, true);
        let height = participants
            .clone()
            .map(|id| labels[id].len())
            .max()
            .unwrap_or(0);
        for line in 0..height {
            let overlays = participants.clone().filter_map(|id| {
                let label = *labels[id].get(line)?;
                let (left, right) = bounds(id);
                let start = left + (right - left + 1 - label.width) / 2;
                Some((start, label))
            });
            output.emit(&middle, overlays);
            paint(output, false);
        }
        output.emit(&bottom, None);
        paint(output, true);
        if !groups.is_empty() {
            let mut footer = self.row(0, !diagram.events.is_empty());
            for group in groups {
                self.horizontal(
                    &mut footer,
                    group.left,
                    group.right,
                    self.box_corners.bottom,
                );
                if !diagram.events.is_empty() {
                    for participant in group.members.clone() {
                        footer[self.centers[participant]] = self.glyphs.cross;
                    }
                }
            }
            output.emit(&footer, None);
        }
    }

    fn place_group<'a>(&self, group: &'a Group<'_>) -> GroupBox<'a> {
        GroupBox {
            left: self.centers[group.first] - self.participant_widths[group.first] / 2 - 1,
            right: self.centers[group.last] + self.participant_widths[group.last].div_ceil(2),
            members: group.first..=group.last,
            labels: if group.label.text.is_empty() {
                Vec::new()
            } else {
                group.label.lines(self.label_width)
            },
            fill: group.fill,
        }
    }

    fn draw_group_headings(&self, output: &mut Rows<'_, '_>, groups: &[GroupBox<'_>]) {
        let mut top = self.row(0, false);
        let mut row = self.row(0, false);
        for group in groups {
            self.horizontal(&mut top, group.left, group.right, self.box_corners.top);
            row[group.left] = self.glyphs.vertical;
            row[group.right] = self.glyphs.vertical;
        }
        output.emit(&top, None);
        let height = groups
            .iter()
            .map(|group| group.labels.len())
            .max()
            .unwrap_or(0);
        for index in 0..height {
            let labels = groups.iter().filter_map(|group| {
                let label = *group.labels.get(index)?;
                let left = group.left + (group.right - group.left + 1 - label.width) / 2;
                Some((left, label))
            });
            output.emit(&row, labels);
        }
    }

    fn draw_message(
        &self,
        output: &mut Rows<'_, '_>,
        depth: usize,
        sender: usize,
        receiver: usize,
        arrow: Arrow,
        labels: &[Line<'_>],
    ) {
        let from = self.centers[sender];
        let to = self.centers[receiver];
        let stroke = if arrow.dotted {
            self.glyphs.dotted
        } else {
            self.glyphs.horizontal
        };
        let (left, right) = (from.min(to), from.max(to));
        for &label in labels {
            let start = if from == to {
                from + 1
            } else {
                left + 1 + (right - left - 1 - label.width) / 2
            };
            output.emit(&self.row(depth, true), Some((start, label)));
            output.paint(start, label.width, foreground(self.theme.message_text));
        }

        let mut row = self.row(depth, true);
        let signal = foreground(self.theme.signal);
        if from == to {
            let turn = from + SELF_REACH;
            let lifeline = row[from];
            let junction = self.junction(self.life[sender], Junction::Right);
            row[from] = if arrow.central_source {
                self.glyphs.central
            } else {
                junction
            };
            row[from + 1..turn].fill(stroke);
            if matches!(arrow.head, ArrowHead::Both) {
                row[from + 1] = self.glyphs.arrow_right;
            }
            row[turn] = self.glyphs.corners.top.1;
            output.emit(&row, None);
            output.paint(from, SELF_REACH + 1, signal);
            row[from..turn].fill(' ');
            row[from] = lifeline;
            row[turn] = self.glyphs.vertical;
            output.emit(&row, None);
            output.paint(turn, 1, signal);
            row[from] = if arrow.central_target {
                self.glyphs.central
            } else {
                junction
            };
            row[from + 1..turn].fill(stroke);
            if let Some(head) = self.arrow_head(arrow.head, false) {
                row[from + 1] = head;
            }
            row[turn] = self.glyphs.corners.bottom.1;
            output.emit(&row, None);
            output.paint(from, SELF_REACH + 1, signal);
        } else {
            row[left + 1..right].fill(stroke);
            for (&column, life) in self.centers.iter().zip(&self.life) {
                if left < column && column < right && matches!(life, Life::Alive(_)) {
                    row[column] = self.junction(*life, Junction::Cross);
                }
            }
            let rightward = from < to;
            let (source_end, next_to_source, next_to_target) = if rightward {
                (Junction::Right, from + 1, to - 1)
            } else {
                (Junction::Left, from - 1, to + 1)
            };
            row[from] = if arrow.central_source {
                self.glyphs.central
            } else {
                self.junction(self.life[sender], source_end)
            };
            if arrow.central_target {
                row[to] = self.glyphs.central;
            }
            if let Some(head) = self.arrow_head(arrow.head, rightward) {
                row[next_to_target] = head;
            }
            if matches!(arrow.head, ArrowHead::Both)
                && let Some(head) = self.arrow_head(arrow.head, !rightward)
            {
                row[next_to_source] = head;
            }
            output.emit(&row, None);
            let paint_left = left + usize::from(to == left && !arrow.central_target);
            let width = right + 1 - paint_left - usize::from(to == right && !arrow.central_target);
            output.paint(paint_left, width, signal);
        }
    }

    fn junction(&self, life: Life, junction: Junction) -> char {
        let index = junction as usize;
        match life {
            Life::Alive(1) => self.glyphs.active_junctions[index],
            Life::Alive(level) if level > 1 => self.glyphs.stacked_junctions[index],
            _ => [
                self.glyphs.branch_right,
                self.glyphs.branch_left,
                self.glyphs.cross,
            ][index],
        }
    }

    fn arrow_head(&self, head: ArrowHead, rightward: bool) -> Option<char> {
        match (head, rightward) {
            (ArrowHead::None, _) => None,
            (ArrowHead::Cross, _) => Some(self.glyphs.cancel),
            (ArrowHead::Open, true) => Some(self.glyphs.open_right),
            (ArrowHead::Open, false) => Some(self.glyphs.open_left),
            (ArrowHead::Both | ArrowHead::Filled, true) => Some(self.glyphs.arrow_right),
            (ArrowHead::Both | ArrowHead::Filled, false) => Some(self.glyphs.arrow_left),
        }
    }

    fn draw_note(
        &self,
        output: &mut Rows<'_, '_>,
        depth: usize,
        placement: Placement,
        labels: &[Line<'_>],
    ) {
        let width = labels.iter().map(|label| label.width).max().unwrap_or(0) + BOX_PADDING;
        let (left, right) = match placement {
            Placement::Left(at) => (self.centers[at] - width - 1, self.centers[at] - 2),
            Placement::Right(at) => (self.centers[at] + 2, self.centers[at] + width + 1),
            Placement::Over(at) => {
                let left = self.centers[at] - width / 2;
                (left, left + width - 1)
            }
            Placement::Between(left, right) => (self.centers[left] - 1, self.centers[right] + 1),
        };
        let mut row = self.row(depth, true);
        self.horizontal(&mut row, left, right, self.box_corners.top);
        for (&column, life) in self.centers.iter().zip(&self.life) {
            if left < column && column < right && matches!(life, Life::Alive(_)) {
                row[column] = self.glyphs.branch_up;
            }
        }
        output.emit(&row, None);
        output.paint_box((left, right), self.theme.note, true);

        row[left + 1..right].fill(' ');
        row[left] = self.glyphs.vertical;
        row[right] = self.glyphs.vertical;
        for &label in labels {
            let start = left + (right - left + 1 - label.width) / 2;
            output.emit(&row, Some((start, label)));
            output.paint_box((left, right), self.theme.note, false);
        }

        self.horizontal(&mut row, left, right, self.box_corners.bottom);
        for (&column, life) in self.centers.iter().zip(&self.life) {
            if left < column && column < right && matches!(life, Life::Alive(_)) {
                row[column] = self.glyphs.branch_down;
            }
        }
        output.emit(&row, None);
        output.paint_box((left, right), self.theme.note, true);
    }

    fn draw_frame_heading(
        &self,
        output: &mut Rows<'_, '_>,
        depth: usize,
        keyword: &str,
        labels: &[Line<'_>],
        ends: (char, char),
    ) {
        let (left, _) = self.frame_bounds(depth);
        let heading = frame_heading(keyword, labels[0].text);
        self.draw_frame_edge(output, depth, &heading, ends);
        for &label in &labels[1..] {
            output.emit(
                &self.row(depth + 1, true),
                Some((left + keyword.len() + 3, label)),
            );
        }
    }

    fn draw_frame_edge(
        &self,
        output: &mut Rows<'_, '_>,
        depth: usize,
        heading: &str,
        ends: (char, char),
    ) {
        let (left, right) = self.frame_bounds(depth);
        let mut row = self.row(depth, false);
        self.horizontal(&mut row, left, right, ends);
        for (&column, life) in self.centers.iter().zip(&self.life) {
            if matches!(life, Life::Alive(_)) {
                row[column] = self.junction(*life, Junction::Cross);
            }
        }
        output.emit(&row, Some((left + 1, Line::new(heading))));
    }

    fn horizontal(&self, row: &mut [char], left: usize, right: usize, ends: (char, char)) {
        row[left + 1..right].fill(self.glyphs.horizontal);
        (row[left], row[right]) = ends;
    }
}

fn heading_width(keyword: &str, label: &Label<'_>, max_label_width: usize) -> usize {
    keyword.len() + label.fitted_width(max_label_width) + usize::from(!label.text.is_empty()) + 4
}

fn message_width(label: &Label<'_>, number: Option<&str>, limit: usize) -> usize {
    label.fitted_width(limit).max(number.map_or(0, str::len))
}

fn foreground(fg: Option<Rgb>) -> Paint {
    Paint { fg, bg: None }
}

fn background(bg: Option<Rgb>) -> Paint {
    Paint { fg: None, bg }
}

struct Rows<'a, 'label> {
    text: String,
    layer: Layer,
    color: bool,
    count: usize,
    groups: &'a [GroupBox<'label>],
    regions: Vec<((usize, usize), Rgb)>,
}

impl<'a, 'label> Rows<'a, 'label> {
    fn new(color: bool) -> Self {
        Self {
            text: String::new(),
            layer: Layer::default(),
            color,
            count: 0,
            groups: &[],
            regions: Vec::new(),
        }
    }

    fn start_region(&mut self, bounds: (usize, usize), fill: Option<Rgb>) {
        if self.color
            && let Some(fill) = fill
        {
            self.regions.push((bounds, fill));
        }
    }

    fn end_region(&mut self, bounds: (usize, usize)) {
        if self.regions.last().is_some_and(|region| region.0 == bounds) {
            self.regions.pop();
        }
    }

    fn emit<'line>(
        &mut self,
        row: &[char],
        overlays: impl IntoIterator<Item = (usize, Line<'line>)>,
    ) {
        if self.color {
            for group in self.groups {
                self.layer.paint(
                    group.left,
                    self.count,
                    group.right - group.left + 1,
                    background(group.fill),
                );
            }
            for &((left, right), fill) in &self.regions {
                self.layer
                    .paint(left, self.count, right - left + 1, background(Some(fill)));
            }
        }
        self.count += 1;

        let end = row
            .iter()
            .rposition(|&cell| cell != ' ')
            .map_or(0, |index| index + 1);
        let mut cursor = 0;
        for (start, label) in overlays {
            self.text.extend(&row[cursor..start]);
            self.text.push_str(label.text);
            cursor = start + label.width;
        }
        if cursor < end {
            self.text.extend(&row[cursor..end]);
        }
        self.text.push('\n');
    }

    fn paint(&mut self, left: usize, width: usize, paint: Paint) {
        if self.color {
            self.layer.paint(left, self.count - 1, width, paint);
        }
    }

    fn paint_box(&mut self, (left, right): (usize, usize), style: Style, edge: bool) {
        if edge {
            self.paint(left, right - left + 1, style.border());
        } else {
            self.paint(left, 1, style.border());
            self.paint(left + 1, right - left - 1, style.ink());
            self.paint(right, 1, style.border());
        }
    }

    fn finish(self) -> String {
        if self.color {
            self.layer.apply(&self.text)
        } else {
            self.text
        }
    }
}
