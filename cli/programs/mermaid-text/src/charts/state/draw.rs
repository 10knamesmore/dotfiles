use std::borrow::Cow;
use std::fmt::Write;

use crate::Options;
use crate::charts::flowchart::Graph;
use crate::charts::shared::Direction;

use super::{Diagram, Edge, Kind, Region, Side, State, StateId, expand_tabs};

pub(super) fn draw(diagram: &Diagram<'_>, options: &Options) -> Result<String, String> {
    let mut positions = vec![0; diagram.states.len()];
    for region in &diagram.regions {
        for (position, &state) in region.states.iter().enumerate() {
            positions[state] = position;
        }
    }
    let mut references: Vec<_> = diagram
        .states
        .iter()
        .map(|state| !state.notes.is_empty())
        .collect();
    for edge in &diagram.cross_region {
        references[edge.from] = true;
        references[edge.to] = true;
    }
    let mut writer = Writer {
        states: &diagram.states,
        positions,
        references,
        options,
        output: String::new(),
    };
    for scope in &diagram.scopes {
        if let Some(owner) = scope.owner {
            let title = label(&diagram.states[owner], options.ascii);
            writeln!(writer.output, "\n{title}").unwrap();
        }
        for (index, &region) in scope.regions.iter().enumerate() {
            if scope.regions.len() > 1 {
                if index > 0 {
                    writer.output.push('\n');
                }
                writeln!(writer.output, "Parallel region {}", index + 1).unwrap();
            }
            writer.region(&diagram.regions[region])?;
        }
    }
    if !diagram.cross_region.is_empty() {
        writer.output.push_str("\nCross-region transitions:\n");
        for edge in &diagram.cross_region {
            writer.transition(edge, diagram.regions[0].direction);
        }
    }
    writer.output.pop();
    Ok(writer.output)
}

struct Writer<'a, 'source> {
    states: &'a [State<'source>],
    positions: Vec<usize>,
    references: Vec<bool>,
    options: &'a Options,
    output: String,
}

impl Writer<'_, '_> {
    fn region(&mut self, region: &Region<'_>) -> Result<(), String> {
        if region.states.is_empty() {
            self.output.push_str("(empty)\n");
            return Ok(());
        }
        let nodes: Vec<_> = region
            .states
            .iter()
            .map(|&id| {
                let state = &self.states[id];
                let label = label(state, self.options.ascii);
                let label = if self.references[id] {
                    format!("[{}] {label}", id + 1)
                } else {
                    label
                };
                let configurable_corners = matches!(state.id, StateId::Named(_))
                    && matches!(
                        state.kind,
                        Kind::Ordinary | Kind::Composite(_) | Kind::FloatingNote
                    );
                (label, configurable_corners)
            })
            .collect();
        let edge_labels: Vec<_> = region
            .edges
            .iter()
            .map(|edge| edge.label.map(expand_tabs))
            .collect();
        let mut graph = Graph::from_parts(
            nodes
                .iter()
                .map(|(label, corners)| (label.as_str(), *corners)),
            region.edges.iter().zip(&edge_labels).map(|(edge, label)| {
                (
                    self.positions[edge.from],
                    self.positions[edge.to],
                    label.as_deref(),
                )
            }),
        )?;
        if self.options.color {
            for (position, &id) in region.states.iter().enumerate() {
                graph.set_node_style(position, self.states[id].style);
            }
        }
        self.output
            .push_str(&graph.render(region.direction, self.options, 0)?);
        self.output.push('\n');
        for &state in &region.states {
            for note in &self.states[state].notes {
                let side = match note.side {
                    Side::Left => "left",
                    Side::Right => "right",
                };
                writeln!(self.output, "Note [{}] ({side}):", state + 1).unwrap();
                for line in &note.lines {
                    if !line.is_empty() {
                        write!(self.output, "  {}", expand_tabs(line)).unwrap();
                    }
                    self.output.push('\n');
                }
            }
        }
        Ok(())
    }

    fn transition(&mut self, edge: &Edge<'_>, direction: Direction) {
        let (left, arrow, right) = match (direction, self.options.ascii) {
            (Direction::Left, true) => (edge.to, "<--", edge.from),
            (Direction::Left, false) => (edge.to, "◀──", edge.from),
            (_, true) => (edge.from, "-->", edge.to),
            (_, false) => (edge.from, "──▶", edge.to),
        };
        write!(self.output, "  [{}] {arrow} [{}]", left + 1, right + 1).unwrap();
        if let Some(label) = edge.label {
            write!(self.output, ": {}", expand_tabs(label)).unwrap();
        }
        self.output.push('\n');
    }
}

fn label(state: &State<'_>, ascii: bool) -> String {
    let id = match state.id {
        StateId::Start(_) => return if ascii { "(*) Start" } else { "● Start" }.into(),
        StateId::End(_) => return if ascii { "(@) End" } else { "◉ End" }.into(),
        StateId::Named(id) => id,
    };
    let mut label = match (state.kind, ascii) {
        (Kind::Ordinary, _) => String::new(),
        (Kind::Composite(index), true) => format!("[C{index}] "),
        (Kind::Composite(index), false) => format!("▣{index} "),
        (Kind::Choice, true) => "<> Choice: ".into(),
        (Kind::Choice, false) => "◇ Choice: ".into(),
        (Kind::Fork, true) => "=== Fork: ".into(),
        (Kind::Fork, false) => "━ Fork: ".into(),
        (Kind::Join, true) => "=== Join: ".into(),
        (Kind::Join, false) => "━ Join: ".into(),
        (Kind::FloatingNote, _) => "Note: ".into(),
    };
    if let Some((title, details)) = state.descriptions.split_first() {
        label.push_str(&node_text(title));
        for description in details {
            label.push_str(if ascii { " / " } else { " · " });
            label.push_str(&node_text(description));
        }
    } else {
        label.push_str(id);
    }
    label
}

fn node_text(text: &str) -> Cow<'_, str> {
    if text.contains('\n') {
        Cow::Owned(
            text.lines()
                .map(|line| expand_tabs(line.trim()))
                .collect::<Vec<_>>()
                .join(" "),
        )
    } else {
        expand_tabs(text)
    }
}
