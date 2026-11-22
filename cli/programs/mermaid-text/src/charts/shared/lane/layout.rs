use super::*;
use crate::charts::shared::plot::image;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default, Clone, Copy)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

#[derive(Default)]
pub(super) struct Block {
    pub rect: Rect,
    pub lines: Vec<String>,
    pub frame: bool,
    heading_width: usize,
}

pub(super) struct Plan {
    pub blocks: Vec<Block>,
    pub order: Vec<usize>,
}

impl Direction {
    fn axes(self, x: usize, y: usize) -> (usize, usize) {
        if self.horizontal { (x, y) } else { (y, x) }
    }
    fn size(self, r: Rect) -> (usize, usize) {
        self.axes(r.w, r.h)
    }
    fn place(self, r: &mut Rect, along: usize, across: usize) {
        (r.x, r.y) = self.axes(along, across);
    }
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut result = vec![];
    for line in text.split('\n') {
        let mut current = String::new();
        let mut columns = 0;
        for word in line.split_whitespace() {
            if columns > 0 {
                if columns + 1 + word.width() > width {
                    result.push(std::mem::take(&mut current));
                    columns = 0;
                } else {
                    current.push(' ');
                    columns += 1;
                }
            }
            let limit = if word.width() <= width + 4 {
                width.max(word.width())
            } else {
                width
            };
            for cluster in word.graphemes(true) {
                if columns > 0 && columns + cluster.width() > limit {
                    result.push(std::mem::take(&mut current));
                    columns = 0;
                }
                columns += cluster.width();
                current.push_str(cluster);
            }
        }
        result.push(current);
    }
    result
}

fn depth_first(order: &mut Vec<usize>, id: usize, children: &[Vec<usize>]) {
    order.push(id);
    for &child in &children[id] {
        depth_first(order, child, children);
    }
}

// Reverse finishing order is topological on the forward edges; ignoring the
// back edges for rank assignment leaves them available to the return router.
fn ranks(members: &[usize], edges: &[(usize, usize)]) -> Vec<usize> {
    let index: FxHashMap<_, _> = members.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let mut adj = vec![vec![]; members.len()];
    for &(a, b) in edges {
        if let (Some(&a), Some(&b)) = (index.get(&a), index.get(&b)) {
            adj[a].push(b);
        }
    }
    let mut seen = vec![false; members.len()];
    let mut order = Vec::new();
    for i in 0..members.len() {
        if seen[i] {
            continue;
        }
        let mut stack = vec![(i, 0)];
        seen[i] = true;
        while let Some((u, k)) = stack.last_mut() {
            if let Some(&v) = adj[*u].get(*k) {
                *k += 1;
                if !seen[v] {
                    seen[v] = true;
                    stack.push((v, 0));
                }
            } else {
                order.push(*u);
                stack.pop();
            }
        }
    }
    let mut rank = vec![0; members.len()];
    for &u in order.iter().rev() {
        seen[u] = false;
        for &v in &adj[u] {
            if seen[v] {
                rank[v] = rank[v].max(rank[u] + 1);
            }
        }
    }
    rank
}

fn group_edges(diagram: &Diagram) -> Vec<Vec<(usize, usize)>> {
    let mut edges = vec![vec![]; diagram.items.len()];
    let path = |mut id: usize| {
        let mut path = Vec::new();
        while id != 0 {
            path.push(id);
            id = diagram.items[id].owner;
        }
        path.reverse();
        path
    };
    for edge in &diagram.edges {
        let mut parent = 0;
        for (a, b) in path(edge.from).into_iter().zip(path(edge.to)) {
            if a != b {
                edges[parent].push((a, b));
                break;
            }
            parent = a;
        }
    }
    edges
}

pub(super) const CELL_WIDTH: f32 = 10.0;
pub(super) const CELL_HEIGHT: f32 = 20.0;

pub(super) enum Metrics<'a> {
    Terminal(usize),
    Image(&'a Scene),
}

pub(super) fn place(d: &Diagram, children: &[Vec<usize>], metrics: Metrics<'_>) -> Plan {
    let dir = d.direction;
    let mut order = vec![];
    depth_first(&mut order, 0, children);
    let swim = d.family == Family::Swimlane;
    let all_edges: Vec<_> = d.edges.iter().map(|e| (e.from, e.to)).collect();
    let global = ranks(&order, &all_edges);
    let mut rank = vec![0; d.items.len()];
    let mut incident = vec![false; rank.len()];
    for &(a, b) in &all_edges {
        incident[a] = true;
        incident[b] = true;
    }
    let mut next = vec![0; rank.len()];
    for (&id, r) in order.iter().zip(global) {
        let owner = d.items[id].owner;
        rank[id] = if swim && !incident[id] && !d.items[id].group {
            next[owner]
        } else {
            r
        };
        next[owner] = next[owner].max(rank[id] + usize::from(id != 0 && !d.items[id].group));
    }
    let n_ranks = 1 + *rank.iter().max().unwrap();
    let columns = if dir.horizontal {
        n_ranks
    } else {
        children[0].len().max(1)
    };
    let page_width = match metrics {
        Metrics::Terminal(width) => width,
        Metrics::Image(_) => columns * 32,
    };
    let width = (page_width / columns).saturating_sub(5).clamp(8, 38);
    let rows = |label: &str, width: usize| match metrics {
        Metrics::Terminal(_) => wrap(label, width),
        Metrics::Image(scene) => image::wrap(scene, label, width as f32 * CELL_WIDTH),
    };
    let line_width = |s: &str| match metrics {
        Metrics::Terminal(_) => s.width(),
        Metrics::Image(scene) => (scene.measure(s) / CELL_WIDTH).ceil() as usize,
    };
    let mut blocks: Vec<Block> = (0..d.items.len()).map(|_| Block::default()).collect();
    for &id in &order[1..] {
        let item = &d.items[id];
        let frame = item.group && !item.collapsed;
        let mut lines = rows(
            &item.label,
            if frame && dir.horizontal {
                page_width.clamp(12, 48)
            } else {
                width
            },
        );
        let role = if item.collapsed && item.group {
            "flow +"
        } else if !swim && !frame {
            match item.shape.as_str() {
                "" | "rect" | "rounded" => "task",
                "diamond" => "decision",
                role => role,
            }
        } else if swim && !frame {
            match item.shape.as_str() {
                "" | "rect" | "rounded" | "stadium" | "circle" | "double-circle" | "decision" => "",
                role => role,
            }
        } else {
            ""
        };
        if !role.is_empty() {
            let roles = rows(role, width);
            lines.splice(
                0..0,
                roles.into_iter().map(|s| match metrics {
                    Metrics::Terminal(_) => format!("[{s}]"),
                    Metrics::Image(_) => s,
                }),
            );
        }
        let heading_width = lines.iter().map(|s| line_width(s)).max().unwrap_or(0) + 4;
        let (w, h) = match metrics {
            Metrics::Image(_)
                if !frame && matches!(item.shape.as_str(), "decision" | "diamond") =>
            {
                (2 * heading_width - 2, 2 * lines.len() + 2)
            }
            Metrics::Image(_)
                if !frame
                    && matches!(
                        item.shape.as_str(),
                        "circle" | "double-circle" | "connector"
                    ) =>
            {
                let h = (lines.len() + 2).max(heading_width.div_ceil(2));
                (2 * h, h)
            }
            Metrics::Image(_) if !frame => (heading_width.max(8), lines.len() + 2),
            _ => (heading_width.max(5), lines.len() + 2),
        };
        blocks[id] = Block {
            rect: Rect { x: 0, y: 0, w, h },
            lines,
            frame,
            heading_width,
        };
    }
    if swim {
        swimlanes(&mut blocks, &order, children, &rank, d);
    } else {
        let edges = group_edges(d);
        for &id in order.iter().rev() {
            if id != 0 && !blocks[id].frame {
                continue;
            }
            let members = &children[id];
            let ranks = ranks(members, &edges[id]);
            let count = ranks.iter().max().map_or(0, |r| r + 1);
            let mut layers = vec![vec![]; count];
            for (&member, r) in members.iter().zip(ranks) {
                layers[r].push(member);
            }
            let mut groups = vec![];
            for layer in &layers {
                let (mut along, mut across) = (0, 0);
                for &m in layer {
                    let (a, b) = dir.size(blocks[m].rect);
                    along = along.max(a);
                    across += b + 3;
                }
                groups.push((along, across.saturating_sub(3)));
            }
            let total = groups
                .iter()
                .map(|(a, _)| a + 4)
                .sum::<usize>()
                .saturating_sub(4);
            let breadth = groups.iter().map(|(_, b)| *b).max().unwrap_or(1);
            let (w, h) = dir.axes(total, breadth);
            let title = blocks[id].lines.len();
            let rect = &mut blocks[id].rect;
            rect.w = rect.w.max(w + 6);
            rect.h = rect.h.max(h + title + 4);
            let (x, y) = ((rect.w - w) / 2, title + (rect.h - title - h) / 2);
            let (start, side) = dir.axes(x, y);
            let mut cursor = 0;
            for (layer, (along, across)) in layers.iter().zip(groups) {
                let mut slot = side + breadth / 2 - across / 2;
                for &m in layer {
                    let (a, b) = dir.size(blocks[m].rect);
                    let pos = if dir.reverse {
                        total - cursor - along
                    } else {
                        cursor
                    };
                    dir.place(&mut blocks[m].rect, start + pos + along / 2 - a / 2, slot);
                    slot += b + 3;
                }
                cursor += along + 4;
            }
        }
    }
    for &id in &order[1..] {
        let parent = blocks[d.items[id].owner].rect;
        blocks[id].rect.x += parent.x;
        blocks[id].rect.y += parent.y;
    }
    if swim {
        for &id in order.iter().rev() {
            if d.items[id].owner == 0 || !blocks[id].frame || children[id].is_empty() {
                continue;
            }
            let extent = |r: Rect| {
                if dir.horizontal {
                    (r.x, r.x + r.w)
                } else {
                    (r.y, r.y + r.h)
                }
            };
            let (mut low, mut high) = (usize::MAX, 0);
            for &child in &children[id] {
                let (lo, hi) = extent(blocks[child].rect);
                low = low.min(lo);
                high = high.max(hi);
            }
            let head = &blocks[id].lines;
            let title = blocks[id].heading_width;
            let padding = if dir.horizontal { 2 } else { head.len() + 2 };
            let rect = &mut blocks[id].rect;
            let (lo, hi) = extent(*rect);
            let end = (high + 2).min(hi);
            let start = low.saturating_sub(padding).max(lo);
            if dir.horizontal {
                rect.w = (end - start).max(title).min(hi - lo);
                rect.x = start.min(hi - rect.w);
            } else {
                rect.y = start;
                rect.h = end - start;
            }
        }
    }
    Plan { blocks, order }
}

fn swimlanes(
    blocks: &mut [Block],
    order: &[usize],
    children: &[Vec<usize>],
    rank: &[usize],
    d: &Diagram,
) {
    let dir = d.direction;
    let mut span = vec![0; 1 + rank.iter().max().unwrap()];
    let mut depth = vec![0; blocks.len()];
    let mut headings = Vec::new();
    for &id in &order[1..] {
        depth[id] = depth[d.items[id].owner] + 1;
        if blocks[id].frame {
            headings.resize(headings.len().max(depth[id]), 0);
            headings[depth[id] - 1] = headings[depth[id] - 1].max(blocks[id].lines.len());
        } else {
            span[rank[id]] = span[rank[id]].max(dir.size(blocks[id].rect).0);
        }
    }
    // A shared origin for the ranks keeps a handoff aligned even when one
    // endpoint is inside a deeper group with several title rows.
    let mut start = vec![0, 2];
    for h in headings {
        start.push(start.last().unwrap() + if dir.horizontal { 2 } else { h + 2 });
    }
    let mut cursor = *start.last().unwrap();
    let mut offsets = vec![0; span.len()];
    for i in 0..span.len() {
        let r = if dir.reverse { span.len() - i - 1 } else { i };
        offsets[r] = cursor;
        cursor += span[r] + 3;
    }
    let full = order
        .iter()
        .filter(|&&id| blocks[id].frame)
        .map(|&id| start[depth[id]] + dir.size(blocks[id].rect).0 + 2 * depth[id])
        .fold(cursor - 3 + 2 * (start.len() - 1), usize::max);
    for &id in order.iter().rev() {
        if id != 0 && !blocks[id].frame {
            continue;
        }
        let level = depth[id];
        let mut side = if dir.horizontal {
            blocks[id].lines.len() + 2
        } else {
            2
        };
        let mut layers = BTreeMap::new();
        for &m in &children[id] {
            if blocks[m].frame {
                dir.place(&mut blocks[m].rect, start[level + 1] - start[level], side);
                side += dir.size(blocks[m].rect).1 + 2;
            } else {
                layers.entry(rank[m]).or_insert_with(Vec::new).push(m);
            }
        }
        let breadth = |members: &[usize]| {
            members
                .iter()
                .map(|&m| dir.size(blocks[m].rect).1 + 3)
                .sum::<usize>()
                .saturating_sub(3)
        };
        let width = layers.values().map(|ms| breadth(ms)).max().unwrap_or(0);
        let strips: Vec<_> = layers
            .into_iter()
            .map(|(r, ms)| (r, breadth(&ms), ms))
            .collect();
        let title_size = blocks[id].rect;
        let major = full - start[level] - 2 * level;
        let minor = (side + width + 2).max(dir.size(title_size).1);
        for (r, w, members) in strips {
            let mut s = side + (minor - 2 - side) / 2 - w / 2;
            for m in members {
                let (a, b) = dir.size(blocks[m].rect);
                dir.place(
                    &mut blocks[m].rect,
                    offsets[r] - start[level] + span[r] / 2 - a / 2,
                    s,
                );
                s += b + 3;
            }
        }
        (blocks[id].rect.w, blocks[id].rect.h) = dir.axes(major.max(dir.size(title_size).0), minor);
    }
}
