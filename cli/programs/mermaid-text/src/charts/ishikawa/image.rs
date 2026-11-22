use super::{HEADERS, Node, board, fish_style, parse};
use crate::{Options, color::Style, graphics::Scene};
use board::image::{Label, PAD, text_on_fill, tint};

const RIB: f32 = 58.0;
const TWIG: f32 = 22.0;
const ROW: f32 = 29.0;
const INSET: f32 = 12.0;
const HEADING: f32 = 36.0;

struct Bone<'a> {
    index: usize,
    nodes: &'a [Node<&'a str>],
    labels: Vec<Label>,
    width: f32,
    height: f32,
}

impl<'a> Bone<'a> {
    fn new(scene: &Scene, index: usize, nodes: &'a [Node<&'a str>]) -> Self {
        let labels: Vec<_> = nodes
            .iter()
            .map(|node| Label::new(scene, node.label))
            .collect();
        let width = nodes
            .iter()
            .zip(&labels)
            .map(|(node, label)| {
                if node.depth == 0 {
                    label.width + 2.0 * INSET
                } else {
                    label.width + INSET + node.depth as f32 * TWIG
                }
            })
            .fold(0.0, f32::max)
            + RIB;
        let height = 49.0 + (nodes.len() - 1) as f32 * ROW;
        Self {
            index,
            nodes,
            labels,
            width,
            height,
        }
    }

    fn draw(&self, scene: &mut Scene, x: f32, spine: f32, side: f32, style: Style) {
        let rib_x = x - RIB;
        let outside = spine + side * self.height;
        let along = |index: usize| outside - side * (index as f32 * ROW + 3.0);
        scene.path(
            &format!("M {rib_x} {outside} L {x} {spine}"),
            None,
            style.stroke,
            2.0,
        );
        for (i, (node, label)) in self.nodes.iter().zip(&self.labels).enumerate().skip(1) {
            let y = along(i);
            let right = rib_x - node.depth as f32 * TWIG;
            let parent = node.parent.expect("non-root cause has a parent") - self.index;
            let path = if parent == 0 {
                let junction = rib_x + RIB * ((outside - y) * side / self.height);
                format!("M {right} {y} H {junction}")
            } else {
                let elbow = right + TWIG + 8.0;
                format!("M {right} {y} H {elbow} V {}", along(parent))
            };
            scene.path(&path, None, style.stroke, 1.4);
            label.draw(
                scene,
                right - label.width - 7.0,
                y - label.height / 2.0,
                None,
            );
        }
        let category = &self.labels[0];
        let w = category.width + 2.0 * INSET;
        scene.rect(
            rib_x - w,
            outside - HEADING / 2.0,
            w,
            HEADING,
            6.0,
            style.fill,
            style.stroke,
        );
        category.draw(
            scene,
            rib_x - w + INSET,
            outside - category.height / 2.0,
            style.text,
        );
    }
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let fish = parse(board::body(source, HEADERS)?)?;
    let effect = Label::new(scene, fish.effect);
    let bones: Vec<_> = fish
        .categories()
        .map(|(index, nodes)| Bone::new(scene, index, nodes))
        .collect();
    let extent = |first| {
        bones
            .iter()
            .skip(first)
            .step_by(2)
            .map(|bone| bone.height + HEADING / 2.0)
            .fold(HEADING / 2.0, f32::max)
    };
    let spine = PAD + extent(0);
    let tail = PAD + 26.0;
    let mut root_x = tail + 10.0;
    let stations: Vec<_> = bones
        .chunks(2)
        .map(|pair| {
            root_x += pair.iter().map(|bone| bone.width).fold(0.0, f32::max) + 12.0;
            root_x
        })
        .collect();
    let head = root_x + 44.0;
    let head_w = effect.width + 2.0 * INSET;
    scene.resize(head + head_w + PAD, spine + extent(1) + PAD)?;

    let ink = scene.ink();
    let style = fish_style(options);
    let line = style.stroke.unwrap_or_else(|| tint(ink, 190));
    let style = Style {
        fill: style.fill.or(Some(tint(ink, 18))),
        stroke: Some(line),
        text: text_on_fill(scene, options, style.fill),
    };
    let arrow_base = head - 12.0;
    let top = spine - 12.0;
    let bottom = spine + 12.0;
    scene.path(
        &format!("M {PAD} {top} L {tail} {spine} L {PAD} {bottom} M {tail} {spine} H {arrow_base}"),
        None,
        Some(line),
        2.5,
    );
    scene.path(
        &format!(
            "M {arrow_base} {} L {head} {spine} L {arrow_base} {} Z",
            spine - 6.0,
            spine + 6.0
        ),
        Some(line),
        None,
        0.0,
    );
    for (pair, station) in bones.chunks(2).zip(stations) {
        for (bone, side) in pair.iter().zip([-1.0, 1.0]) {
            bone.draw(scene, station, spine, side, style);
        }
    }
    scene.rect(
        head,
        spine - HEADING / 2.0,
        head_w,
        HEADING,
        7.0,
        style.fill,
        style.stroke,
    );
    effect.draw(scene, head + INSET, spine - effect.height / 2.0, style.text);
    Ok(())
}
