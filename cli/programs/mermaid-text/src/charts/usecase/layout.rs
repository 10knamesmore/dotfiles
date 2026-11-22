use std::collections::BTreeSet;

use super::{Diagram, Kind, Node, Tip};
use crate::text::wrap;
use crate::{Options, canvas::Canvas};
use unicode_width::UnicodeWidthStr;

type Cell = (usize, usize);

struct Drawing {
    width: usize,
    lines: Vec<u8>,
    rect_corners: Vec<[usize; 4]>,
    text: Vec<(Cell, String)>,
    ascii: bool,
}

impl Drawing {
    fn line(&mut self, (x, y): Cell, (xx, yy): Cell) {
        if y == yy {
            for i in x.min(xx)..x.max(xx) {
                self.lines[y * self.width + i] |= 2;
                self.lines[y * self.width + i + 1] |= 8;
            }
        } else {
            for j in y.min(yy)..y.max(yy) {
                self.lines[j * self.width + x] |= 4;
                self.lines[(j + 1) * self.width + x] |= 1;
            }
        }
    }
    fn write(&mut self, p: Cell, text: impl Into<String>) {
        self.text.push((p, text.into()));
    }
    fn tip(&mut self, (x, y): Cell, t: Tip, dx: isize, dy: isize) {
        if t == Tip::None {
            return;
        }
        self.write((x, y), mark(t, dx, dy, self.ascii));
        if self.ascii && t == Tip::Triangle {
            self.write(
                (x.wrapping_add_signed(-dx), y.wrapping_add_signed(-dy)),
                if dx == 0 { "-" } else { "|" },
            );
        }
    }
    fn rect(&mut self, (x, y): Cell, w: usize, h: usize, rounded: bool) {
        let r = x + w - 1;
        let b = y + h - 1;
        self.line((x, y), (r, y));
        self.line((x, b), (r, b));
        self.line((x, y), (x, b));
        self.line((r, y), (r, b));
        if rounded {
            for (pos, c) in [
                ((x, y), if self.ascii { "." } else { "╭" }),
                ((r, y), if self.ascii { "." } else { "╮" }),
                ((x, b), if self.ascii { "'" } else { "╰" }),
                ((r, b), if self.ascii { "'" } else { "╯" }),
            ] {
                self.write(pos, c);
            }
        } else {
            self.rect_corners
                .push([(x, y), (r, y), (x, b), (r, b)].map(|(x, y)| y * self.width + x));
        }
    }
    fn finish(self, mut c: Canvas, options: &Options) -> Result<String, String> {
        for (i, &mask) in self.lines.iter().enumerate() {
            if mask == 0 {
                continue;
            }
            let glyph = if self.ascii {
                match mask {
                    1 | 4 | 5 => '|',
                    2 | 8 | 10 => '-',
                    _ => '+',
                }
            } else {
                [
                    ' ', '│', '─', '└', '│', '│', '┌', '├', '─', '┘', '─', '┴', '┐', '┤', '┬', '┼',
                ][usize::from(mask)]
            };
            c.put(i % self.width, i / self.width, glyph)?;
        }
        let natural = if self.ascii {
            ['+'; 4]
        } else {
            ['┌', '┐', '└', '┘']
        };
        let [tl, tr, bl, br] = options.corners.resolve(self.ascii, natural);
        let corners = [(6, tl), (12, tr), (3, bl), (9, br)];
        for indices in self.rect_corners {
            for (i, (mask, glyph)) in indices.into_iter().zip(corners) {
                if self.lines[i] == mask {
                    c.put(i % self.width, i / self.width, glyph)?;
                }
            }
        }
        for ((x, y), s) in self.text {
            c.text(x, y, &s)?;
        }
        Ok(c.finish())
    }
}

pub(super) fn labels(n: &Node, ascii: bool) -> Vec<String> {
    let mut lines = Vec::new();
    if !n.stereotype.is_empty() {
        lines.push(if ascii {
            format!("<<{}>>", n.stereotype)
        } else {
            format!("«{}»", n.stereotype)
        });
    }
    lines.push(n.label.clone());
    if n.attr.get("business").is_some_and(|v| v == "true") {
        lines.push("/ business".into());
    }
    lines.extend(n.rows.iter().cloned());
    lines
}

fn content(n: &Node, o: &Options) -> Vec<String> {
    let cap = o.width.saturating_sub(12).clamp(10, 60);
    let mut lines = Vec::new();
    if n.kind == Kind::Actor {
        if let Some(icon) = n.attr.get("icon") {
            lines.extend(wrap(icon, cap));
        } else {
            let head = match (o.ascii, n.attr.get("type").map(String::as_str)) {
                (false, Some("hollow")) => "◎",
                (false, Some("awesome")) => "★",
                (false, _) => "○",
                (true, Some("awesome")) => "*",
                (true, Some("hollow")) => "O",
                (true, _) => "o",
            };
            lines.extend([
                head.into(),
                if o.ascii { "-+-" } else { "─┼─" }.into(),
                "/ \\".into(),
            ]);
        }
    }
    lines.extend(labels(n, o.ascii).into_iter().flat_map(|s| wrap(&s, cap)));
    lines
}

fn mark(t: Tip, dx: isize, dy: isize, ascii: bool) -> &'static str {
    match t {
        Tip::None => "",
        Tip::Circle => {
            if ascii {
                "o"
            } else {
                "○"
            }
        }
        Tip::Cross => {
            if ascii {
                "x"
            } else {
                "×"
            }
        }
        Tip::Triangle => match (dx, dy, ascii) {
            (1, _, false) => "▷",
            (-1, _, false) => "◁",
            (_, 1, false) => "▽",
            (_, _, false) => "△",
            (1, _, true) => ">",
            (-1, _, true) => "<",
            (_, 1, true) => "v",
            _ => "^",
        },
        Tip::Arrow => match (dx, dy, ascii) {
            (1, _, false) => "▶",
            (-1, _, false) => "◀",
            (_, 1, false) => "▼",
            (_, _, false) => "▲",
            (1, _, true) => ">",
            (-1, _, true) => "<",
            (_, 1, true) => "v",
            _ => "^",
        },
    }
}

struct Card {
    rows: Vec<String>,
    size: Cell,
    pos: Cell, // layout coordinates: along direction, across direction
    ports: usize,
}

pub(super) fn groups(d: &Diagram) -> Vec<(Option<usize>, Vec<usize>)> {
    let mut groups: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
    let mut slots = vec![0; d.nodes.len()];
    for (i, n) in d.nodes.iter().enumerate() {
        if n.kind == Kind::Boundary {
            slots[i] = groups.len();
            groups.push((Some(i), vec![]));
        } else if let Some(g) = n.group {
            groups[slots[g]].1.push(i);
        } else {
            groups.push((None, vec![i]));
        }
    }
    if matches!(d.direction.as_str(), "RL" | "BT") {
        groups.reverse();
        for (_, members) in &mut groups {
            members.reverse();
        }
    }
    groups
}

pub(super) fn draw(d: &Diagram, o: &Options) -> Result<String, String> {
    let groups = groups(d);
    let order: Vec<_> = groups.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    let mut ranks = vec![0; d.nodes.len()];
    for (r, i) in order.iter().copied().enumerate() {
        ranks[i] = r;
    }
    let mut direct = BTreeSet::new();
    let mut routes = Vec::new();
    let mut degree = vec![0; d.nodes.len()];
    for e in &d.edges {
        let (a, b) = (d.ids[&e.from], d.ids[&e.to]);
        let adjacent = ranks[a].abs_diff(ranks[b]) == 1
            && direct.insert((a.min(b), a.max(b)))
            && e.label.is_empty();
        if !adjacent {
            degree[a] += 1;
            degree[b] += 1;
        }
        routes.push((a, b, adjacent));
    }
    let mut cards: Vec<_> = d
        .nodes
        .iter()
        .map(|n| {
            let rows = content(n, o);
            let size = (
                rows.iter().map(|r| r.width()).max().unwrap_or(0) + 4,
                rows.len() + 2,
            );
            Card {
                rows,
                size,
                pos: (0, 0),
                ports: 0,
            }
        })
        .collect();
    let titles: Vec<_> = d
        .nodes
        .iter()
        .map(|n| {
            if n.kind != Kind::Boundary {
                return Vec::new();
            }
            let title = if n.attr.get("type").is_some_and(|v| v == "package") {
                format!("[{}]", n.label)
            } else {
                n.label.clone()
            };
            wrap(&title, o.width.saturating_sub(6).clamp(10, 60))
        })
        .collect();
    let title_width = |id: usize| titles[id].iter().map(|s| s.width()).max().unwrap_or(0) + 4;
    let estimated: usize = groups
        .iter()
        .map(|(g, v)| {
            let n = v
                .iter()
                .map(|&i| cards[i].size.0.max(degree[i] + 2) + 4)
                .sum::<usize>();
            g.map_or(n, |id| (n + 4).max(title_width(id) + 4))
        })
        .sum();
    let horizontal = !matches!(d.direction.as_str(), "TD" | "TB" | "BT")
        && (estimated <= o.width + 4 || order.len() <= 1);
    let xy = |a: usize, b: usize| if horizontal { (a, b) } else { (b, a) };
    for (i, c) in cards.iter_mut().enumerate() {
        if horizontal {
            c.size.0 = c.size.0.max(degree[i] + 2);
        } else {
            c.size.1 = c.size.1.max(degree[i] + 2);
        }
    }
    let cross = order
        .iter()
        .map(|&i| xy(cards[i].size.0, cards[i].size.1).1)
        .max()
        .unwrap_or(3);
    let group_cross = groups
        .iter()
        .filter_map(|(g, _)| *g)
        .map(title_width)
        .max()
        .unwrap_or(0);
    let title_height = titles.iter().map(Vec::len).max().unwrap_or(0);
    let band = if horizontal {
        cross + 2 * title_height + 2
    } else {
        (cross + 6).max(group_cross)
    };
    let mut bounds = Vec::new();
    let mut pos = 0;
    for (g, members) in &groups {
        let start = pos;
        if let Some(id) = g {
            pos += if horizontal { 3 } else { titles[*id].len() + 2 };
        }
        for &i in members {
            let c = &mut cards[i];
            let (len, thick) = xy(c.size.0, c.size.1);
            c.pos = (pos, band / 2 - thick / 2);
            pos += len + 4;
        }
        if !members.is_empty() {
            pos -= 4;
        }
        if let Some(id) = g {
            pos += 3;
            if horizontal {
                pos = pos.max(start + title_width(*id));
            }
            bounds.push((*id, start, pos));
        }
        pos += 4;
    }
    let extent = pos - 4;
    let count = routes.iter().filter(|r| !r.2).count();
    let mut captions: Vec<_> = d
        .edges
        .iter()
        .map(|e| {
            if e.dotted {
                if o.ascii {
                    format!("<<{}>>", e.label)
                } else {
                    format!("«{}»", e.label)
                }
            } else {
                e.label.replace('\n', " ")
            }
        })
        .collect();
    let mut footnotes = Vec::new();
    for caption in &mut captions {
        if caption.width() > o.width.saturating_sub(band).clamp(12, 50) {
            let id = format!("[{}]", footnotes.len() + 1);
            footnotes.push(format!("{id} {caption}"));
            *caption = id;
        }
    }
    let max_caption = captions.iter().map(|s| s.width()).max().unwrap_or(0);
    let lane_start = band + if horizontal { 0 } else { max_caption + 1 };
    let (mut width, height) = xy(extent, lane_start + count);
    if horizontal && max_caption > 0 {
        width += max_caption + 2;
    }
    let mut canvas = Canvas::new(width, height)?;
    let mut graph = Drawing {
        width,
        lines: vec![0; width * height],
        rect_corners: vec![],
        text: vec![],
        ascii: o.ascii,
    };
    for (id, start, end) in bounds {
        let (x, y) = xy(start, 0);
        let (w, h) = xy(end - start, band);
        graph.rect((x, y), w, h, false);
        if o.color {
            canvas.paint_box(
                x,
                y,
                w,
                h,
                d.styles.resolve(&d.nodes[id].id, o.theme_style()),
            );
        }
        for (offset, title) in titles[id].iter().enumerate() {
            graph.write((x + 1, y + offset), format!(" {title} "));
        }
    }
    for &i in &order {
        let n = &d.nodes[i];
        let c = &cards[i];
        let (x, y) = xy(c.pos.0, c.pos.1);
        let (w, h) = c.size;
        graph.rect((x, y), w, h, n.kind == Kind::Ellipse);
        if o.color {
            canvas.paint_box(x, y, w, h, d.styles.resolve(&n.id, o.theme_style()));
        }
        let top = y + (h - c.rows.len()) / 2;
        for (j, line) in c.rows.iter().enumerate() {
            let offset = if n.kind == Kind::Json {
                2
            } else {
                (w - line.width()) / 2
            };
            graph.write((x + offset, top + j), line);
        }
    }
    let mut lane = lane_start;
    let mut lanes = Vec::new();
    for (ei, (a, b, adjacent)) in routes.into_iter().enumerate() {
        let e = &d.edges[ei];
        if adjacent {
            let forward = cards[a].pos.0 < cards[b].pos.0;
            let (le, ri) = if forward { (a, b) } else { (b, a) };
            let lp = cards[le].pos.0 + xy(cards[le].size.0, cards[le].size.1).0 - 1;
            let rp = cards[ri].pos.0;
            let mid = band / 2;
            graph.line(xy(lp, mid), xy(rp, mid));
            let (l, r) = if forward {
                e.ends
            } else {
                (e.ends.1, e.ends.0)
            };
            let (dx, dy) = xy(1, 0);
            graph.tip(xy(lp + 1, mid), l, -(dx as isize), -(dy as isize));
            graph.tip(xy(rp - 1, mid), r, dx as isize, dy as isize);
        } else {
            let mut ports = [0; 2];
            for (j, (i, tip)) in [(a, e.ends.0), (b, e.ends.1)].into_iter().enumerate() {
                let c = &mut cards[i];
                let (len, thick) = xy(c.size.0, c.size.1);
                let port = c.pos.0 + (len - degree[i]) / 2 + c.ports;
                c.ports += 1;
                ports[j] = port;
                let edge = c.pos.1 + thick - 1;
                graph.line(xy(port, edge), xy(port, lane));
                let (dx, dy) = xy(0, 1);
                graph.tip(xy(port, edge + 1), tip, -(dx as isize), -(dy as isize));
            }
            graph.line(xy(ports[0], lane), xy(ports[1], lane));
            lanes.push((ei, lane, ports));
            lane += 1;
        }
    }
    for (ei, lane, ports) in lanes {
        let lo = ports[0].min(ports[1]) + 1;
        let hi = ports[0].max(ports[1]);
        if d.edges[ei].dotted {
            for u in lo..hi {
                let (x, y) = xy(u, lane);
                if matches!(graph.lines[y * width + x], 5 | 10) {
                    graph.write((x, y), if o.ascii { "." } else { "·" });
                }
            }
        }
        let label = &captions[ei];
        if label.is_empty() {
            continue;
        }
        if horizontal {
            let space = label.width() + 2;
            let centered = (hi - lo).saturating_sub(space) / 2;
            let runs = graph.lines[lane * width + lo..lane * width + hi]
                .windows(space)
                .enumerate();
            if let Some((offset, _)) = runs
                .filter(|(_, run)| run.iter().all(|&s| s == 10))
                .min_by_key(|(i, _)| i.abs_diff(centered))
            {
                graph.write((lo + offset, lane), format!(" {label} "));
            } else {
                let x = (hi + 2..=extent + 1)
                    .find(|&x| {
                        graph.lines[lane * width + x..lane * width + x + label.width()]
                            .iter()
                            .all(|&s| s == 0)
                    })
                    .unwrap_or(extent + 1);
                graph.write((x, lane), label);
            }
        } else {
            graph.write((band, ports[0]), label);
        }
    }
    let mut parts = d.headings.clone();
    parts.push(graph.finish(canvas, o)?.trim_matches('\n').to_string());
    for text in footnotes {
        parts.extend(wrap(&text, o.width.max(12)));
    }
    Ok(parts.join("\n"))
}
