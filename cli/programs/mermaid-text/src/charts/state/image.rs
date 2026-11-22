use std::borrow::Cow;

use crate::Options;
use crate::charts::flowchart::{
    Outline,
    image::{Node, render_graph},
};
use crate::charts::shared::Direction;
use crate::color::Style;
use crate::graphics::Scene;

use super::{Diagram, Kind, Scope, Side, State, StateId, expand_tabs};

pub(super) fn render(
    diagram: &Diagram<'_>,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    let color = |key| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let cluster = Style {
        fill: color("clusterBkg"),
        stroke: color("clusterBorder"),
        text: color("titleColor"),
    };
    let note = Style {
        fill: color("noteBkgColor"),
        stroke: color("noteBorderColor"),
        text: color("noteTextColor"),
    };

    let mut parents = vec![None; diagram.regions.len()];
    let mut regions = Vec::new();
    for scope in &diagram.scopes {
        for &region in &scope.regions {
            parents[region] = if scope.regions.len() == 1 {
                scope.owner
            } else {
                let id = diagram.states.len() + regions.len();
                regions.push(Node {
                    lines: Vec::new(),
                    outline: Outline::FixedRectangle,
                    parent: scope.owner,
                    direction: Some(diagram.regions[region].direction),
                    style: cluster,
                    notes: Vec::new(),
                });
                Some(id)
            };
        }
    }

    let direction = |scope: &Scope| {
        if let [region] = scope.regions[..] {
            diagram.regions[region].direction
        } else {
            Direction::Down
        }
    };
    let mut nodes: Vec<_> = diagram
        .states
        .iter()
        .map(|state| {
            let (outline, child_direction, theme) = match (state.id, state.kind) {
                (StateId::Start(_), _) => (Outline::Initial, None, Style::default()),
                (StateId::End(_), _) => (Outline::Final, None, Style::default()),
                (_, Kind::Ordinary) => (Outline::Rectangle, None, Style::default()),
                (_, Kind::Composite(scope)) => (
                    Outline::Rectangle,
                    Some(direction(&diagram.scopes[scope])),
                    cluster,
                ),
                (_, Kind::Choice) => (Outline::Decision, None, Style::default()),
                (_, Kind::Fork | Kind::Join) => (
                    Outline::Synchronization {
                        vertical: matches!(
                            diagram.regions[state.region].direction,
                            Direction::Left | Direction::Right
                        ),
                    },
                    None,
                    Style::default(),
                ),
                (_, Kind::FloatingNote) => (Outline::Note, None, note),
            };
            Node {
                lines: lines(state),
                outline,
                parent: parents[state.region],
                direction: child_direction,
                style: if options.color {
                    theme.overlay(state.style)
                } else {
                    theme
                },
                notes: state
                    .notes
                    .iter()
                    .map(|note| {
                        (
                            matches!(note.side, Side::Left),
                            note.lines.iter().map(|line| expand_tabs(line)).collect(),
                        )
                    })
                    .collect(),
            }
        })
        .collect();
    nodes.extend(regions);
    if nodes.is_empty() {
        let label = "(empty)";
        scene.resize(scene.measure(label) + 32.0, 52.0)?;
        scene.text(16.0, 16.0, label, None);
        return Ok(());
    }
    let edges = diagram
        .regions
        .iter()
        .flat_map(|region| &region.edges)
        .chain(&diagram.cross_region)
        .map(|edge| (edge.from, edge.to, edge.label.map(expand_tabs)));
    render_graph(nodes, edges, direction(&diagram.scopes[0]), options, scene)
}

fn lines<'a>(state: &State<'a>) -> Vec<Cow<'a, str>> {
    let StateId::Named(id) = state.id else {
        return Vec::new();
    };
    if state.descriptions.is_empty() {
        return vec![Cow::Borrowed(id)];
    }
    state
        .descriptions
        .iter()
        .flat_map(|description| description.lines())
        .map(|line| expand_tabs(line.trim()))
        .collect()
}
