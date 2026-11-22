use super::{Diagram, exact, parse};
use crate::{
    Options,
    charts::shared::plot::image::{
        MARGIN, ROW, center, circle, contrast, fade, line, palette, theme, wrap,
    },
    graphics::Scene,
};

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    scene.set_title_color(theme(options, "quadrantTitleFill"));
    let Diagram {
        title,
        captions,
        x_names,
        y_names,
        data,
        metadata,
    } = parse(source)?;
    let positions: Vec<_> = data
        .iter()
        .enumerate()
        .map(|(i, datum)| {
            let fill =
                theme(options, "quadrantPointFill").unwrap_or_else(|| palette(0, options.color));
            let (fill, stroke) = if options.color {
                (
                    datum.style.colors.fill.unwrap_or(fill),
                    datum.style.colors.stroke,
                )
            } else {
                (palette(i, false), None)
            };
            let radius = datum.style.radius.unwrap_or(7.0) as f32;
            let stroke_width =
                datum
                    .style
                    .stroke_width
                    .unwrap_or(if stroke.is_some() { 1.8 } else { 1.3 }) as f32;
            (
                fill,
                stroke.unwrap_or_else(|| scene.paper()),
                radius,
                stroke_width,
            )
        })
        .collect();
    let clearance = positions
        .iter()
        .map(|&(_, _, r, sw)| r + sw / 2.0)
        .fold(0.0, f32::max);
    let axis_gutter = 138.0;
    let left = MARGIN + axis_gutter + clearance;
    let width = left + 560.0 + clearance + MARGIN;
    let right = left + 560.0;
    let mid = f32::midpoint(left, right);
    let title = if title.is_empty() {
        Vec::new()
    } else {
        wrap(scene, &title, width - 2.0 * MARGIN)
    };
    let captions = captions.map(|caption| wrap(scene, &caption, 252.0));
    let y_titles = y_names.map(|name| wrap(scene, &name, axis_gutter - 20.0));
    let x_titles = x_names.map(|name| wrap(scene, &name, 250.0));
    let caption_height = captions.iter().map(Vec::len).max().unwrap_or(0) as f32 * ROW;
    let y_labels_height = y_titles.iter().map(Vec::len).sum::<usize>() as f32 * ROW;
    let plot_height = 420.0_f32
        .max(2.0 * caption_height + 70.0)
        .max(y_labels_height + 90.0);
    let top = MARGIN + clearance + title.len() as f32 * ROW;
    let bottom = top + plot_height;
    let middle = f32::midpoint(top, bottom);
    let ink = scene.ink();
    for (i, row) in title.iter().enumerate() {
        center(
            scene,
            mid,
            MARGIN + i as f32 * ROW,
            row,
            theme(options, "quadrantTitleFill").or_else(|| theme(options, "titleColor")),
        );
    }
    let mut occupied = Vec::new();
    for (i, ((x, y), caption)) in [(mid, top), (left, top), (left, middle), (mid, middle)]
        .into_iter()
        .zip(captions)
        .enumerate()
    {
        let fill = theme(options, &format!("quadrant{}Fill", i + 1)).unwrap_or_else(|| {
            fade(
                palette(i, options.color),
                if options.color {
                    0.16
                } else {
                    0.12 + (i % 2) as f32 * 0.13
                },
            )
        });
        scene.rect(x, y, 280.0, plot_height / 2.0, 0.0, Some(fill), None);
        let color = theme(options, &format!("quadrant{}TextFill", i + 1))
            .or_else(|| Some(contrast(fill, scene.paper())));
        let label_top = if i < 2 {
            y + 9.0
        } else {
            bottom - 7.0 - caption.len() as f32 * ROW
        };
        for (n, row) in caption.iter().enumerate() {
            let w = scene.measure(row);
            let y = label_top + n as f32 * ROW;
            center(scene, x + 140.0, y, row, color);
            occupied.push([x + 140.0 - w / 2.0, y, x + 140.0 + w / 2.0, y + 20.0]);
        }
    }
    let outer =
        theme(options, "quadrantExternalBorderStrokeFill").unwrap_or_else(|| fade(ink, 0.65));
    let inner =
        theme(options, "quadrantInternalBorderStrokeFill").unwrap_or_else(|| fade(ink, 0.45));
    scene.rect(left, top, 560.0, plot_height, 0.0, None, Some(outer));
    line(scene, (mid, top), (mid, bottom), inner, 1.0);
    line(scene, (left, middle), (right, middle), inner, 1.0);
    let tick_bottom = bottom + clearance + 5.0;
    for (i, tick) in ["0", "0.5", "1"].into_iter().enumerate() {
        let x = [left, mid, right][i];
        let y = [bottom, middle, top][i];
        line(scene, (x, bottom), (x, bottom + 5.0), outer, 1.0);
        line(scene, (left, y), (left - 5.0, y), outer, 1.0);
        center(scene, x, tick_bottom, tick, None);
        scene.text(
            left - clearance - 12.0 - scene.measure(tick),
            y - 10.0,
            tick,
            None,
        );
    }
    for i in 0..2 {
        let (x, y) = (
            [left, right][i],
            [bottom - y_titles[i].len() as f32 * ROW, top + 16.0][i],
        );
        for (n, row) in x_titles[i].iter().enumerate() {
            let x = if i == 0 { x } else { x - scene.measure(row) };
            scene.text(
                x,
                tick_bottom + ROW + n as f32 * ROW,
                row,
                theme(options, "quadrantXAxisTextFill"),
            );
        }
        for (n, row) in y_titles[i].iter().enumerate() {
            scene.text(
                left - clearance - 12.0 - scene.measure(row),
                y + n as f32 * ROW,
                row,
                theme(options, "quadrantYAxisTextFill"),
            );
        }
    }
    let at = |datum: &super::Datum| {
        (
            left + 560.0 * datum.x as f32,
            bottom - plot_height * datum.y as f32,
        )
    };
    for (datum, &(fill, stroke, r, sw)) in data.iter().zip(&positions) {
        circle(scene, at(datum), r, Some(fill), Some(stroke), sw);
    }
    let text_color = theme(options, "quadrantPointTextFill");
    let mut legend = Vec::new();
    for (i, (datum, &(fill, _, r, sw))) in data.iter().zip(&positions).enumerate() {
        let id = (i + 1).to_string();
        let w = scene.measure(&id);
        let (x, y) = at(datum);
        let offset = r + sw / 2.0 + 5.0;
        for (px, py) in [
            (x + offset, y - 10.0),
            (x - offset - w, y - 10.0),
            (x - w / 2.0, y - offset - 20.0),
            (x - w / 2.0, y + offset),
            (x + offset, y + offset),
            (x - offset - w, y + offset),
            (x + offset, y - offset - 20.0),
            (x - offset - w, y - offset - 20.0),
        ] {
            let overlap = occupied
                .iter()
                .any(|&[l, t, r, b]| l - 3.0 < px + w && px < r + 3.0 && t < py + 20.0 && py < b);
            if !overlap && px >= left && px + w <= right && py >= top && py + 20.0 <= bottom {
                scene.rect(
                    px - 2.0,
                    py + 1.0,
                    w + 4.0,
                    18.0,
                    3.0,
                    Some(scene.paper()),
                    None,
                );
                scene.text(px, py, &id, text_color);
                occupied.push([px, py, px + w, py + 20.0]);
                break;
            }
        }
        let class = datum
            .class
            .as_ref()
            .map(|name| format!(" [{name}]"))
            .unwrap_or_default();
        let caption = format!(
            "{id}  {}{class}  ({}, {})",
            datum.name,
            exact(datum.x),
            exact(datum.y)
        );
        legend.push((fill, caption));
    }
    let mut y =
        tick_bottom + (x_titles.iter().map(Vec::len).max().unwrap() + 1) as f32 * ROW + 14.0;
    let slot = (width - 2.0 * MARGIN) / 2.0;
    for pair in legend.chunks(2) {
        let mut rows = 0;
        for (i, (fill, text)) in pair.iter().enumerate() {
            let x = MARGIN + i as f32 * slot;
            let lines = wrap(scene, text, slot - 28.0);
            circle(scene, (x + 6.0, y + 10.0), 5.0, Some(*fill), None, 0.0);
            for (n, row) in lines.iter().enumerate() {
                scene.text(x + 18.0, y + n as f32 * ROW, row, text_color);
            }
            rows = rows.max(lines.len());
        }
        y += rows as f32 * ROW;
    }
    for note in metadata {
        for row in wrap(scene, &note, width - 2.0 * MARGIN) {
            scene.text(MARGIN, y, &row, None);
            y += ROW;
        }
    }
    scene.resize(width, y + MARGIN)
}
