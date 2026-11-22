use std::collections::BTreeMap;

use super::layout::{self, CANVAS_LIMIT, MAX_CELLS};
use super::{
    Edge, EdgeStyle, Graph, Label, Node, NodeDeclaration, Panel, ParsedLink, Shape, Tip, Topology,
};
use crate::Options;
use crate::charts::shared::Direction;
use crate::color::Style;

#[derive(Default)]
struct Group {
    direction: Option<Direction>,
}

pub(super) struct Entry<'a> {
    id: Option<&'a str>,
    pub(super) node: Node<'a>,
    parent: Option<usize>,
    declared: bool,
    group: Option<Group>,
}

impl Entry<'_> {
    pub(super) fn is_group(&self) -> bool {
        self.group.is_some()
    }
}

#[derive(Default)]
pub(super) struct Document<'a> {
    entries: Vec<Entry<'a>>,
    ids: BTreeMap<&'a str, usize>,
    edge_ids: BTreeMap<&'a str, usize>,
    edges: Vec<Edge<'a>>,
    current: Option<usize>,
    direction: Option<Direction>,
    grouped: bool,
    styles: Styles<'a>,
}

#[derive(Default)]
struct Styles<'a> {
    definitions: BTreeMap<&'a str, Style>,
    assignments: BTreeMap<&'a str, Vec<&'a str>>,
    node_overrides: BTreeMap<&'a str, Style>,
    edge_overrides: BTreeMap<usize, Style>,
    default_edge: Style,
}

impl Styles<'_> {
    fn with_classes(&self, id: &str, base: Style) -> Style {
        self.assignments
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|name| self.definitions.get(name))
            .fold(base, |style, &class| style.overlay(class))
    }

    fn resolve(
        &self,
        entries: &mut [Entry<'_>],
        edges: &mut [Edge<'_>],
        edge_ids: &BTreeMap<&str, usize>,
    ) {
        let default_node = self.definitions.get("default").copied().unwrap_or_default();
        for entry in entries {
            entry.node.style = match entry.id {
                Some(id) => self
                    .with_classes(id, default_node)
                    .overlay(self.node_overrides.get(id).copied().unwrap_or_default()),
                None => default_node,
            };
        }
        for edge in edges.iter_mut() {
            edge.style = self.default_edge;
        }
        for (&id, &index) in edge_ids {
            edges[index].style = self.with_classes(id, self.default_edge);
        }
        for (&index, &style) in &self.edge_overrides {
            if let Some(edge) = edges.get_mut(index) {
                edge.style = edge.style.overlay(style);
            }
        }
    }
}

impl<'a> Document<'a> {
    pub(super) fn node(
        &mut self,
        id: &'a str,
        declaration: Option<NodeDeclaration<'a>>,
        standalone: bool,
    ) -> Result<usize, String> {
        let declared = standalone || declaration.is_some();
        if let Some(&index) = self.ids.get(id) {
            let entry = &mut self.entries[index];
            if entry.group.is_some() {
                if declaration.is_some() {
                    return Err(format!("subgraph {id} cannot be redeclared as a node"));
                }
                return Ok(index);
            }
            if self.current.is_some() && (entry.parent.is_none() || (!entry.declared && declared)) {
                entry.parent = self.current;
                entry.declared = declared;
            } else if entry.parent == self.current {
                entry.declared |= declared;
            }
            if let Some(declaration) = declaration {
                if let Some(label) = declaration.label {
                    entry.node.label = label;
                }
                if let Some(shape) = declaration.shape {
                    entry.node.shape = shape;
                }
            }
            return Ok(index);
        }
        let index = self.entries.len();
        self.ids.insert(id, index);
        self.entries.push(Entry {
            id: Some(id),
            node: Node::new(
                declaration
                    .as_ref()
                    .and_then(|node| node.label)
                    .map_or_else(|| Label::new(id), Ok)?,
                declaration
                    .and_then(|node| node.shape)
                    .unwrap_or(Shape::RECTANGLE),
            ),
            parent: self.current,
            declared,
            group: None,
        });
        Ok(index)
    }

    pub(super) fn open(&mut self, id: Option<&'a str>, title: Label<'a>) -> Result<(), String> {
        let entry = Entry {
            id,
            node: Node::new(title, Shape::RECTANGLE),
            parent: self.current,
            declared: true,
            group: Some(Group::default()),
        };
        let index = match id.and_then(|id| self.ids.get(id).copied()) {
            Some(index) => {
                if self.entries[index].group.is_some() {
                    return Err(format!("duplicate subgraph {}", title.text));
                }
                self.entries[index] = entry;
                index
            }
            None => {
                let index = self.entries.len();
                if let Some(id) = id {
                    self.ids.insert(id, index);
                }
                self.entries.push(entry);
                index
            }
        };
        self.current = Some(index);
        self.grouped = true;
        Ok(())
    }

    pub(super) fn close(&mut self) -> Result<(), String> {
        let index = self.current.ok_or("end without an open subgraph")?;
        self.current = self.entries[index].parent;
        Ok(())
    }

    pub(super) fn direction(&mut self, direction: Direction) -> Result<(), String> {
        if self.current.is_none() {
            self.direction = Some(direction);
            return Ok(());
        }
        let group = self
            .current
            .and_then(|index| self.entries[index].group.as_mut())
            .ok_or("missing current subgraph")?;
        group.direction = Some(direction);
        Ok(())
    }

    pub(super) fn finish(&self) -> Result<(), String> {
        if let Some(open) = self.current {
            return Err(format!(
                "missing end for subgraph {}",
                self.entries[open].node.label.text
            ));
        }
        Ok(())
    }

    pub(super) fn connect(
        &mut self,
        from: &[usize],
        to: &[usize],
        parsed: ParsedLink<'a>,
    ) -> Result<(), String> {
        // Every edge needs two distinct attachment cells and at least one cell between them.
        let total = from
            .len()
            .checked_mul(to.len())
            .and_then(|count| self.edges.len().checked_add(count))
            .filter(|&count| count <= MAX_CELLS / 3)
            .ok_or(CANVAS_LIMIT)?;
        self.edges.reserve(total - self.edges.len());
        if let Some(id) = parsed.id {
            // A named fan-out link identifies the last source's first edge.
            self.edge_ids.entry(id).or_insert(total - to.len());
        }
        for &from in from {
            for &to in to {
                self.edges.push(Edge {
                    from,
                    to,
                    label: parsed.label,
                    link: parsed.link,
                    style: Style::default(),
                });
            }
        }
        Ok(())
    }

    pub(super) fn is_edge_id(&self, id: &str) -> bool {
        self.edge_ids.contains_key(id)
    }

    pub(super) fn style_nodes(&mut self, targets: &[&'a str], style: Style) -> Result<(), String> {
        for &id in targets {
            if !self.ids.contains_key(id) {
                self.node(id, None, false)?;
            }
            let target = self.styles.node_overrides.entry(id).or_default();
            *target = target.overlay(style);
        }
        Ok(())
    }

    pub(super) fn define_classes(&mut self, names: &[&'a str], style: Style) {
        for &name in names {
            let class = self.styles.definitions.entry(name).or_default();
            *class = class.overlay(style);
        }
    }

    pub(super) fn assign_classes(
        &mut self,
        target: &'a str,
        classes: impl IntoIterator<Item = &'a str>,
    ) {
        self.styles
            .assignments
            .entry(target)
            .or_default()
            .extend(classes);
    }

    pub(super) fn style_edge(&mut self, index: Option<usize>, style: Style) {
        let target = match index {
            Some(index) => self.styles.edge_overrides.entry(index).or_default(),
            None => &mut self.styles.default_edge,
        };
        *target = target.overlay(style);
    }

    fn resolve_styles(&mut self, options: &Options) {
        if options.color {
            self.styles
                .resolve(&mut self.entries, &mut self.edges, &self.edge_ids);
            let config = &options.config;
            let nodes = Style {
                fill: config.theme_color("mainBkg"),
                stroke: config.theme_color("nodeBorder"),
                text: config.theme_color("nodeTextColor"),
            };
            let groups = Style {
                fill: config.theme_color("clusterBkg"),
                stroke: config.theme_color("clusterBorder"),
                text: config.theme_color("titleColor"),
            };
            for entry in &mut self.entries {
                let theme = if entry.group.is_some() { groups } else { nodes };
                entry.node.style = theme.overlay(entry.node.style);
            }
            let link = config.theme_color("defaultLinkColor");
            for edge in &mut self.edges {
                edge.style.stroke = edge.style.stroke.or(link);
            }
        }
    }

    pub(super) fn render(
        mut self,
        direction: Direction,
        options: &Options,
    ) -> Result<String, String> {
        let direction = self.direction.unwrap_or(direction);
        self.resolve_styles(options);
        if !self.grouped {
            return Graph {
                nodes: self.entries.into_iter().map(|entry| entry.node).collect(),
                edges: self.edges,
            }
            .render(direction, options, options.width);
        }
        let hierarchy = Hierarchy::new(&self.entries, direction)?;
        hierarchy.render(&self.entries, &self.edges, options)
    }

    pub(super) fn image(
        mut self,
        direction: Direction,
        options: &Options,
        scene: &mut crate::graphics::Scene,
    ) -> Result<(), String> {
        self.resolve_styles(options);
        let hierarchy = Hierarchy::new(&self.entries, self.direction.unwrap_or(direction))?;
        super::image::render(&self.entries, &self.edges, &hierarchy, options, scene)
    }
}

pub(super) struct Hierarchy {
    pub(super) root: usize,
    pub(super) scopes: Vec<usize>,
    pub(super) parents: Vec<usize>,
    pub(super) members: Vec<Vec<usize>>,
    pub(super) positions: Vec<usize>,
    depths: Vec<usize>,
    pub(super) directions: Vec<Direction>,
}

impl Hierarchy {
    fn new(entries: &[Entry<'_>], direction: Direction) -> Result<Self, String> {
        Self::from_parts(
            entries.len(),
            entries.iter().map(|entry| {
                (
                    entry.parent,
                    entry.group.is_some(),
                    entry.group.as_ref().and_then(|group| group.direction),
                )
            }),
            direction,
        )
    }

    pub(super) fn from_parts(
        root: usize,
        entries: impl Iterator<Item = (Option<usize>, bool, Option<Direction>)>,
        direction: Direction,
    ) -> Result<Self, String> {
        let mut members = vec![Vec::new(); root + 1];
        let mut positions = Vec::with_capacity(root);
        let mut parents = Vec::with_capacity(root);
        let mut groups = Vec::with_capacity(root);
        for (index, (parent, group, direction)) in entries.enumerate() {
            let scope = parent.unwrap_or(root);
            parents.push(scope);
            positions.push(members[scope].len());
            members[scope].push(index);
            groups.push((group, direction));
        }
        let mut scopes = vec![root];
        let mut depths = vec![0; root + 1];
        let mut directions = vec![direction; root + 1];
        let mut next = 0;
        while let Some(&scope) = scopes.get(next) {
            next += 1;
            for &index in &members[scope] {
                depths[index] = depths[scope] + 1;
                let (group, direction) = groups[index];
                if group {
                    let minimum_side = 2 * depths[index];
                    if minimum_side > MAX_CELLS / minimum_side {
                        return Err(CANVAS_LIMIT.into());
                    }
                    directions[index] = direction.unwrap_or(directions[scope]);
                    scopes.push(index);
                }
            }
        }
        Ok(Self {
            root,
            scopes,
            parents,
            members,
            positions,
            depths,
            directions,
        })
    }

    pub(super) fn project(&self, mut from: usize, mut to: usize) -> (usize, usize, usize) {
        // An edge belongs to the first scope where both endpoints can be drawn as peers.
        while self.parents[from] != self.parents[to] {
            if self.depths[from] > self.depths[to] {
                from = self.parents[from];
            } else if self.depths[to] > self.depths[from] {
                to = self.parents[to];
            } else {
                from = self.parents[from];
                to = self.parents[to];
            }
        }
        (self.parents[from], from, to)
    }

    fn render(
        &self,
        entries: &[Entry<'_>],
        edges: &[Edge<'_>],
        options: &Options,
    ) -> Result<String, String> {
        let mut referenced = vec![false; entries.len()];
        let mut routed = vec![Vec::new(); self.root + 1];
        for (index, edge) in edges.iter().enumerate() {
            let (scope, from, to) = self.project(edge.from, edge.to);
            if edge.link.style != EdgeStyle::Invisible {
                referenced[edge.from] |= from != edge.from;
                referenced[edge.to] |= to != edge.to;
            }
            routed[scope].push((index, from, to));
        }

        let draw = |branch_gap| {
            self.draw_scopes(entries, edges, options, &referenced, &routed, branch_gap)
        };
        let (compact, slots) = draw(0)?;
        let width = Panel::new(&compact).columns;
        if let Some(extra) = options.width.saturating_sub(width).checked_div(slots)
            && extra > 0
            && let Ok((expanded, _)) = draw(extra)
            && Panel::new(&expanded).columns <= options.width
        {
            return Ok(expanded.render());
        }
        Ok(compact.render())
    }

    fn draw_scopes(
        &self,
        entries: &[Entry<'_>],
        edges: &[Edge<'_>],
        options: &Options,
        referenced: &[bool],
        routed: &[Vec<(usize, usize, usize)>],
        branch_gap: usize,
    ) -> Result<(layout::Drawing, usize), String> {
        let mut remaining = MAX_CELLS;
        let mut branch_slots = 0;
        let mut bodies: Vec<_> = (0..=self.root)
            .map(|_| layout::Drawing::default())
            .collect();
        for &scope in self.scopes.iter().rev() {
            let mut nodes = Vec::new();
            for &index in &self.members[scope] {
                let entry = &entries[index];
                let mut node = entry.node;
                if entry.group.is_some() {
                    node.body = Some(Panel::new(&bodies[index]));
                }
                if referenced[index]
                    && let Some(id) = entry.id
                    && node.label.text != id
                {
                    node.port_name = Some(Label::new(id)?);
                }
                nodes.push(node);
            }

            if nodes.is_empty() {
                continue;
            }
            let mut annotation_bytes = 0;
            let annotations = routed[scope]
                .iter()
                .map(|&(index, from, to)| {
                    let edge = &edges[index];
                    if edge.link.style == EdgeStyle::Invisible {
                        return Ok(None);
                    }
                    let from = (from != edge.from)
                        .then_some(entries[edge.from].id)
                        .flatten();
                    let to = (to != edge.to).then_some(entries[edge.to].id).flatten();
                    if from.is_none() && to.is_none() {
                        return Ok(None);
                    }
                    let arrow = match (options.ascii, edge.link.start, edge.link.end) {
                        (false, Tip::Arrow, Tip::Arrow) => "↔",
                        (false, _, Tip::Arrow) => "→",
                        (false, Tip::Arrow, _) => "←",
                        (false, _, _) => "─",
                        (true, Tip::Arrow, Tip::Arrow) => "<->",
                        (true, _, Tip::Arrow) => "->",
                        (true, Tip::Arrow, _) => "<-",
                        (true, _, _) => "-",
                    };
                    let bytes = from.unwrap_or_default().len() + to.unwrap_or_default().len();
                    annotation_bytes += bytes + arrow.len() + 4;
                    if annotation_bytes > remaining * 4 {
                        return Err(CANVAS_LIMIT);
                    }
                    let note = match (from, to) {
                        (Some(from), Some(to)) => format!("[{from} {arrow} {to}]"),
                        (Some(from), None) => format!("[{from} {arrow}]"),
                        (None, Some(to)) => format!("[{arrow} {to}]"),
                        (None, None) => return Ok(None),
                    };
                    Ok(Some(note))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let local_edges = routed[scope]
                .iter()
                .zip(&annotations)
                .map(|(&(index, from, to), note)| {
                    let edge = &edges[index];
                    let label = match (edge.label, note) {
                        (Some(label), Some(note)) => Some(label.with_note(note)),
                        (None, Some(note)) => Some(Label::new(note)?),
                        (label, None) => label,
                    };
                    Ok(Edge {
                        from: self.positions[from],
                        to: self.positions[to],
                        label,
                        ..*edge
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let graph = Graph {
                nodes,
                edges: local_edges,
            };
            let topology = Topology::new(&graph);
            let body = layout::render_scoped(
                &graph,
                &topology,
                self.directions[scope],
                options,
                &mut remaining,
                branch_gap,
            )?;
            branch_slots += body.branch_slots;
            for &member in &self.members[scope] {
                bodies[member] = layout::Drawing::default();
            }
            bodies[scope] = body;
        }
        Ok((std::mem::take(&mut bodies[self.root]), branch_slots))
    }
}
