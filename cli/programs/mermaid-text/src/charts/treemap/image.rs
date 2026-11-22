use super::{Tree, body, parse, split};
use crate::{
    Options,
    charts::shared::plot::image::{MARGIN, ROW, center, contrast, fade, palette, theme, wrap},
    color::{Rgb, Style},
    graphics::Scene,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct Map<'a> {
    scene: &'a mut Scene,
    tree: &'a Tree,
    styles: Vec<Style>,
    root_colors: BTreeMap<usize, Rgb>,
    legend: Vec<(usize, Rgb, String)>,
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let tree = parse(body(source, &["treemap", "treemap-beta"])?)?;
    let width = 788.0;
    let span = width - 2.0 * MARGIN;
    let mut y = MARGIN;
    for heading in &tree.headings {
        for row in wrap(scene, heading, span) {
            scene.text(MARGIN, y, &row, theme(options, "titleColor"));
            y += ROW;
        }
    }
    let rect = Rect {
        x: MARGIN,
        y,
        w: span,
        h: 420.0,
    };
    let mut map = Map {
        scene,
        tree: &tree,
        styles: tree.styles(options),
        root_colors: tree
            .roots
            .iter()
            .enumerate()
            .map(|(index, &id)| (id, palette(index, options.color)))
            .collect(),
        legend: Vec::new(),
    };
    if tree.roots.iter().all(|&i| tree.nodes[i].weight == 0.0) {
        let style = options.theme_style();
        map.scene.rect(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            0.0,
            style.fill,
            style.stroke.or(Some(fade(map.scene.ink(), 0.40))),
        );
        map.forest(
            &tree.roots,
            Rect {
                w: 0.0,
                h: 0.0,
                ..rect
            },
            None,
        );
    } else {
        map.forest(&tree.roots, rect, None);
    }
    y += rect.h + 16.0;
    map.legend.sort_by_key(|(id, _, _)| *id);
    let slot = span / 2.0;
    for pair in map.legend.chunks(2) {
        let mut rows = 0;
        for (i, &(_, color, ref text)) in pair.iter().enumerate() {
            let x = MARGIN + slot * i as f32;
            let lines = wrap(map.scene, text, slot - 30.0);
            map.scene
                .rect(x, y + 5.0, 12.0, 12.0, 2.0, Some(color), None);
            for (n, row) in lines.iter().enumerate() {
                map.scene.text(x + 20.0, y + n as f32 * ROW, row, None);
            }
            rows = rows.max(lines.len());
        }
        y += rows as f32 * ROW;
    }
    map.scene.resize(width, y + MARGIN)
}

impl Map<'_> {
    fn forest(&mut self, ids: &[usize], rect: Rect, root: Option<usize>) {
        let mut sorted = ids.to_vec();
        sorted.sort_by(|&a, &b| {
            self.tree.nodes[b]
                .weight
                .total_cmp(&self.tree.nodes[a].weight)
        });
        self.place(&sorted, rect, root);
    }

    fn place(&mut self, ids: &[usize], rect: Rect, root: Option<usize>) {
        match ids {
            [] => {}
            &[id] => self.tile(id, rect, root.unwrap_or(id)),
            _ => {
                let (at, share) = split(ids, &self.tree.nodes);
                let (first, last) = if rect.w >= rect.h {
                    let edge = (f64::from(rect.w) * share) as f32;
                    (
                        Rect { w: edge, ..rect },
                        Rect {
                            x: rect.x + edge,
                            w: rect.w - edge,
                            ..rect
                        },
                    )
                } else {
                    let edge = (f64::from(rect.h) * share) as f32;
                    (
                        Rect { h: edge, ..rect },
                        Rect {
                            y: rect.y + edge,
                            h: rect.h - edge,
                            ..rect
                        },
                    )
                };
                self.place(&ids[..at], first, root);
                self.place(&ids[at..], last, root);
            }
        }
    }

    fn tile(&mut self, id: usize, rect: Rect, root: usize) {
        let node = &self.tree.nodes[id];
        let section = !node.children.is_empty();
        let style = self.styles[id];
        let base = self.root_colors[&root];
        let fill = style.fill.unwrap_or_else(|| {
            fade(
                base,
                if section {
                    0.24
                } else {
                    0.08 + (id % 4) as f32 * 0.07
                },
            )
        });
        let border = style.stroke.or(Some(fade(
            self.scene.ink(),
            if section { 0.60 } else { 0.28 },
        )));
        let ink = style.text.or_else(|| {
            let behind = if id == root {
                self.scene.paper()
            } else {
                self.styles[root].fill.unwrap_or(self.scene.paper())
            };
            Some(contrast(fill, behind))
        });
        let visible = node.weight > 0.0 && rect.w >= 1.0 && rect.h >= 1.0;
        if visible {
            let stroke = if rect.w >= 4.0 && rect.h >= 4.0 {
                border
            } else {
                None
            };
            self.scene
                .rect(rect.x, rect.y, rect.w, rect.h, 0.0, Some(fill), stroke);
        }
        let caption = if let Some(value) = &node.amount {
            format!("{}: {value}", node.name)
        } else {
            node.name.clone()
        };
        let key = format!("#{}", id + 1);
        let fits = |s: &str| visible && rect.h >= 25.0 && self.scene.measure(s) <= rect.w - 12.0;
        let center_x = rect.x + rect.w / 2.0;
        let label_y = if section {
            rect.y + 3.0
        } else {
            rect.y + (rect.h - 20.0) / 2.0
        };
        let header = if fits(&caption) {
            center(self.scene, center_x, label_y, &caption, ink);
            ROW
        } else if let Some(value) = &node.amount
            && rect.h >= 46.0
            && fits(&node.name)
            && fits(value)
        {
            center(self.scene, center_x, label_y - 10.0, &node.name, ink);
            center(self.scene, center_x, label_y + 10.0, value, ink);
            ROW
        } else {
            self.legend.push((
                id,
                style.fill.or(style.stroke).unwrap_or(base),
                format!("{key}  {caption}"),
            ));
            if fits(&key) {
                center(self.scene, center_x, label_y, &key, ink);
                ROW
            } else {
                0.0
            }
        };
        if section {
            let pad = if rect.w > 10.0 && rect.h > header + 10.0 {
                3.0
            } else {
                0.0
            };
            let interior = Rect {
                x: rect.x + pad,
                y: rect.y + pad + header,
                w: if visible {
                    (rect.w - 2.0 * pad).max(0.0)
                } else {
                    0.0
                },
                h: if visible {
                    (rect.h - 2.0 * pad - header).max(0.0)
                } else {
                    0.0
                },
            };
            self.forest(&node.children, interior, Some(root));
        }
    }
}
