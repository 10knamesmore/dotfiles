use super::shared::area::{body, columns, single_line, weight};
use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint, Style},
};
use std::collections::BTreeMap;
use unicode_width::UnicodeWidthStr;

mod image;

struct Node {
    name: String,
    class: Option<String>,
    amount: Option<String>,
    weight: f64,
    children: Vec<usize>,
}

struct Tree {
    nodes: Vec<Node>,
    roots: Vec<usize>,
    headings: Vec<String>,
    class_defs: Vec<(String, String)>,
}

impl Tree {
    fn styles(&self, options: &Options) -> Vec<Style> {
        let mut classes = BTreeMap::<&str, Style>::new();
        if options.color {
            for (name, declarations) in &self.class_defs {
                let style = classes.entry(name).or_default();
                *style = style.overlay(Style::css(declarations));
            }
        }
        let theme = options.theme_style();
        self.nodes
            .iter()
            .map(|node| {
                let class = node.class.as_deref().and_then(|name| classes.get(name));
                theme.overlay(class.copied().unwrap_or_default())
            })
            .collect()
    }
}

fn identifier(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn uncomment(s: &str) -> &str {
    s.split_once("%%").map_or(s, |(before, _)| before).trim()
}

fn parse(body: &str) -> Result<Tree, String> {
    let mut tree = Tree {
        nodes: Vec::new(),
        roots: Vec::new(),
        headings: Vec::new(),
        class_defs: Vec::new(),
    };
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut lines = body.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let text = line.trim_start();
        let error = |detail: &str| format!("treemap line {}: {detail}", index + 2);
        if text.is_empty() || text.starts_with("%%") {
            continue;
        }
        if let Some(title) = text
            .strip_prefix("title")
            .and_then(|s| s.strip_prefix(char::is_whitespace))
        {
            tree.headings.push(uncomment(title).to_string());
            continue;
        }
        if let Some(heading) = text
            .strip_prefix("accTitle:")
            .or_else(|| text.strip_prefix("accDescr:"))
        {
            tree.headings.push(uncomment(heading).to_string());
            continue;
        }
        if let Some(mut rest) = text
            .strip_prefix("accDescr")
            .and_then(|s| s.trim_start().strip_prefix('{'))
        {
            let mut desc = Vec::new();
            loop {
                if let Some((end, tail)) = rest.split_once('}') {
                    if !uncomment(tail).is_empty() {
                        return Err(error("unexpected text after accDescr block"));
                    }
                    desc.push(end.trim());
                    break;
                }
                desc.push(rest.trim());
                rest = lines
                    .next()
                    .ok_or_else(|| error("unterminated accDescr block"))?
                    .1;
            }
            tree.headings.push(desc.join("\n").trim().to_string());
            continue;
        }
        if let Some(def) = text
            .strip_prefix("classDef")
            .and_then(|s| s.strip_prefix(char::is_whitespace))
        {
            let def = uncomment(def).trim_end_matches(';').trim();
            let (class, style) = def.split_once(char::is_whitespace).unwrap_or((def, ""));
            if !identifier(class) || style.contains(';') {
                return Err(error("invalid classDef"));
            }
            tree.class_defs.push((class.into(), style.trim().into()));
            continue;
        }
        let quote = text.chars().next().unwrap();
        if quote != '"' && quote != '\'' {
            return Err(error(
                "expected a quoted node, title, accessibility text, or classDef",
            ));
        }
        let mut name = String::new();
        let mut rest = &text[1..];
        let suffix = loop {
            if let Some((part, after)) = rest.split_once(quote) {
                name.push_str(part);
                break uncomment(after);
            }
            name.push_str(rest);
            name.push('\n');
            rest = lines
                .next()
                .ok_or_else(|| error("unterminated quoted name"))?
                .1;
        };
        let mut name = single_line(&name);
        let (spec, class) = if let Some((spec, class)) = suffix.split_once(":::") {
            let class = class.trim();
            if !identifier(class) {
                return Err(error("invalid class name after :::"));
            }
            name.push_str(&format!(" [{class}]"));
            (spec.trim(), Some(class.to_owned()))
        } else {
            (suffix, None)
        };
        let amount = if spec.is_empty() {
            None
        } else {
            let value = spec
                .strip_prefix([':', ','])
                .ok_or_else(|| error("expected : or , before weight"))?
                .trim();
            Some(value.to_string())
        };
        let value = match &amount {
            Some(s) => {
                for (i, _) in s.match_indices([',', '_']) {
                    let bytes = s.as_bytes();
                    if i == 0
                        || !bytes[i - 1].is_ascii_digit()
                        || !bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                    {
                        return Err(error("expected digits on both sides of numeric separator"));
                    }
                }
                weight(&s.replace([',', '_'], "")).map_err(|e| error(&e))?
            }
            None => 1.0,
        };
        let indent = line[..line.len() - text.len()]
            .chars()
            .fold(0, |n, c| if c == '\t' { n + 4 - n % 4 } else { n + 1 });
        if stack.last().is_some_and(|&(level, _)| indent < level)
            && !stack.iter().any(|&(level, _)| indent == level)
        {
            return Err(error("dedent does not match an earlier indentation level"));
        }
        while stack.last().is_some_and(|&(level, _)| level >= indent) {
            stack.pop();
        }
        if stack.len() >= 64 || tree.nodes.len() >= 8192 {
            return Err(error("treemap exceeds 64 nesting levels or 8192 nodes"));
        }
        let id = tree.nodes.len();
        if let Some(&(_, parent)) = stack.last() {
            if tree.nodes[parent].amount.is_some() {
                return Err(error("a weighted leaf cannot have children"));
            }
            tree.nodes[parent].children.push(id);
        } else {
            tree.roots.push(id);
        }
        tree.nodes.push(Node {
            name,
            class,
            amount,
            weight: value,
            children: Vec::new(),
        });
        stack.push((indent, id));
    }
    for i in (0..tree.nodes.len()).rev() {
        let node = &tree.nodes[i];
        if !node.children.is_empty() {
            let total: f64 = node.children.iter().map(|&j| tree.nodes[j].weight).sum();
            if !total.is_finite() {
                return Err("treemap section weights overflow".into());
            }
            tree.nodes[i].weight = total;
        }
    }
    Ok(tree)
}

// First choose the cuts in continuous space. Propagating the space needed for
// each tile back up that same tree lets us snap to cells without losing tiles.
struct Layout {
    w: usize,
    h: usize,
    kind: Kind,
}

enum Kind {
    Tile(usize, Option<Box<Layout>>),
    Split {
        across: bool,
        share: f64,
        a: Box<Layout>,
        b: Box<Layout>,
    },
}

fn layout(ids: &[usize], nodes: &[Node], w: f64, h: f64) -> Layout {
    if let [id] = ids {
        let node = &nodes[*id];
        let children = if node.children.is_empty() {
            None
        } else {
            Some(Box::new(forest(
                &node.children,
                nodes,
                (w - 2.0).max(4.0),
                (h - 3.0).max(3.0),
            )))
        };
        let key = format!("#{}", id + 1);
        return Layout {
            w: (key.len() + 2).max(children.as_ref().map_or(0, |c| c.w + 2)),
            h: children.as_ref().map_or(3, |c| c.h + 3),
            kind: Kind::Tile(*id, children),
        };
    }
    let (cut, share) = split(ids, nodes);
    let across = w >= 2.0 * h;
    let (wa, ha, wb, hb) = if across {
        (w * share, h, w * (1.0 - share), h)
    } else {
        (w, h * share, w, h * (1.0 - share))
    };
    let a = Box::new(layout(&ids[..cut], nodes, wa, ha));
    let b = Box::new(layout(&ids[cut..], nodes, wb, hb));
    Layout {
        w: if across { a.w + b.w - 1 } else { a.w.max(b.w) },
        h: if across { a.h.max(b.h) } else { a.h + b.h - 1 },
        kind: Kind::Split {
            across,
            share,
            a,
            b,
        },
    }
}

fn split(ids: &[usize], nodes: &[Node]) -> (usize, f64) {
    let max = nodes[ids[0]].weight;
    let mass = |id: &usize| {
        if max == 0.0 {
            1.0
        } else {
            nodes[*id].weight / max
        }
    };
    let total: f64 = ids.iter().map(mass).sum();
    let mut subtotal = 0.0;
    let mut cut = 1;
    let mut share = 0.0;
    let mut best = f64::INFINITY;
    for (i, id) in ids[..ids.len() - 1].iter().enumerate() {
        subtotal += mass(id);
        // Bound cut-tree depth even for exponentially decreasing weights.
        if (i + 1).min(ids.len() - i - 1) < ids.len() / 8 {
            continue;
        }
        let distance = (subtotal - total * 0.5).abs();
        if distance < best {
            best = distance;
            cut = i + 1;
            share = subtotal / total;
        }
    }
    (cut, share)
}

fn forest(ids: &[usize], nodes: &[Node], w: f64, h: f64) -> Layout {
    let mut sorted = ids.to_vec();
    sorted.sort_by(|&a, &b| nodes[b].weight.total_cmp(&nodes[a].weight));
    layout(&sorted, nodes, w, h)
}

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

struct Map<'a> {
    canvas: Canvas,
    styles: &'a [Style],
    width: usize,
    borders: Vec<u8>,
    legend: Vec<(usize, String)>,
}

impl Map<'_> {
    fn frame(&mut self, r: Rect, border: Paint) {
        let mut join = |x, y, mask: u8| self.borders[y * self.width + x] |= mask;
        let (right, bottom) = (r.x + r.w - 1, r.y + r.h - 1);
        for y in [r.y, bottom] {
            self.canvas.paint(r.x, y, r.w, border);
        }
        for y in r.y + 1..bottom {
            for x in [r.x, right] {
                self.canvas.paint(x, y, 1, border);
            }
        }
        for x in r.x..right {
            for y in [r.y, bottom] {
                join(x, y, 2);
                join(x + 1, y, 8);
            }
        }
        for y in r.y..bottom {
            for x in [r.x, right] {
                join(x, y, 4);
                join(x, y + 1, 1);
            }
        }
    }

    fn place(&mut self, layout: &Layout, r: Rect, nodes: &[Node]) -> Result<(), String> {
        match &layout.kind {
            Kind::Split {
                across,
                share,
                a,
                b,
            } => {
                let (span, min_a, min_b) = if *across {
                    (r.w, a.w, b.w)
                } else {
                    (r.h, a.h, b.h)
                };
                let offset =
                    (((span - 1) as f64 * share).round() as usize).clamp(min_a - 1, span - min_b);
                let (ra, rb) = if *across {
                    (
                        Rect { w: offset + 1, ..r },
                        Rect {
                            x: r.x + offset,
                            w: r.w - offset,
                            ..r
                        },
                    )
                } else {
                    (
                        Rect { h: offset + 1, ..r },
                        Rect {
                            y: r.y + offset,
                            h: r.h - offset,
                            ..r
                        },
                    )
                };
                self.place(a, ra, nodes)?;
                self.place(b, rb, nodes)?;
            }
            Kind::Tile(id, children) => {
                let style = self.styles[*id];
                self.frame(r, style.border());
                let node = &nodes[*id];
                let area = r.w - 2;
                let interior = if children.is_some() { 1 } else { r.h - 2 };
                for y in r.y + 1..r.y + 1 + interior {
                    self.canvas.paint(
                        r.x + 1,
                        y,
                        area,
                        Paint {
                            fg: None,
                            bg: style.fill,
                        },
                    );
                }
                let key = format!("#{}", id + 1);
                let full = match &node.amount {
                    Some(value) => format!("{}: {value}", node.name),
                    None => node.name.clone(),
                };
                let fits = |s: &str| columns(s).is_some_and(|w| w <= area);
                let mut caption = if fits(&full) {
                    vec![full.clone()]
                } else if let Some(value) = &node.amount
                    && r.h >= 4
                    && fits(&node.name)
                    && fits(value)
                {
                    vec![node.name.clone(), value.clone()]
                } else {
                    self.legend.push((*id, format!("{key}  {full}")));
                    vec![key]
                };
                if let Some(children) = children {
                    let text = caption.pop().unwrap();
                    let w = columns(&text).unwrap();
                    let x = r.x + 1 + usize::from(w < area);
                    self.canvas.text(x, r.y + 1, &text)?;
                    self.canvas.paint(x, r.y + 1, w, style.ink());
                    self.place(
                        children,
                        Rect {
                            x: r.x + 1,
                            y: r.y + 2,
                            w: r.w - 2,
                            h: r.h - 3,
                        },
                        nodes,
                    )?;
                } else {
                    let first_y = r.y + (r.h - caption.len()) / 2;
                    for (line, s) in caption.into_iter().enumerate() {
                        let w = columns(&s).unwrap();
                        let x = r.x + (r.w - w) / 2;
                        self.canvas.text(x, first_y + line, &s)?;
                        self.canvas.paint(x, first_y + line, w, style.ink());
                    }
                }
            }
        }
        Ok(())
    }
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let tree = parse(body(source, &["treemap", "treemap-beta"])?)?;
        let styles = tree.styles(options);
        let theme = options.theme_style();
        let w = options.width.clamp(16, 140);
        let h = (w / 4).max(7);
        let layout =
            (!tree.roots.is_empty()).then(|| forest(&tree.roots, &tree.nodes, w as f64, h as f64));
        let width = layout.as_ref().map_or(w.min(32), |l| w.max(l.w));
        let height = layout.as_ref().map_or(3, |l| h.max(l.h));
        let mut map = Map {
            canvas: Canvas::new(width, height)?,
            styles: &styles,
            width,
            borders: vec![0; width * height],
            legend: Vec::new(),
        };
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        if let Some(layout) = layout {
            map.place(&layout, rect, &tree.nodes)?;
        } else {
            map.frame(rect, theme.border());
        }
        let [tl, tr, bl, br] = options.corners.resolve(options.ascii, ['┌', '┐', '└', '┘']);
        let strokes = [
            ' ', '│', '─', bl, '│', '│', tl, '├', '─', br, '─', '┴', tr, '┤', '┬', '┼',
        ];
        for (index, mask) in map.borders.into_iter().enumerate() {
            if mask != 0 {
                let ch = if options.ascii {
                    match mask {
                        5 => '|',
                        10 => '-',
                        _ => '+',
                    }
                } else {
                    strokes[mask as usize]
                };
                map.canvas.put(index % width, index / width, ch)?;
            }
        }
        let mut blocks = tree.headings;
        blocks.push(map.canvas.finish());
        map.legend.sort_by_key(|(id, _)| *id);
        let mut legend_colors = Layer::default();
        let notes: Vec<_> = map
            .legend
            .into_iter()
            .enumerate()
            .map(|(row, (id, line))| {
                legend_colors.paint(0, row, line.width(), styles[id].ink());
                line
            })
            .chain(
                tree.class_defs
                    .into_iter()
                    .map(|(class, style)| format!("[{class}] {style}")),
            )
            .collect();
        if !notes.is_empty() {
            blocks.push(legend_colors.apply(&notes.join("\n")));
        }
        Ok(blocks.join("\n\n"))
    }
}
