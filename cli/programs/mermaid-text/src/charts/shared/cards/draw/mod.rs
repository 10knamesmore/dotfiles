use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write;

use unicode_width::UnicodeWidthStr;

use crate::{Options, canvas::Canvas, charts::shared::Direction, color::Paint};

use super::{
    Diagram, Link, Relationship,
    card::{Card, Picture, columns, display_text, wrap},
    wire::{Axis, Captions, Port, Rect, Router, Wire, marker},
};

pub(super) mod image;

const CARD_GAP: usize = 4;

struct Component {
    nodes: Vec<usize>,
    edges: Vec<usize>,
}

struct Panel {
    size: (usize, usize),
    cards: Vec<(usize, Rect, Card)>,
    wires: Vec<(usize, Wire, bool)>,
    remaining: Vec<usize>,
}

impl Panel {
    fn picture(&self, layout: Layout<'_>) -> Result<Option<String>, String> {
        let Ok(mut canvas) = Canvas::new(self.size.0, self.size.1) else {
            return Ok(None);
        };
        for (_, rect, card) in &self.cards {
            card.paint(&mut canvas, rect.x, rect.y)?;
        }
        let (line, caption) = layout.wire_paints();
        for (_, wire, _) in &self.wires {
            wire.paint(&mut canvas, line, caption)?;
        }
        Ok(Some(canvas.finish().trim_matches('\n').to_owned()))
    }
}

#[derive(Clone, Copy)]
struct Layout<'a> {
    axis: Axis,
    reverse: bool,
    width: usize,
    options: &'a Options,
}

pub(super) fn render(diagram: &Diagram<'_>, options: &Options) -> Result<String, String> {
    let layout = Layout::new(diagram, options);
    let mut output = String::new();
    if let Some(title) = diagram.title {
        for line in wrap(title, layout.width) {
            let _ = writeln!(output, "{line}");
        }
    }
    for note in &diagram.notes {
        let note = display_text(note);
        for (index, line) in note.split("\\n").enumerate() {
            let prefix = if index == 0 { "Note: " } else { "      " };
            for part in wrap(line, layout.width - 6) {
                let _ = writeln!(output, "{prefix}{part}");
            }
        }
    }
    for component in components(diagram) {
        separate(&mut output);
        if component.edges.is_empty() {
            output
                .push_str(&Card::new(diagram, component.nodes[0], layout.width, options).picture());
            continue;
        }
        let mut remaining = component.edges.as_slice();
        let mut panel = None;
        if component.nodes.len() <= 64 && component.edges.len() <= 96 {
            let levels = levels(diagram, &component, layout.reverse);
            panel = layout.panel(diagram, &component.edges, &levels, false);
        }
        if let Some(panel) = &panel
            && let Some(picture) = panel.picture(layout)?
        {
            let _ = writeln!(output, "{picture}");
            remaining = &panel.remaining;
        } else if let [index] = component.edges[..]
            && component.nodes.len() == 2
        {
            output.push_str(&layout.vertical_pair(diagram, index, false));
            continue;
        } else {
            for (row, &node) in component.nodes.iter().enumerate() {
                if row > 0 {
                    separate(&mut output);
                }
                output.push_str(&Card::new(diagram, node, layout.width, options).picture());
            }
        }

        if !remaining.is_empty() {
            separate(&mut output);
            output.push_str("Connections\n");
        }
        for &index in remaining {
            separate(&mut output);
            let edge = &diagram.relationships[index];
            if matches!(layout.axis, Axis::Horizontal) && edge.left != edge.right {
                let [left, right] = if layout.reverse {
                    [edge.right, edge.left]
                } else {
                    [edge.left, edge.right]
                };
                if let Some(pair) =
                    layout.panel(diagram, &[index], &[vec![left], vec![right]], true)
                    && let Some(picture) = pair.picture(layout)?
                {
                    let _ = writeln!(output, "{picture}");
                    continue;
                }
            }
            output.push_str(&layout.vertical_pair(diagram, index, true));
        }
    }
    output.pop();
    Ok(output)
}

fn separate(output: &mut String) {
    if !output.is_empty() {
        output.push('\n');
    }
}

fn components(diagram: &Diagram<'_>) -> Vec<Component> {
    let mut neighbors = vec![Vec::new(); diagram.nodes.len()];
    for (index, edge) in diagram.relationships.iter().enumerate() {
        neighbors[edge.left].push((edge.right, index));
        neighbors[edge.right].push((edge.left, index));
    }
    let mut seen_node = vec![false; neighbors.len()];
    let mut seen_edge = vec![false; diagram.relationships.len()];
    let mut result = Vec::new();
    for start in 0..neighbors.len() {
        if std::mem::replace(&mut seen_node[start], true) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        while let Some(node) = queue.pop_front() {
            nodes.push(node);
            for &(next, edge) in &neighbors[node] {
                if !std::mem::replace(&mut seen_node[next], true) {
                    queue.push_back(next);
                }
                if !std::mem::replace(&mut seen_edge[edge], true) {
                    edges.push(edge);
                }
            }
        }
        nodes.sort_unstable();
        edges.sort_unstable();
        result.push(Component { nodes, edges });
    }
    result
}

fn levels(diagram: &Diagram<'_>, component: &Component, reverse: bool) -> Vec<Vec<usize>> {
    let map: BTreeMap<_, _> = component
        .nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();
    let count = component.nodes.len();
    let mut dependencies = vec![0; count];
    let mut successors = vec![Vec::new(); count];
    for &index in &component.edges {
        let edge = &diagram.relationships[index];
        let (source, target) = (map[&edge.left], map[&edge.right]);
        if source != target {
            dependencies[target] += 1;
            successors[source].push(target);
        }
    }
    let mut ranks = vec![0; count];
    let mut processed = vec![false; count];
    let mut ready: VecDeque<_> = (0..count).filter(|&node| dependencies[node] == 0).collect();
    for seed in 0..count {
        if ready.is_empty() && !processed[seed] {
            ready.push_back(seed);
        }
        while let Some(node) = ready.pop_front() {
            processed[node] = true;
            for &target in &successors[node] {
                if !processed[target] {
                    dependencies[target] -= 1;
                    ranks[target] = ranks[target].max(ranks[node] + 1);
                    if dependencies[target] == 0 {
                        ready.push_back(target);
                    }
                }
            }
        }
    }
    let mut levels = vec![Vec::new(); ranks.iter().max().copied().unwrap_or(0) + 1];
    for (node, rank) in component.nodes.iter().zip(ranks) {
        levels[rank].push(*node);
    }
    if reverse {
        levels.reverse();
    }
    levels
}

impl<'a> Layout<'a> {
    fn new(diagram: &Diagram<'_>, options: &'a Options) -> Self {
        let direction = diagram.direction.unwrap_or(Direction::Down);
        Self {
            axis: match direction {
                Direction::Down | Direction::Up => Axis::Vertical,
                Direction::Right | Direction::Left => Axis::Horizontal,
            },
            reverse: matches!(direction, Direction::Up | Direction::Left),
            width: options.width.clamp(20, 4096),
            options,
        }
    }

    fn wire_paints(self) -> (Paint, Paint) {
        if self.options.color {
            (
                Paint {
                    fg: self.options.theme_line(),
                    bg: None,
                },
                Paint {
                    fg: self.options.theme_style().text,
                    bg: None,
                },
            )
        } else {
            (Paint::default(), Paint::default())
        }
    }

    fn panel(
        self,
        diagram: &Diagram<'_>,
        edges: &[usize],
        levels: &[Vec<usize>],
        reference: bool,
    ) -> Option<Panel> {
        let horizontal = matches!(self.axis, Axis::Horizontal);
        let position: BTreeMap<_, _> = levels
            .iter()
            .enumerate()
            .flat_map(|(rank, nodes)| nodes.iter().map(move |&node| (node, rank)))
            .collect();
        let mut gaps = vec![3; levels.len().saturating_sub(1)];
        let mut edge_counts = vec![0_usize; gaps.len()];
        let mut caption_width = 0;
        for &index in edges {
            let edge = &diagram.relationships[index];
            let [a, b] = [position[&edge.left], position[&edge.right]];
            if a.abs_diff(b) == 1 {
                let rank = a.min(b);
                let label = self.label(diagram, edge, a > b);
                let label_width = label.lines().map(UnicodeWidthStr::width).max().unwrap_or(0);
                let count_lengths = edge
                    .ends
                    .map(|end| end.count.map_or(0, |text| text.width()));
                caption_width = caption_width
                    .max(label_width)
                    .max(count_lengths[0])
                    .max(count_lengths[1]);
                let gap = if horizontal {
                    let count_room = count_lengths.iter().sum::<usize>() + 3;
                    let caption = (label_width + 2).min((self.width / 3).max(7));
                    caption.max(count_room.min(18)).max(6)
                } else {
                    let caption_width = (self.width / 3).max(8);
                    let line_count = label
                        .lines()
                        .map(|line| line.width().div_ceil(caption_width).max(1))
                        .sum::<usize>()
                        + count_lengths
                            .iter()
                            .map(|width| width.div_ceil(caption_width))
                            .sum::<usize>();
                    line_count.clamp(3, 20)
                };
                gaps[rank] = gaps[rank].max(gap);
                edge_counts[rank] += 1;
            }
        }
        for (gap, count) in gaps.iter_mut().zip(edge_counts) {
            *gap += count.saturating_sub(1).min(7) * 2;
        }
        let cards_in_row = levels.iter().map(Vec::len).max().unwrap_or(1);
        let budget = if horizontal {
            self.width.saturating_sub(gaps.iter().sum::<usize>()) / levels.len()
        } else {
            self.width.saturating_sub(CARD_GAP * (cards_in_row - 1)) / cards_in_row
        };
        let make_card = if reference {
            Card::reference
        } else {
            Card::new
        };
        let cards: Vec<Vec<_>> = levels
            .iter()
            .map(|nodes| {
                nodes
                    .iter()
                    .map(|&index| make_card(diagram, index, budget.max(12), self.options))
                    .collect()
            })
            .collect();
        let measure = |card: &Card| {
            if horizontal {
                (card.width, card.height())
            } else {
                (card.height(), card.width)
            }
        };
        let sizes: Vec<_> = cards
            .iter()
            .map(|band| {
                let along = band.iter().map(|card| measure(card).0).max().unwrap_or(0);
                let across = band.iter().map(|card| measure(card).1).sum::<usize>()
                    + CARD_GAP * band.len().saturating_sub(1);
                (along, across)
            })
            .collect();
        let across = sizes.iter().map(|size| size.1).max().unwrap_or(1);
        let across = if horizontal {
            across + 2
        } else if caption_width > 0 {
            across.max((2 * (caption_width + 3)).clamp(20, self.width + 6))
        } else {
            across
        };
        let along = sizes.iter().map(|size| size.0).sum::<usize>() + gaps.iter().sum::<usize>();
        let size = self.axis.xy(along, across);
        if size.0 > self.width + 6 {
            return None;
        }
        let mut rects = Vec::new();
        let mut nodes = BTreeMap::new();
        let mut placements = Vec::new();
        let mut start = 0;
        let mut centers = Vec::new();
        for (rank, band) in cards.into_iter().enumerate() {
            let (along, band_cross) = sizes[rank];
            let mut cross = if horizontal && cards_in_row == 1 {
                1
            } else {
                (across - band_cross) / 2
            };
            for (card, &node) in band.into_iter().zip(&levels[rank]) {
                let (thickness, span) = measure(&card);
                let offset = if horizontal {
                    (along - thickness) / 2
                } else {
                    0
                };
                let (x, y) = self.axis.xy(start + offset, cross);
                let rect = Rect {
                    x,
                    y,
                    width: card.width,
                    height: card.height(),
                };
                nodes.insert(node, rects.len());
                rects.push(rect);
                placements.push((node, rect, card));
                cross += span + CARD_GAP;
            }
            start += along;
            if let Some(&gap) = gaps.get(rank) {
                centers.push((start, gap));
                start += gap;
            }
        }
        let mut router = Router::new(self.axis, size, &rects, self.options.ascii);
        let mut remaining = Vec::new();
        let mut wires = Vec::new();
        for &index in edges {
            let edge = &diagram.relationships[index];
            let [a, b] = [position[&edge.left], position[&edge.right]];
            // A route stays between neighboring bands; crossing a card band would hide its members.
            if a.abs_diff(b) != 1 {
                remaining.push(index);
                continue;
            }
            let (from, to, ends) = if a < b {
                (edge.left, edge.right, edge.ends)
            } else {
                (edge.right, edge.left, [edge.ends[1], edge.ends[0]])
            };
            let (_, first, first_card) = &placements[nodes[&from]];
            let (_, last, last_card) = &placements[nodes[&to]];
            let label = self.label(diagram, edge, a > b);
            if label.width() > 256
                || ends
                    .iter()
                    .any(|end| end.count.is_some_and(|count| count.width() > 96))
            {
                remaining.push(index);
                continue;
            }
            let captions = Captions {
                ends,
                label: &label,
            };
            let mut candidates: Vec<_> = self
                .ports(*first, first_card, *last, true)
                .into_iter()
                .enumerate()
                .flat_map(|(i, a)| {
                    self.ports(*last, last_card, *first, false)
                        .into_iter()
                        .enumerate()
                        .map(move |(j, b)| {
                            (
                                (
                                    a.across.abs_diff(b.across) + i + j,
                                    a.across.abs_diff(b.across),
                                ),
                                a,
                                b,
                            )
                        })
                })
                .collect();
            candidates.sort_by_key(|&(cost, _, _)| cost);
            let (start, gap) = centers[a.min(b)];
            let lanes: Vec<_> = near(start..start + gap, start + gap / 2).collect();
            let wire = candidates.into_iter().find_map(|(_, from, to)| {
                lanes
                    .iter()
                    .find_map(|&bend| router.route(from, to, bend, edge.link, &captions))
            });
            if let Some(wire) = wire {
                wires.push((index, wire, a > b));
            } else {
                remaining.push(index);
            }
        }
        if remaining.len() == edges.len() {
            return None;
        }
        Some(Panel {
            size,
            cards: placements,
            wires,
            remaining,
        })
    }

    fn ports(self, rect: Rect, card: &Card, other: Rect, start: bool) -> Vec<Port> {
        let (along, lo, hi, preferred) = match self.axis {
            Axis::Vertical => {
                let along = if start {
                    rect.y + rect.height - 1
                } else {
                    rect.y
                };
                (
                    along,
                    rect.x + 1,
                    rect.x + rect.width - 1,
                    other.x + other.width / 2,
                )
            }
            Axis::Horizontal => {
                let along = if start {
                    rect.x + rect.width - 1
                } else {
                    rect.x
                };
                (along, rect.y + 1, rect.y + rect.height - 1, other.y + 1)
            }
        };
        near(lo..hi, preferred.clamp(lo, hi - 1))
            .filter(|&across| {
                matches!(self.axis, Axis::Vertical) || card.side_port(across - rect.y)
            })
            .take(8)
            .map(|across| Port { along, across })
            .collect()
    }

    fn label(self, diagram: &Diagram<'_>, edge: &Relationship<'_>, reversed: bool) -> String {
        let label = display_text(edge.label.unwrap_or("").trim());
        if !label.is_empty() && edge.directional_label && reversed {
            format!("{}: {label}", diagram.nodes[edge.left].name.written)
        } else {
            label.into_owned()
        }
    }

    fn vertical_pair(self, diagram: &Diagram<'_>, index: usize, reference: bool) -> String {
        let edge = &diagram.relationships[index];
        let make_card = if reference {
            Card::reference
        } else {
            Card::new
        };
        let (top, bottom, ends) = if self.reverse {
            (edge.right, edge.left, [edge.ends[1], edge.ends[0]])
        } else {
            (edge.left, edge.right, edge.ends)
        };
        let top = make_card(diagram, top, self.width, self.options);
        let bottom = make_card(diagram, bottom, self.width, self.options);
        let cross = top.width.max(bottom.width) / 2;
        let ascii = self.options.ascii;
        let [out, enter, line] = match (ascii, edge.link) {
            (true, Link::Solid) => ['+', '+', '|'],
            (true, Link::Dashed) => ['+', '+', ':'],
            (_, Link::Solid) => ['┬', '┴', '│'],
            (_, Link::Dashed) => ['┬', '┴', '┊'],
        };
        let label = self.label(diagram, edge, self.reverse);
        let caption_width = self.width.saturating_sub(cross + 3).max(9);
        let mark_from = marker(ends[0].mark, Axis::Vertical, true, ascii);
        let mark_to = marker(ends[1].mark, Axis::Vertical, false, ascii);
        let mut from = ends[0]
            .count
            .map(|text| wrap(text, caption_width))
            .unwrap_or_default();
        let mut to = ends[1]
            .count
            .map(|text| wrap(text, caption_width))
            .unwrap_or_default();
        if from.is_empty() {
            from.push(String::new());
        }
        if to.is_empty() {
            to.push(String::new());
        }
        let mut picture = Picture::default();
        top.append(
            &mut picture,
            cross - top.width / 2,
            Some((top.height() - 1, top.width / 2, out)),
        );
        let (line_paint, caption_paint) = self.wire_paints();
        let mut row = |stroke: &str, text: &str| {
            let mut line = format!("{:cross$}{stroke}", "");
            picture.paint(cross, stroke.width(), line_paint);
            if !text.is_empty() {
                let _ = write!(
                    line,
                    "{:pad$}{text}",
                    "",
                    pad = 3_usize.saturating_sub(stroke.width())
                );
                picture.paint(cross + stroke.width().max(3), columns(text), caption_paint);
            }
            picture.line(&line);
        };
        let stroke = line.to_string();
        for (index, text) in from.iter().enumerate() {
            row(
                if index == 0 && !mark_from.is_empty() {
                    mark_from
                } else {
                    &stroke
                },
                text,
            );
        }
        for text in wrap(&label, caption_width)
            .iter()
            .filter(|line| !line.is_empty())
        {
            row(&stroke, text);
        }
        for (index, text) in to.iter().enumerate() {
            row(
                if index + 1 == to.len() && !mark_to.is_empty() {
                    mark_to
                } else {
                    &stroke
                },
                text,
            );
        }
        bottom.append(
            &mut picture,
            cross - bottom.width / 2,
            Some((0, bottom.width / 2, enter)),
        );
        picture.finish()
    }
}

fn near(range: std::ops::Range<usize>, pivot: usize) -> impl Iterator<Item = usize> {
    (0..range.len())
        .flat_map(move |delta| {
            [
                pivot.checked_sub(delta),
                (delta > 0).then_some(pivot + delta),
            ]
            .into_iter()
            .flatten()
        })
        .filter(move |point| range.contains(point))
}
