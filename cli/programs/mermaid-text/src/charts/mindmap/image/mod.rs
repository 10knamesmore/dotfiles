use super::{Node, Shape, palette, parse};
use crate::{
    Options,
    charts::shared::board::image::{Label, PAD, text_on_fill, tint},
    color::{Rgb, Style},
    graphics::Scene,
};

mod shape;

const GAP_X: f32 = 48.0;
const GAP_Y: f32 = 12.0;
const ACCENTS: [[u8; 3]; 6] = [
    [83, 148, 211],
    [61, 164, 142],
    [212, 151, 77],
    [171, 133, 205],
    [202, 119, 129],
    [142, 168, 92],
];

struct Tile {
    title: Label,
    tags: Vec<Label>,
    width: f32,
    height: f32,
    slot: usize,
    side: usize,
    x: f32,
    y: f32,
}

impl Tile {
    fn new(scene: &Scene, node: &Node<'_>, slot: usize, side: usize) -> Self {
        let title = Label::new(scene, &node.label);
        let tags: Vec<_> = [("icon", node.icon), ("class", node.classes)]
            .into_iter()
            .filter_map(|(key, value)| {
                value.map(|value| Label::new(scene, &format!("{key}: {value}")))
            })
            .collect();
        let width = tags.iter().map(|l| l.width).fold(title.width, f32::max);
        let height = title.height + tags.iter().map(|l| l.height).sum::<f32>();
        let (width, height) = match node.shape {
            Shape::Circle => {
                let diameter = width.hypot(height) + 20.0;
                (diameter, diameter)
            }
            Shape::Bang | Shape::Cloud => (width + 44.0, height + 30.0),
            Shape::Hexagon => (width + 42.0, height + 16.0),
            _ => (width + 24.0, height + 14.0),
        };
        Self {
            title,
            tags,
            width,
            height,
            slot,
            side,
            x: 0.0,
            y: 0.0,
        }
    }

    fn draw(&self, scene: &mut Scene, shape: Shape, style: Style) {
        shape::draw(
            scene,
            shape,
            (self.x, self.y),
            (self.width, self.height),
            style,
        );
        let height = self.title.height + self.tags.iter().map(|l| l.height).sum::<f32>();
        let mut y = self.y - height / 2.0;
        self.title.center(scene, self.x, y, style.text);
        y += self.title.height;
        for tag in &self.tags {
            tag.center(scene, self.x, y, style.text);
            y += tag.height;
        }
    }
}

struct Tier {
    width: f32,
    inner_edge: f32,
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let nodes = parse(source)?;
    let palette = palette(options);
    let mut tiles: Vec<Tile> = Vec::with_capacity(nodes.len());
    let mut tiers: [Vec<Tier>; 2] = std::array::from_fn(|_| Vec::new());
    let mut branches = 0;
    for node in &nodes {
        let (slot, side) = match node.parent {
            None => (0, 0),
            Some(0) => {
                let pair = (branches % (palette.len() - 1) + 1, branches % 2);
                branches += 1;
                pair
            }
            Some(parent) => (tiles[parent].slot, tiles[parent].side),
        };
        let tile = Tile::new(scene, node, slot, side);
        if node.depth > 0 {
            let tier = node.depth - 1;
            tiers[side].resize_with(tiers[side].len().max(node.depth), || Tier {
                width: 0.0,
                inner_edge: 0.0,
            });
            tiers[side][tier].width = tiers[side][tier].width.max(tile.width);
        }
        tiles.push(tile);
    }

    let mut total_width = [0.0; 2];
    for (side, tiers) in tiers.iter_mut().enumerate() {
        let mut extent = tiles[0].width / 2.0;
        for tier in tiers {
            tier.inner_edge = extent + GAP_X;
            extent = tier.inner_edge + tier.width;
        }
        total_width[side] = extent;
    }
    let center = PAD + total_width[1];

    let mut spans = vec![0.0; nodes.len()];
    let mut child_spans = vec![0.0; nodes.len()];
    let mut side_spans = [0.0; 2];
    for (i, node) in nodes.iter().enumerate().rev() {
        let span = tiles[i].height.max(child_spans[i]);
        spans[i] = span;
        if let Some(parent) = node.parent {
            let sum = if parent == 0 {
                &mut side_spans[tiles[i].side]
            } else {
                &mut child_spans[parent]
            };
            *sum += span + if *sum > 0.0 { GAP_Y } else { 0.0 };
        }
    }
    let height = side_spans.into_iter().fold(tiles[0].height, f32::max);
    let mut side_top = side_spans.map(|span| PAD + (height - span) / 2.0);
    let mut child_top = vec![0.0; nodes.len()];
    tiles[0].x = center;
    tiles[0].y = PAD + height / 2.0;
    for (i, node) in nodes.iter().enumerate().skip(1) {
        let parent = node.parent.expect("non-root mindmap node has a parent");
        let tile = &mut tiles[i];
        let top = if parent == 0 {
            &mut side_top[tile.side]
        } else {
            &mut child_top[parent]
        };
        let tier = &tiers[tile.side][node.depth - 1];
        let direction = [1.0, -1.0][tile.side];
        tile.x = center + direction * (tier.inner_edge + tier.width / 2.0);
        tile.y = *top + spans[i] / 2.0;
        *top += spans[i] + GAP_Y;
        child_top[i] = tile.y - child_spans[i] / 2.0;
    }
    scene.resize(center + total_width[0] + PAD, height + 2.0 * PAD)?;

    let ink = scene.ink();
    let line = options.theme_line();
    let styles: [_; 12] = std::array::from_fn(|slot| {
        let paint = palette[slot];
        let default = if options.color && slot > 0 {
            let [r, g, b] = ACCENTS[(slot - 1) % ACCENTS.len()];
            Rgb::from_rgba(r, g, b, 255)
        } else {
            tint(ink, 160)
        };
        let stroke = paint.bg.or(line).unwrap_or(default);
        Style {
            stroke: Some(stroke),
            fill: paint
                .bg
                .or(Some(tint(stroke, if slot == 0 { 42 } else { 25 }))),
            text: paint.fg.or_else(|| text_on_fill(scene, options, paint.bg)),
        }
    });
    for (node, tile) in nodes.iter().zip(&tiles) {
        let Some(parent) = node.parent else { continue };
        let direction = [1.0, -1.0][tile.side];
        let wall = center + direction * tiers[tile.side][node.depth - 1].inner_edge;
        let near = wall - direction * GAP_X;
        let mid = (near + wall) / 2.0;
        let parent = &tiles[parent];
        let sx = parent.x + direction * parent.width / 2.0;
        let ex = tile.x - direction * tile.width / 2.0;
        let (sy, ey) = (parent.y, tile.y);
        scene.path(
            &format!("M {sx} {sy} H {near} C {mid} {sy} {mid} {ey} {wall} {ey} H {ex}"),
            None,
            styles[tile.slot].stroke,
            if node.depth == 1 { 2.5 } else { 1.6 },
        );
    }
    for (node, tile) in nodes.iter().zip(&tiles) {
        tile.draw(scene, node.shape, styles[tile.slot]);
    }
    Ok(())
}
