use std::collections::{BTreeSet, VecDeque};

use rustc_hash::FxHashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{Diagram, Node, Palette, Side, wires::Drawing};
use crate::{Options, graphics::Scene};

mod image;
pub(super) use image::image;

#[derive(Default, Clone, Copy)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let text = text
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace('\t', " ");
    for line in text.split('\n') {
        let mut row = String::new();
        let mut used = 0;
        for word in line.split_word_bounds() {
            let len = word.width();
            if used > 0 && used + len > width {
                lines.push(row.trim_end().into());
                row.clear();
                used = 0;
                if word.trim().is_empty() {
                    continue;
                }
            }
            for grapheme in word.graphemes(true) {
                let w = grapheme.width();
                if used > 0 && used + w > width {
                    lines.push(std::mem::take(&mut row));
                    used = 0;
                }
                row.push_str(grapheme);
                used += w;
            }
        }
        if !row.is_empty() || line.is_empty() {
            lines.push(row.trim_end().to_owned());
        }
    }
    lines
}

fn caption(node: &Node, ascii: bool, width: usize, image: bool) -> Vec<String> {
    let icon = if image {
        None
    } else {
        match node.icon.to_ascii_lowercase().as_str() {
            "cloud" => Some(("☁", "~")),
            "server" => Some(("▤", "=")),
            "database" => Some(("▥", "#")),
            "disk" => Some(("◉", "o")),
            "internet" => Some(("◎", "@")),
            _ if node.kind == "junction" => Some(("◆", "*")),
            _ => None,
        }
    };
    let title = if let Some((u, a)) = icon {
        format!("{} {}", if ascii { a } else { u }, node.title)
    } else {
        node.title.clone()
    };
    let mut lines = wrap(&title, width);
    let mut meta = vec![node.kind.as_str()];
    if !node.icon.is_empty() && node.icon != node.title {
        meta.push(&node.icon);
    }
    if node.id != node.title {
        meta.push(&node.id);
    }
    lines.extend(wrap(&meta.join(if ascii { " | " } else { " · " }), width));
    for detail in &node.details {
        lines.extend(wrap(detail, width));
    }
    lines
}

struct Frame {
    node: Option<usize>,
    rect: Rect,
    text: Vec<String>,
    children: Vec<Frame>,
}

struct Layout<'a> {
    diagram: &'a Diagram,
    children: Vec<Vec<usize>>,
    align: Vec<Vec<(bool, Vec<usize>)>>,
    options: &'a Options,
    scene: Option<&'a Scene>,
}

const GAP_X: usize = 6;
const GAP_Y: usize = 4;

impl Layout<'_> {
    fn frame(&self, node: Option<usize>, budget: usize) -> Result<Frame, String> {
        let root = self.diagram.nodes.len();
        let mut children = Vec::new();
        let text = node
            .map(|i| {
                caption(
                    &self.diagram.nodes[i],
                    self.options.ascii,
                    budget.saturating_sub(4).clamp(16, 38),
                    self.scene.is_some(),
                )
            })
            .unwrap_or_default();
        let text_w = text
            .iter()
            .map(|s| {
                self.scene
                    .map_or_else(|| s.width(), |scene| image::columns(scene, s))
            })
            .max()
            .unwrap_or(0);
        let ids = &self.children[node.unwrap_or(root)];
        for &i in ids {
            children.push(self.frame(Some(i), budget.saturating_sub(6))?);
        }
        let mut grid = Vec::new();
        if !ids.is_empty() {
            let align = &self.align[node.unwrap_or(root)];
            if !align.is_empty() {
                grid = aligned_cells(ids, align)?;
            } else {
                let local: FxHashMap<_, _> =
                    ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
                let mut relations = Vec::new();
                for edge in &self.diagram.edges {
                    let (Some(&a), Some(&b)) = (
                        local.get(&self.diagram.endpoint(&edge.from)?),
                        local.get(&self.diagram.endpoint(&edge.to)?),
                    ) else {
                        continue;
                    };
                    if a == b {
                        continue;
                    }
                    if let Some(side) = edge.from.side
                        && edge.to.side == Some(side.opposite())
                    {
                        let (a, b) = if matches!(side, Side::Left | Side::Top) {
                            (b, a)
                        } else {
                            (a, b)
                        };
                        relations.push((a, b, matches!(side, Side::Top | Side::Bottom)));
                    }
                }
                let mut order = sorted(ids.len(), relations.iter().map(|&(a, b, _)| (a, b)))
                    .unwrap_or_else(|| (0..ids.len()).collect());
                if matches!(self.diagram.direction, Some(Side::Left | Side::Top)) {
                    order.reverse();
                }
                let mut row = 0;
                let mut members = Vec::new();
                let mut used = 0;
                let mut groups = 0;
                let mut shapes = 0;
                for i in order {
                    let group = self.diagram.nodes[ids[i]].boundary;
                    let forced = relations
                        .iter()
                        .any(|&(a, b, vertical)| vertical && b == i && members.contains(&a));
                    if !members.is_empty()
                        && (forced
                            || matches!(self.diagram.direction, Some(Side::Top | Side::Bottom))
                            || used + GAP_X + children[i].rect.w > budget.saturating_sub(6)
                            || (group
                                && groups >= self.diagram.groups_per_row.unwrap_or(usize::MAX))
                            || (!group
                                && shapes >= self.diagram.shapes_per_row.unwrap_or(usize::MAX)))
                    {
                        row += 1;
                        members.clear();
                        used = 0;
                        shapes = 0;
                        groups = 0;
                    }
                    grid.push((i, members.len(), row));
                    used += children[i].rect.w + if members.is_empty() { 0 } else { GAP_X };
                    members.push(i);
                    groups += usize::from(group);
                    shapes += usize::from(!group);
                }
            }
        }
        let cols = grid.iter().map(|&(_, x, _)| x + 1).max().unwrap_or(0);
        let rows = grid.iter().map(|&(_, _, y)| y + 1).max().unwrap_or(0);
        let mut widths = vec![0; cols];
        let mut heights = vec![0; rows];
        for &(i, x, y) in &grid {
            widths[x] = widths[x].max(children[i].rect.w);
            heights[y] = heights[y].max(children[i].rect.h);
        }
        fn offsets(sizes: &[usize], gap: usize) -> (Vec<usize>, usize) {
            let mut next = 0;
            let offsets = sizes
                .iter()
                .map(|&s| {
                    let n = next;
                    next += s + gap;
                    n
                })
                .collect();
            (offsets, next.saturating_sub(gap))
        }
        let (xs, w) = offsets(&widths, GAP_X);
        let (ys, h) = offsets(&heights, GAP_Y);
        let inset = if children.is_empty() { 2 } else { 3 };
        let top = if node.is_none() { 2 } else { text.len() + 2 };
        for (i, x, y) in grid {
            children[i].rect.x = inset + xs[x] + (widths[x] - children[i].rect.w) / 2;
            children[i].rect.y = top + ys[y];
        }
        let rect = Rect {
            x: 0,
            y: 0,
            w: (text_w + 4).max(w + inset * 2),
            h: if children.is_empty() {
                text.len() + 2
            } else {
                top + h + 2
            },
        };
        Ok(Frame {
            node,
            rect,
            text,
            children,
        })
    }
}

fn sorted(count: usize, edges: impl Iterator<Item = (usize, usize)>) -> Option<Vec<usize>> {
    let mut degree = vec![0; count];
    let mut next = vec![Vec::new(); count];
    for (a, b) in edges {
        next[a].push(b);
        degree[b] += 1;
    }
    let mut ready: BTreeSet<_> = (0..count).filter(|&i| degree[i] == 0).collect();
    let mut order = Vec::new();
    while let Some(i) = ready.pop_first() {
        order.push(i);
        for &j in &next[i] {
            degree[j] -= 1;
            if degree[j] == 0 {
                ready.insert(j);
            }
        }
    }
    (order.len() == count).then_some(order)
}

fn aligned_cells(
    ids: &[usize],
    align: &[(bool, Vec<usize>)],
) -> Result<Vec<(usize, usize, usize)>, String> {
    let local: FxHashMap<_, _> = ids.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    let constraints: Vec<_> = align
        .iter()
        .map(|(row, names)| (*row, names.iter().map(|s| local[s]).collect::<Vec<_>>()))
        .collect();
    fn root(set: &[usize], mut i: usize) -> usize {
        while set[i] != i {
            i = set[i];
        }
        i
    }
    fn coordinates(
        component: &[usize],
        constraints: &[(bool, Vec<usize>)],
        x: bool,
    ) -> Result<Vec<usize>, String> {
        let n = component.len();
        let mut set: Vec<_> = (0..n).collect();
        for (_, v) in constraints.iter().filter(|(row, _)| *row != x) {
            for pair in v.windows(2) {
                let a = root(&set, pair[0]);
                let b = root(&set, pair[1]);
                set[b] = a;
            }
        }
        let mut next = vec![Vec::new(); n];
        for (_, v) in constraints.iter().filter(|(row, _)| *row == x) {
            for pair in v.windows(2) {
                next[root(&set, pair[0])].push(root(&set, pair[1]));
            }
        }
        let order = sorted(
            n,
            next.iter()
                .enumerate()
                .flat_map(|(a, v)| v.iter().map(move |&b| (a, b))),
        )
        .ok_or("conflicting align order")?;
        let mut pos = vec![0; n];
        let mut slots = vec![0; n];
        for i in order {
            if i == set[i] {
                let c = root(component, i);
                pos[i] = slots[c];
                slots[c] += 1;
            }
        }
        Ok((0..n).map(|i| pos[root(&set, i)]).collect())
    }
    let mut component: Vec<_> = (0..ids.len()).collect();
    for (_, v) in &constraints {
        for pair in v.windows(2) {
            let a = root(&component, pair[0]);
            let b = root(&component, pair[1]);
            component[b] = a;
        }
    }
    let xs = coordinates(&component, &constraints, true)?;
    let ys = coordinates(&component, &constraints, false)?;
    let mut bottom = FxHashMap::<usize, usize>::default();
    for (i, &y) in ys.iter().enumerate() {
        let h = bottom.entry(root(&component, i)).or_default();
        *h = (*h).max(y + 1);
    }
    let mut offsets = FxHashMap::default();
    let mut next = 0;
    let mut occupied = BTreeSet::new();
    (0..ids.len())
        .map(|i| {
            let c = root(&component, i);
            let dy = *offsets.entry(c).or_insert_with(|| {
                let y = next;
                next += bottom[&c];
                y
            });
            let (x, y) = (xs[i], ys[i] + dy);
            if !occupied.insert((x, y)) {
                return Err("align constraints place two elements in the same cell".into());
            }
            Ok((i, x, y))
        })
        .collect()
}

impl Frame {
    // Each bound means that the far coordinate must be at least `distance`
    // beyond the near one. Leaf boxes keep their measured size; boundaries
    // can expand when alignment continues through a different group.
    fn bounds(
        &self,
        root: usize,
        x: bool,
        graph: &mut Vec<Vec<(usize, isize)>>,
        sizes: &mut [usize],
    ) {
        let i = self.node.unwrap_or(root);
        let size = if x { self.rect.w } else { self.rect.h };
        sizes[i] = size;
        graph[2 * i].push((2 * i + 1, size as isize));
        if self.children.is_empty() {
            graph[2 * i + 1].push((2 * i, -(size as isize)));
        }
        let mut children: Vec<_> = self.children.iter().collect();
        children.sort_by_key(|child| (child.rect.y, child.rect.x));
        let mut previous_row: Option<usize> = None;
        for row in children.chunk_by(|a, b| a.rect.y == b.rect.y) {
            let mut right: Option<usize> = None;
            let bottom = graph.len();
            if !x {
                graph.push(Vec::new());
            }
            for child in row {
                let j = child.node.unwrap();
                let offset = if x { child.rect.x } else { child.rect.y };
                graph[2 * i].push((2 * j, offset as isize));
                graph[2 * j + 1].push((2 * i + 1, if x { 3 } else { 2 }));
                if x {
                    if let Some(prev) = right {
                        graph[prev].push((2 * j, GAP_X as isize));
                    }
                    right = Some(2 * j + 1);
                } else {
                    if let Some(prev) = previous_row {
                        graph[prev].push((2 * j, GAP_Y as isize));
                    }
                    graph[2 * j + 1].push((bottom, 0));
                }
                child.bounds(root, x, graph, sizes);
            }
            previous_row = Some(bottom);
        }
    }

    fn position(&mut self, root: usize, parent: usize, x: bool, coordinates: &[usize]) {
        let i = 2 * self.node.unwrap_or(root);
        let start = coordinates[i];
        let size = coordinates[i + 1] - start;
        if x {
            self.rect.x = start - parent;
            self.rect.w = size;
        } else {
            self.rect.y = start - parent;
            self.rect.h = size;
        }
        for child in &mut self.children {
            child.position(root, start, x, coordinates);
        }
    }

    fn align(&mut self, d: &Diagram) -> Result<(), String> {
        let root = d.nodes.len();
        for x in [true, false] {
            let mut graph = vec![Vec::new(); 2 * (root + 1)];
            let mut sizes = vec![0; root + 1];
            self.bounds(root, x, &mut graph, &mut sizes);
            for (row, ids) in &d.align {
                for pair in ids.windows(2) {
                    let a = d.ids[&pair[0]];
                    let b = d.ids[&pair[1]];
                    if x != *row {
                        let delta = (sizes[a] / 2) as isize - (sizes[b] / 2) as isize;
                        graph[2 * a].push((2 * b, delta));
                        graph[2 * b].push((2 * a, -delta));
                    } else {
                        graph[2 * a + 1].push((2 * b, if x { GAP_X } else { GAP_Y } as isize));
                    }
                }
            }
            let mut pos = vec![0isize; graph.len()];
            let mut pending = VecDeque::from_iter(0..graph.len());
            let mut queued = vec![true; graph.len()];
            let mut depth = vec![0; graph.len()];
            let mut work = 0usize;
            while let Some(i) = pending.pop_front() {
                queued[i] = false;
                work += graph[i].len();
                if work > 4_000_000 {
                    return Err("align exceeds the constraint layout budget".into());
                }
                for &(j, distance) in &graph[i] {
                    if pos[j] < pos[i] + distance {
                        pos[j] = pos[i] + distance;
                        depth[j] = depth[i] + 1;
                        if depth[j] >= graph.len() {
                            return Err(
                                "align constraints conflict with containment or member order"
                                    .into(),
                            );
                        }
                        if !queued[j] {
                            pending.push_back(j);
                            queued[j] = true;
                        }
                    }
                }
            }
            self.position(
                root,
                0,
                x,
                &pos.into_iter().map(|n| n as usize).collect::<Vec<_>>(),
            );
        }
        Ok(())
    }

    fn paint(
        &self,
        d: &Diagram,
        palette: &Palette,
        drawing: &mut Drawing,
        boxes: &mut [Rect],
        x: usize,
        y: usize,
    ) -> Result<(), String> {
        let rect = Rect {
            x: x + self.rect.x,
            y: y + self.rect.y,
            ..self.rect
        };
        if let Some(i) = self.node {
            boxes[i] = rect;
            let node = &d.nodes[i];
            drawing.frame(
                rect,
                &self.text,
                node.boundary,
                node.kind == "Node_R",
                palette.element(node),
            )?;
        }
        for child in &self.children {
            child.paint(d, palette, drawing, boxes, rect.x, rect.y)?;
        }
        Ok(())
    }
}

fn arrange(d: &Diagram, options: &Options, scene: Option<&Scene>) -> Result<Frame, String> {
    let mut children = vec![Vec::new(); d.nodes.len() + 1];
    for (i, node) in d.nodes.iter().enumerate() {
        children[node
            .parent
            .as_ref()
            .map(|s| d.ids[s])
            .unwrap_or(d.nodes.len())]
        .push(i);
    }
    let root = d.nodes.len();
    let mut align = vec![Vec::new(); root + 1];
    for (row, members) in &d.align {
        let mut scopes = FxHashMap::<usize, Vec<usize>>::default();
        for name in members {
            let mut child = d.ids[name];
            while child != root {
                let parent = d.nodes[child]
                    .parent
                    .as_ref()
                    .map(|s| d.ids[s])
                    .unwrap_or(root);
                let group = scopes.entry(parent).or_default();
                if group.last() != Some(&child) {
                    group.push(child);
                }
                child = parent;
            }
        }
        for (scope, siblings) in scopes {
            if siblings.len() > 1 {
                align[scope].push((*row, siblings));
            }
        }
    }
    let mut frame = Layout {
        diagram: d,
        children,
        align,
        options,
        scene,
    }
    .frame(
        None,
        if scene.is_some() {
            options.width.clamp(56, 132)
        } else {
            options.width.max(20)
        },
    )?;
    if !d.align.is_empty() {
        frame.align(d)?;
    }
    Ok(frame)
}

pub(super) fn render(d: &Diagram, options: &Options, palette: &Palette) -> Result<String, String> {
    let frame = arrange(d, options, None)?;
    let mut drawing = Drawing::new(frame.rect.w, frame.rect.h, options)?;
    let mut boxes = vec![Rect::default(); d.nodes.len()];
    frame.paint(d, palette, &mut drawing, &mut boxes, 0, 0)?;
    if boxes.is_empty() {
        drawing.frame(frame.rect, &[], true, false, palette.boundary)?;
    }
    let mut connections = Vec::new();
    for (n, edge) in d.edges.iter().enumerate() {
        let paint = palette.relationship(edge);
        if !drawing.connect(d, edge, &boxes, n + 1, paint)? {
            connections.push((n + 1, edge, paint));
        }
    }
    let mut result = Vec::new();
    for heading in &d.heading {
        result.extend(wrap(heading, options.width.max(16)));
    }
    let picture = drawing.finish();
    result.push(picture.trim_matches('\n').to_owned());
    if !connections.is_empty() {
        result.push(String::new());
        let mut plain_bytes = 0;
        for (n, edge, paint) in connections {
            let (panel, size) =
                super::wires::connection_panel(d, n, edge, options, palette, paint)?;
            plain_bytes += size;
            if plain_bytes > 16 * 1024 * 1024 {
                return Err("connection panels exceed 16 MiB".into());
            }
            result.push(panel);
        }
    }
    Ok(result.join("\n"))
}
