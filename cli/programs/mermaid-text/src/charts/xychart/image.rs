use std::fmt::Write;

use crate::{
    Options,
    charts::shared::plot::{exact, parse_source},
    color::Rgb,
    graphics::Scene,
};

use super::{Chart, HEADERS, Series, palette, parse};

const PAD: f32 = 24.0;
const ROW: f32 = 23.0;
const TICK: f32 = 5.0;
const TICK_GAP: f32 = 6.0;

const PALETTE: &[&str] = &[
    "#568bd1", "#e59453", "#4aa89c", "#bb71a3", "#bd983b", "#8499b7", "#d2797b", "#899a58",
];
const MONOCHROME: &[&str] = &[
    "#606060", "#a6a6a6", "#797979", "#c4c4c4", "#919191", "#494949", "#b9b9b9",
];

type Point = (f32, f32);

struct Plot {
    horizontal: bool,
    left: f32,
    top: f32,
    slot: f32,
    category_span: f32,
    value_span: f32,
}

impl Plot {
    fn category(&self, chart: &Chart, index: usize, count: usize) -> f32 {
        self.slot / 2.0 + chart.category_offset(index, count, f64::from(self.slot)) as f32
    }

    fn point(&self, category: f32, value: f32) -> Point {
        if self.horizontal {
            (self.left + self.value_span * value, self.top + category)
        } else {
            (
                self.left + category,
                self.top + self.value_span * (1.0 - value),
            )
        }
    }

    fn bottom(&self) -> f32 {
        self.top
            + if self.horizontal {
                self.category_span
            } else {
                self.value_span
            }
    }
}

struct AxisColors {
    line: Option<Rgb>,
    tick: Option<Rgb>,
    label: Option<Rgb>,
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let plot = parse_source(source)?;
    if !HEADERS.contains(&plot.kind) {
        return Err(format!("expected an XY chart, got {}", plot.kind));
    }
    let mut chart = parse(&plot.statements)?;
    let horizontal = plot.horizontal;
    let metadata = &plot.metadata;
    if let Some((start, end)) = chart.x_range {
        chart.labels[0] = exact(start);
        if chart.labels.len() > 1 {
            *chart.labels.last_mut().unwrap() = exact(end);
        }
    }
    let theme = |key: &str| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let visible = |rgb: &Rgb| !rgb.is_transparent();
    let background = theme("xyChart.backgroundColor").filter(visible);
    let paper = background.unwrap_or_else(|| scene.paper());
    let ink = background.map_or_else(
        || scene.ink(),
        |background| contrasting_ink(background, scene.paper()),
    );
    let axes = ["x", "y"].map(|axis| {
        let line = theme(&format!("xyChart.{axis}AxisLineColor")).or(Some(ink));
        AxisColors {
            line,
            tick: theme(&format!("xyChart.{axis}AxisTickColor")).or(line),
            label: theme(&format!("xyChart.{axis}AxisLabelColor")).or(Some(ink)),
        }
    });
    let grid = theme("xyChart.yAxisTickColor").or_else(|| Rgb::parse("#ababab"));
    let legend_color = theme("xyChart.legendTextColor").or(Some(ink));
    let declared_colors = palette(options);
    let default_colors = if options.color { PALETTE } else { MONOCHROME };
    let series_colors: Vec<_> = (0..chart.series.len())
        .map(|index| {
            declared_colors
                .get(index % declared_colors.len().max(1))
                .copied()
                .flatten()
                .or_else(|| Rgb::parse(default_colors[index % default_colors.len()]))
                .filter(visible)
        })
        .collect();

    let mut ticks = chart.ticks();
    for (value, text) in &mut ticks {
        if *value == chart.lo || *value == chart.hi {
            *text = exact(*value);
        }
    }
    let label_width = chart
        .labels
        .iter()
        .map(|text| scene.measure(text))
        .fold(0.0, f32::max);
    let tick_width = ticks
        .iter()
        .map(|(_, text)| scene.measure(text))
        .fold(0.0, f32::max);
    let (above, above_color, below, below_color) = if horizontal {
        (
            &chart.x_name,
            theme("xyChart.xAxisTitleColor"),
            &chart.y_name,
            theme("xyChart.yAxisTitleColor"),
        )
    } else {
        (
            &chart.y_name,
            theme("xyChart.yAxisTitleColor"),
            &chart.x_name,
            theme("xyChart.xAxisTitleColor"),
        )
    };
    let n = chart.labels.len() as f32;
    let slot = (chart.bars as f32 * 12.0 + 16.0).max(if horizontal {
        40.0
    } else {
        (label_width + 16.0).max(520.0 / n)
    });
    let left_margin = if horizontal {
        label_width.max(tick_width / 2.0)
    } else {
        tick_width
    };
    let left = PAD + left_margin + TICK + TICK_GAP;
    let top =
        PAD + if chart.title.is_empty() {
            0.0
        } else {
            ROW + 12.0
        } + if above.is_empty() { 0.0 } else { ROW + 10.0 };
    let plot = Plot {
        horizontal,
        left,
        top,
        slot,
        category_span: slot * n,
        value_span: if horizontal {
            520.0_f32.max((tick_width + 16.0) * 4.0)
        } else {
            280.0
        },
    };
    let plot_width = if horizontal {
        plot.value_span
    } else {
        plot.category_span
    };
    let right_margin = if horizontal { tick_width / 2.0 } else { 0.0 };
    let clipping_note = chart.clipping_note();
    let content_width = [&chart.title, above, below]
        .into_iter()
        .chain(metadata)
        .chain(clipping_note.iter())
        .map(|text| scene.measure(&text.replace('\t', " ")))
        .fold(left - PAD + plot_width + right_margin, f32::max)
        .max(
            chart
                .series
                .iter()
                .map(|series| scene.measure(&format!("{}:", series.name)) + 46.0)
                .fold(0.0, f32::max),
        );
    let width = (content_width + 2.0 * PAD).ceil();
    let legend: Vec<_> = chart
        .series
        .iter()
        .map(|series| legend_rows(scene, series, content_width - 46.0))
        .collect();
    let legend_height = legend
        .iter()
        .map(|rows| rows.len() as f32 * ROW + 4.0)
        .sum::<f32>();
    let labels_top = plot.bottom() + TICK + TICK_GAP;
    let below_top = labels_top + ROW + 8.0;
    let legend_top = below_top + if below.is_empty() { 0.0 } else { ROW + 8.0 };
    let metadata_top = legend_top + legend_height + 4.0;
    let footer_rows = metadata.len() + usize::from(clipping_note.is_some());
    let height = (metadata_top + footer_rows as f32 * ROW + PAD).ceil();
    scene.resize(width, height)?;

    if let Some(background) = background {
        scene.rect(0.0, 0.0, width, height, 0.0, Some(background), None);
    }
    if !chart.title.is_empty() {
        scene.text(
            (width - scene.measure(&chart.title)) / 2.0,
            PAD,
            &chart.title,
            theme("xyChart.titleColor").or(Some(ink)),
        );
    }
    if !above.is_empty() {
        scene.text(
            left.min(width - PAD - scene.measure(above)),
            top - ROW - 10.0,
            above,
            above_color.or(Some(ink)),
        );
    }
    if !below.is_empty() {
        let x = (left + (plot_width - scene.measure(below)) / 2.0).max(PAD);
        scene.text(
            x.min(width - PAD - scene.measure(below)),
            below_top,
            below,
            below_color.or(Some(ink)),
        );
    }

    draw_grid(scene, &plot, &chart, ticks, &axes, grid);
    draw_series(scene, &plot, &chart, &series_colors, paper);

    let mut row_top = legend_top;
    for ((series, rows), color) in chart.series.iter().zip(legend).zip(series_colors) {
        if series.bar {
            scene.rect(PAD + 2.0, row_top + 5.0, 26.0, 10.0, 0.0, color, None);
        } else {
            let y = row_top + 10.0;
            line(scene, (PAD, y), (PAD + 30.0, y), color, 2.4);
            marker(scene, (PAD + 15.0, y), series.index, color, paper);
        }
        for row in rows {
            scene.text(PAD + 46.0, row_top, &row, legend_color);
            row_top += ROW;
        }
        row_top += 4.0;
    }
    for (row, text) in clipping_note.iter().chain(metadata).enumerate() {
        scene.text(
            PAD,
            metadata_top + row as f32 * ROW,
            &text.replace('\t', " "),
            Some(ink),
        );
    }
    Ok(())
}

fn draw_grid(
    scene: &mut Scene,
    plot: &Plot,
    chart: &Chart,
    ticks: Vec<(f64, String)>,
    [x_axis, y_axis]: &[AxisColors; 2],
    grid: Option<Rgb>,
) {
    let labels_top = plot.bottom() + TICK + TICK_GAP;
    let mut ticks: Vec<_> = ticks
        .into_iter()
        .map(|(value, text)| (chart.value_fraction(value) as f32, value, text))
        .collect();
    ticks.sort_by_key(|(_, value, _)| {
        if *value == chart.lo || *value == chart.hi {
            0
        } else if *value == 0.0 {
            1
        } else {
            2
        }
    });
    let value_axis_category = if plot.horizontal {
        plot.category_span
    } else {
        0.0
    };
    let mut tick_labels: Vec<(f32, f32)> = Vec::new();
    for (fraction, value, text) in &ticks {
        let a = plot.point(0.0, *fraction);
        let b = plot.point(plot.category_span, *fraction);
        let zero = *value == 0.0;
        if *fraction > 0.0 {
            line(
                scene,
                a,
                b,
                if zero { y_axis.line } else { grid },
                if zero { 1.4 } else { 0.6 },
            );
        }
        let (x, y) = plot.point(value_axis_category, *fraction);
        let text_width = scene.measure(text);
        let interval = if plot.horizontal {
            (x - text_width / 2.0, x + text_width / 2.0)
        } else {
            (y - 10.0, y + 10.0)
        };
        if tick_labels
            .iter()
            .any(|&(start, end)| interval.0 < end + 4.0 && start < interval.1 + 4.0)
        {
            continue;
        }
        tick_labels.push(interval);
        if plot.horizontal {
            line(scene, (x, y), (x, y + TICK), y_axis.tick, 1.0);
            scene.text(interval.0, labels_top, text, y_axis.label);
        } else {
            line(scene, (x, y), (x - TICK, y), y_axis.tick, 1.0);
            scene.text(
                x - TICK - TICK_GAP - text_width,
                interval.0,
                text,
                y_axis.label,
            );
        }
    }
    for (i, text) in chart.labels.iter().enumerate() {
        let (x, y) = plot.point(plot.category(chart, i, chart.labels.len()), 0.0);
        let text_width = scene.measure(text);
        if plot.horizontal {
            line(scene, (x, y), (x - TICK, y), x_axis.tick, 1.0);
            scene.text(
                x - TICK - TICK_GAP - text_width,
                y - 10.0,
                text,
                x_axis.label,
            );
        } else {
            line(scene, (x, y), (x, y + TICK), x_axis.tick, 1.0);
            scene.text(x - text_width / 2.0, labels_top, text, x_axis.label);
        }
    }
    line(
        scene,
        plot.point(0.0, 0.0),
        plot.point(plot.category_span, 0.0),
        x_axis.line,
        1.4,
    );
    line(
        scene,
        plot.point(value_axis_category, 0.0),
        plot.point(value_axis_category, 1.0),
        y_axis.line,
        1.4,
    );
}

fn draw_series(scene: &mut Scene, plot: &Plot, chart: &Chart, colors: &[Option<Rgb>], paper: Rgb) {
    let fraction = |value| chart.value_fraction(value) as f32;
    let bar_width = (plot.slot * 0.76 / chart.bars.max(1) as f32).min(36.0);
    for (series, color) in chart
        .series
        .iter()
        .zip(colors.iter().copied())
        .filter(|(series, _)| series.bar)
    {
        let bar_offset = bar_width * (series.index as f32 - chart.bars as f32 / 2.0 + 0.5);
        let base = fraction(0.0);
        for (i, &value) in series.values.iter().enumerate() {
            let category = plot.category(chart, i, series.values.len()) + bar_offset;
            let value = fraction(value);
            let (x0, y0) = plot.point(category - bar_width / 2.0 + 1.0, base);
            let (x1, y1) = plot.point(category + bar_width / 2.0 - 1.0, value);
            if base != value {
                scene.rect(
                    x0.min(x1),
                    y0.min(y1),
                    (x1 - x0).abs(),
                    (y1 - y0).abs(),
                    0.0,
                    color,
                    None,
                );
            }
        }
    }

    for (series, color) in chart
        .series
        .iter()
        .zip(colors.iter().copied())
        .filter(|(series, _)| !series.bar)
    {
        let points: Vec<_> = series
            .values
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                plot.point(
                    plot.category(chart, i, series.values.len()),
                    fraction(value),
                )
            })
            .collect();
        if let Some((&(x, y), rest)) = points.split_first().filter(|(_, rest)| !rest.is_empty()) {
            let mut path = format!("M {x} {y}");
            for &(x, y) in rest {
                write!(&mut path, " L {x} {y}").unwrap();
            }
            scene.path(&path, None, color, 2.4);
        }
        for point in points {
            marker(scene, point, series.index, color, paper);
        }
    }
}

fn legend_rows(scene: &Scene, series: &Series, width: f32) -> Vec<String> {
    let mut rows = Vec::new();
    let mut line = format!("{}:", series.name);
    for &value in &series.values {
        let value = exact(value);
        let joined = format!("{line}  {value}");
        if scene.measure(&joined) > width {
            rows.push(std::mem::replace(&mut line, value));
        } else {
            line = joined;
        }
    }
    rows.push(line);
    rows
}

fn line(scene: &mut Scene, (x0, y0): Point, (x1, y1): Point, color: Option<Rgb>, width: f32) {
    scene.path(&format!("M {x0} {y0} L {x1} {y1}"), None, color, width);
}

fn marker(scene: &mut Scene, (x, y): Point, index: usize, color: Option<Rgb>, paper: Rgb) {
    if color.is_none() {
        return;
    }
    let r = 4.8;
    match index % 4 {
        0 | 2 => {
            let corner = if index.is_multiple_of(4) { r } else { 0.0 };
            scene.rect(x - r, y - r, 2.0 * r, 2.0 * r, corner, color, Some(paper));
        }
        1 => scene.path(
            &format!(
                "M {x} {} L {} {y} L {x} {} L {} {y} Z",
                y - r,
                x + r,
                y + r,
                x - r
            ),
            color,
            Some(paper),
            1.0,
        ),
        _ => scene.path(
            &format!(
                "M {x} {} L {} {} L {} {} Z",
                y - r,
                x + r,
                y + r,
                x - r,
                y + r
            ),
            color,
            Some(paper),
            1.0,
        ),
    }
}

fn contrasting_ink(background: Rgb, paper: Rgb) -> Rgb {
    let [r, g, b, alpha] = background.rgba().map(|v| f32::from(v) / 255.0);
    let [pr, pg, pb, pa] = paper.rgba().map(|v| f32::from(v) / 255.0);
    let linear = |foreground, underlying| {
        let on_white = underlying * pa + 1.0 - pa;
        let value: f32 = foreground * alpha + on_white * (1.0 - alpha);
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(r, pr) + 0.7152 * linear(g, pg) + 0.0722 * linear(b, pb);
    if luminance > 0.18 {
        Rgb::from_rgba(22, 22, 22, 255)
    } else {
        Rgb::from_rgba(247, 247, 247, 255)
    }
}
