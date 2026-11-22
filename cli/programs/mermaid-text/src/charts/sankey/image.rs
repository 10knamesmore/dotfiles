use super::{Flow, body, ranks, records, routing, single_line};
use crate::{
    Options,
    charts::shared::plot::image::{MARGIN, ROW, center, contrast, fade, palette, wrap},
    color::Rgb,
    graphics::Scene,
};

const BAR: f32 = 14.0;
const LABEL_GAP: f32 = 23.0;
const PORT_GAP: f32 = 10.0;

struct Block {
    top: f32,
    height: f32,
    incoming: f32,
    outgoing: f32,
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (mut labels, mut flows) = records(body(source, &["sankey", "sankey-beta"])?)?;
    let rank = ranks(&mut labels, &mut flows, options.ascii);
    let (stages, ports) = routing(&rank, &flows)?;
    let full: Vec<_> = labels.iter().map(|label| single_line(label)).collect();
    let labels: Vec<_> = full
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if scene.measure(label) > 166.0 {
                format!("#{}", i + 1)
            } else {
                label.clone()
            }
        })
        .collect();
    let gap = 155.0;
    let mut x = MARGIN + 70.0;
    let mut stage_x = Vec::new();
    let mut right = 0.0;
    for stage in &stages {
        let half = stage
            .iter()
            .filter_map(|&i| labels.get(i))
            .map(|label| scene.measure(label) / 2.0 + 8.0)
            .fold(68.0, f32::max);
        x = x.max(right + half);
        stage_x.push(x);
        right = x + half;
        x += gap;
    }
    let width = 500.0_f32.max(right + MARGIN);
    let shift = (width - (right + MARGIN)) / 2.0;
    for x in &mut stage_x {
        *x += shift;
    }

    let peak = flows.iter().map(|flow| flow.value).fold(0.0, f64::max);
    let weights: Vec<_> = flows
        .iter()
        .map(|flow| if peak > 0.0 { flow.value / peak } else { 0.0 })
        .collect();
    let sum = |links: &[(usize, usize)]| links.iter().map(|&(_, e)| weights[e]).sum::<f64>();
    let masses: Vec<_> = ports
        .iter()
        .map(|p| (sum(&p.incoming), sum(&p.outgoing)))
        .collect();
    let max_stage = stages
        .iter()
        .map(|stage| {
            stage
                .iter()
                .map(|&i| masses[i].0.max(masses[i].1))
                .sum::<f64>()
        })
        .fold(f64::MIN_POSITIVE, f64::max);
    let span = if stages.len() > 6 { 280.0 } else { 308.0 };
    let scaled = |mass: f64| (mass / max_stage * span) as f32;
    let thickness: Vec<_> = weights.into_iter().map(scaled).collect();

    let footprint = |i: usize| {
        let height = scaled(masses[i].0.max(masses[i].1));
        height
            + if ports[i].through.is_some() {
                0.0
            } else {
                LABEL_GAP
            }
    };
    let heights: Vec<_> = stages
        .iter()
        .map(|stage| stage.iter().map(|&i| footprint(i) + PORT_GAP).sum::<f32>() - PORT_GAP)
        .collect();
    let height = heights.iter().copied().fold(0.0, f32::max);
    let mut blocks = Vec::with_capacity(ports.len());
    blocks.resize_with(ports.len(), || Block {
        top: 0.0,
        height: 0.0,
        incoming: 0.0,
        outgoing: 0.0,
    });
    for (stage, stage_height) in stages.iter().zip(heights) {
        let mut y = MARGIN + (height - stage_height) / 2.0;
        for &i in stage {
            let (incoming, outgoing) = masses[i];
            let h = scaled(incoming.max(outgoing));
            let top = y + if ports[i].through.is_some() {
                0.0
            } else {
                LABEL_GAP
            };
            blocks[i] = Block {
                top,
                height: h,
                incoming: top + (h - scaled(incoming)) / 2.0,
                outgoing: top + (h - scaled(outgoing)) / 2.0,
            };
            y += footprint(i) + PORT_GAP;
        }
    }
    let style = options.theme_style();
    let node_color = |i| style.fill.unwrap_or_else(|| palette(i, options.color));
    let link = options.theme_line();
    let colors: Vec<_> = flows
        .iter()
        .map(|Flow { from, .. }| {
            fade(
                link.unwrap_or_else(|| node_color(*from)),
                if options.color { 0.58 } else { 0.48 },
            )
        })
        .collect();
    let name = |i: usize| {
        if labels[i] == full[i] {
            full[i].clone()
        } else {
            format!("{} {}", labels[i], full[i])
        }
    };
    let arrow = if options.ascii { " -> " } else { " → " };
    let slot = (width - 2.0 * MARGIN) / 2.0;
    let legend: Vec<_> = flows
        .iter()
        .map(|f| {
            wrap(
                scene,
                &format!("{}{arrow}{}: {}", name(f.from), name(f.to), f.amount),
                slot - 30.0,
            )
        })
        .collect();
    let row_height =
        |pair: &[Vec<String>]| pair.iter().map(Vec::len).max().unwrap_or(0) as f32 * ROW;
    let legend_top = height + MARGIN + 22.0;
    scene.resize(
        width,
        legend_top + legend.chunks(2).map(row_height).sum::<f32>() + MARGIN,
    )?;

    let mut in_y = vec![0.0; flows.len()];
    for (stage, (xs, xt)) in stages.windows(2).zip(
        stage_x
            .windows(2)
            .map(|x| (x[0] + BAR / 2.0, x[1] - BAR / 2.0)),
    ) {
        for &i in &stage[1] {
            let mut y = blocks[i].incoming;
            for &(_, edge) in &ports[i].incoming {
                in_y[edge] = y;
                y += thickness[edge];
            }
        }
        for &i in &stage[0] {
            let mut y = blocks[i].outgoing;
            for &(_, edge) in &ports[i].outgoing {
                let h = thickness[edge];
                if h > 0.0 {
                    ribbon(scene, (xs, y), (xt, in_y[edge]), h, colors[edge]);
                }
                y += h;
            }
        }
    }
    for (stage, &x) in stages.iter().zip(&stage_x) {
        for &i in stage {
            let Block { top, height, .. } = blocks[i];
            if let Some(edge) = ports[i].through {
                if height > 0.0 {
                    scene.rect(
                        x - BAR / 2.0,
                        top,
                        BAR,
                        height,
                        0.0,
                        Some(colors[edge]),
                        None,
                    );
                }
            } else {
                let fill = node_color(i);
                scene.rect(
                    x - BAR / 2.0,
                    top,
                    BAR,
                    height.max(1.0),
                    0.0,
                    Some(fill),
                    style.stroke,
                );
                let text = &labels[i];
                let w = scene.measure(text) + 10.0;
                scene.rect(
                    x - w / 2.0,
                    top - LABEL_GAP + 1.0,
                    w,
                    19.0,
                    3.0,
                    Some(style.fill.unwrap_or_else(|| scene.paper())),
                    None,
                );
                let ink = style
                    .text
                    .or_else(|| style.fill.map(|fill| contrast(fill, scene.paper())));
                center(scene, x, top - LABEL_GAP, text, ink);
            }
        }
    }
    let mut y = legend_top;
    for (pair, colors) in legend.chunks(2).zip(colors.chunks(2)) {
        for (i, (rows, &color)) in pair.iter().zip(colors).enumerate() {
            let x = MARGIN + slot * i as f32;
            scene.rect(x, y + 6.0, 15.0, 8.0, 0.0, Some(color), None);
            for (n, row) in rows.iter().enumerate() {
                scene.text(x + 22.0, y + n as f32 * ROW, row, None);
            }
        }
        y += row_height(pair);
    }
    Ok(())
}

fn ribbon(scene: &mut Scene, (x, y): (f32, f32), (u, v): (f32, f32), h: f32, color: Rgb) {
    if color.is_transparent() {
        return;
    }
    let a = x + (u - x) * 0.46;
    let b = u - (u - x) * 0.46;
    let (bottom, far_bottom) = (y + h, v + h);
    let top_edge = format!("M {x} {y} C {a} {y} {b} {v} {u} {v}");
    let bottom_edge = format!("C {b} {far_bottom} {a} {bottom} {x} {bottom}");
    scene.path(
        &format!("{top_edge} L {u} {far_bottom} {bottom_edge} Z"),
        Some(color),
        None,
        0.0,
    );
    scene.path(
        &format!("{top_edge} M {u} {far_bottom} {bottom_edge}"),
        None,
        Some(fade(scene.paper(), 0.36)),
        (h * 0.20).min(1.0),
    );
}
