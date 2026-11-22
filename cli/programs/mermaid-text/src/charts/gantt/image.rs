use std::fmt::Write;

use super::{
    Chart, DAY, Dependency, Schedule, Tags,
    plot::{Interval, Palette, captions, date_label, time_domain},
};
use crate::{
    Options,
    charts::shared::history::{IMAGE_LINE, IMAGE_PAD, ImageLabel, dashed_image_path},
    color::{Rgb, Style},
    graphics::Scene,
};

const TASK_HEIGHT: f32 = 42.0;
const BAR: f32 = 16.0;
const PLOT_PAD: f32 = 16.0;
const AXIS_HEIGHT: f32 = 32.0;

struct Axis {
    first: i64,
    last: i64,
    left: f32,
    width: f32,
    first_label: ImageLabel,
    last_label: ImageLabel,
}

impl Axis {
    fn x(&self, time: i64) -> f32 {
        self.left
            + if self.last == self.first {
                self.width / 2.0
            } else {
                ((time - self.first) as f64 / (self.last - self.first) as f64
                    * f64::from(self.width)) as f32
            }
    }

    fn time(&self, x: f32) -> i64 {
        self.first + ((self.last - self.first) as f64 * f64::from(x) / f64::from(self.width)) as i64
    }

    fn right(&self) -> f32 {
        self.left + self.width
    }
}

struct Plot {
    axis: Axis,
    top: f32,
    bottom: f32,
}

struct TaskRow {
    label: ImageLabel,
    style: Style,
    center: f32,
    height: f32,
    start_x: f32,
    end_x: f32,
    position: f32,
}

impl TaskRow {
    fn shape(&self, tags: &Tags) -> (f32, f32) {
        if tags.milestone {
            (self.position - 8.0, self.position + 8.0)
        } else if tags.vert {
            (self.position - 3.0, self.position + 3.0)
        } else {
            (self.start_x - 1.0, self.end_x.max(self.start_x + 2.0) + 1.0)
        }
    }

    fn draw(&self, scene: &mut Scene, tags: &Tags, vert: Rgb) {
        self.label.draw(
            scene,
            IMAGE_PAD + 10.0,
            self.center - self.label.height() / 2.0,
            self.style.text,
        );
        let (x, y) = (self.position, self.center);
        if tags.milestone {
            diamond(scene, x, y, self.style.fill, self.style.stroke);
        } else if tags.vert {
            scene.rect(x - 3.0, y - 3.0, 6.0, 6.0, 3.0, Some(vert), None);
        } else if self.end_x == self.start_x {
            line(
                scene,
                (self.start_x, y - BAR / 2.0),
                (self.start_x, y + BAR / 2.0),
                self.style.stroke,
                3.0,
            );
        } else {
            scene.rect(
                self.start_x,
                y - BAR / 2.0,
                (self.end_x - self.start_x).max(1.0),
                BAR,
                4.0,
                self.style.fill,
                self.style.stroke,
            );
        }
    }
}

struct SectionRow {
    label: ImageLabel,
    y: f32,
}

#[derive(Clone, Copy)]
enum Key {
    After,
    Until,
    Today,
    Milestone,
    Excluded(bool),
    Vertical,
}

impl Key {
    fn text(self) -> &'static str {
        match self {
            Self::After => "After",
            Self::Until => "Until",
            Self::Today => "Today",
            Self::Milestone => "Milestone",
            Self::Excluded(true) => "Excluded days",
            Self::Excluded(false) => "Calendar exclusions applied",
            Self::Vertical => "Vertical marker",
        }
    }
}

fn fade(color: Rgb, opacity: f32) -> Rgb {
    let [r, g, b, alpha] = color.rgba();
    Rgb::from_rgba(r, g, b, (f32::from(alpha) * opacity).round() as u8)
}

fn line(
    scene: &mut Scene,
    (x, y): (f32, f32),
    (end_x, end_y): (f32, f32),
    color: Option<Rgb>,
    width: f32,
) {
    scene.path(&format!("M {x} {y} L {end_x} {end_y}"), None, color, width);
}

fn diamond(scene: &mut Scene, x: f32, y: f32, fill: Option<Rgb>, stroke: Option<Rgb>) {
    scene.path(
        &format!("M {x} {} l 8 8 -8 8 -8 -8 Z", y - 8.0),
        fill,
        stroke,
        1.6,
    );
}

fn task_style(
    palette: &Palette,
    tags: &Tags,
    clickable: bool,
    options: &Options,
    ink: Rgb,
) -> Style {
    let style = palette.task_style(tags, clickable);
    let accent = if options.color {
        match (tags.crit, tags.active, tags.done) {
            (true, _, _) => Rgb::from_rgba(242, 130, 144, 255),
            (_, true, _) => Rgb::from_rgba(108, 182, 241, 255),
            (_, _, true) => Rgb::from_rgba(110, 196, 158, 255),
            _ => Rgb::from_rgba(171, 174, 218, 255),
        }
    } else {
        ink
    };
    let stroke = style.stroke.unwrap_or(accent);
    Style {
        stroke: Some(stroke),
        fill: style
            .fill
            .or(Some(fade(stroke, if tags.done { 0.23 } else { 0.53 }))),
        text: style.text,
    }
}

impl Plot {
    fn grid(
        &self,
        chart: &Chart<'_>,
        format: &str,
        scene: &mut Scene,
        grid: Rgb,
        excluded: Option<Rgb>,
    ) -> Result<(), String> {
        let axis = &self.axis;
        if let Some(fill) = excluded {
            let mut day = axis.first - axis.first.rem_euclid(DAY);
            while day < axis.last {
                let start = day.max(axis.first);
                let end = (day + DAY).min(axis.last);
                if chart.calendar.excluded(start)? {
                    let x = axis.x(start);
                    scene.rect(
                        x,
                        self.top,
                        axis.x(end) - x,
                        self.bottom - self.top,
                        0.0,
                        Some(fill),
                        None,
                    );
                }
                day += DAY;
            }
        }
        if axis.first == axis.last {
            let x = axis.x(axis.first);
            line(scene, (x, self.top), (x, self.bottom), Some(grid), 1.0);
            self.axis_label(scene, chart.top_axis, x, &axis.first_label);
        } else {
            let label_width = axis.first_label.width.max(axis.last_label.width);
            let spacing = ((axis.last - axis.first) as f64
                / f64::from((axis.width / (label_width + 16.0)).max(1.0)))
                as i64;
            let interval = chart.ticks.unwrap_or_else(|| {
                Interval::automatic(if format == "%Y-%m-%d" {
                    spacing.max(DAY)
                } else {
                    spacing
                })
            });
            self.axis_label(scene, chart.top_axis, axis.left, &axis.first_label);
            self.axis_label(scene, chart.top_axis, axis.right(), &axis.last_label);
            let mut used_label = axis.left + axis.first_label.width / 2.0 + 12.0;
            let label_end = axis.right() - axis.last_label.width / 2.0 - 12.0;
            let mut last_tick = axis.left;
            let mut last_label = date_label(axis.first, format)?;
            let end_label = date_label(axis.last, format)?;
            for px in 0..axis.width.ceil() as usize {
                let Some(tick) = interval.ceil(axis.time(px as f32), chart.weekday) else {
                    break;
                };
                if tick >= axis.last {
                    break;
                }
                let x = axis.x(tick);
                if x <= last_tick + 2.0 || x >= axis.right() - 2.0 {
                    continue;
                }
                line(
                    scene,
                    (x, self.top),
                    (x, self.bottom),
                    Some(fade(grid, 0.55)),
                    1.0,
                );
                last_tick = x;
                if x < used_label || x > label_end {
                    continue;
                }
                let caption = date_label(tick, format)?;
                if caption == last_label || caption == end_label {
                    continue;
                }
                let caption = ImageLabel::new(scene, &caption);
                let half = caption.width / 2.0;
                if x - half > used_label && x + half < label_end {
                    self.axis_label(scene, chart.top_axis, x, &caption);
                    used_label = x + half + 12.0;
                    last_label = date_label(tick, format)?;
                }
            }
        }
        scene.rect(
            axis.left,
            self.top,
            axis.width,
            self.bottom - self.top,
            0.0,
            None,
            Some(grid),
        );
        Ok(())
    }

    fn axis_label(&self, scene: &mut Scene, top: bool, x: f32, label: &ImageLabel) {
        if top {
            label.center(scene, x, self.top - AXIS_HEIGHT + 4.0, None);
        }
        label.center(scene, x, self.bottom + 5.0, None);
    }

    fn dependency(
        &self,
        scene: &mut Scene,
        dependency: &Dependency,
        chart: &Chart,
        rows: &[TaskRow],
        color: Rgb,
    ) {
        let finish = &rows[dependency.finish];
        let start = &rows[dependency.start];
        let finish_tags = &chart.tasks[dependency.finish].tags;
        let start_tags = &chart.tasks[dependency.start].tags;
        let x0 = if finish_tags.milestone || finish_tags.vert {
            finish.position
        } else {
            finish.end_x
        };
        let x1 = if start_tags.milestone || start_tags.vert {
            start.position
        } else {
            start.start_x
        };
        let y0 = finish.center;
        let y1 = start.center;
        let direction = if y1 >= y0 { 1.0 } else { -1.0 };
        let (out, enter) = (y0 + direction * 15.0, y1 - direction * 15.0);
        let (first, last) = (
            dependency.finish.min(dependency.start),
            dependency.finish.max(dependency.start),
        );
        let center = (x0 + x1) / 2.0;
        let bridge = if first == last {
            // `until` can only be self-referential on the known start boundary.
            x0.max(x1) + 12.0
        } else {
            clear_column(
                center,
                rows[first + 1..last]
                    .iter()
                    .zip(&chart.tasks[first + 1..last])
                    .map(|(row, task)| row.shape(&task.tags)),
            )
        };
        let end = y1 - direction * (BAR / 2.0 + 1.0);
        let points = if first == last {
            [
                (x0, y0 + 7.0),
                (x0, out),
                (bridge, out),
                (bridge, enter),
                (x1, enter),
                (x1, end),
            ]
        } else {
            [
                (x0, y0 + direction * 6.0),
                (x0, out),
                (bridge, out),
                (bridge, enter),
                (x1, enter),
                (x1, end),
            ]
        };
        if dependency.until {
            dashed_image_path(scene, &points, color, 1.4);
        } else {
            let mut path = String::new();
            for (i, (x, y)) in points.into_iter().enumerate() {
                let command = if i == 0 { 'M' } else { 'L' };
                write!(path, "{command} {x} {y} ").unwrap();
            }
            scene.path(&path, None, Some(color), 1.4);
        }
        let tail = end - direction * 5.0;
        scene.path(
            &format!("M {x1} {end} l -3.5 {} h 7 Z", tail - end),
            Some(color),
            None,
            0.0,
        );
    }
}

fn clear_column(center: f32, obstacles: impl Iterator<Item = (f32, f32)>) -> f32 {
    let mut spans: Vec<_> = obstacles.collect();
    spans.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Option<(f32, f32)> = None;
    let nearest = |left: f32, right: f32| {
        if center - left < right - center {
            left
        } else {
            right
        }
    };
    for (left, right) in spans {
        let (left, right) = (left - 5.0, right + 5.0);
        if let Some((low, high)) = merged {
            if left <= high {
                merged = Some((low, high.max(right)));
                continue;
            }
            if (low..=high).contains(&center) {
                return nearest(low, high);
            }
        }
        merged = Some((left, right));
    }
    merged
        .filter(|&(low, high)| (low..=high).contains(&center))
        .map_or(center, |(low, high)| nearest(low, high))
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let mut chart = Chart::parse(source)?;
    let Schedule {
        times,
        dependencies,
    } = chart.schedule()?;
    let palette = Palette::new(&chart, options);
    let ink = scene.ink();
    let grid = palette.grid.fg.unwrap_or(fade(ink, 0.32));
    let excluded = palette.excluded.bg.unwrap_or(fade(ink, 0.055));
    let today = fade(palette.today.fg.unwrap_or(ink), chart.today_opacity);
    let vert = palette.vert.fg.unwrap_or(fade(ink, 0.7));
    let connector = fade(options.theme_line().unwrap_or(ink), 0.76);

    let title = ImageLabel::new(scene, &chart.title);
    let description = ImageLabel::wrapped(scene, &chart.description, 100);
    let notes: Vec<_> = chart
        .notes
        .iter()
        .enumerate()
        .map(|(i, (_, text))| ImageLabel::wrapped(scene, &format!("[{}] {text}", i + 1), 100))
        .collect();
    let captions: Vec<_> = captions(&chart)
        .into_iter()
        .map(|(text, clickable)| (ImageLabel::wrapped(scene, &text, 36), clickable))
        .collect();
    let sections: Vec<_> = chart
        .sections
        .iter()
        .map(|(_, text)| ImageLabel::wrapped(scene, text, 36))
        .collect();
    let gutter = captions
        .iter()
        .map(|(label, _)| label.width)
        .chain(sections.iter().map(|label| label.width))
        .fold(0.0, f32::max)
        + 32.0;
    let (first, last, fmt) = time_domain(&chart, &times);
    let first_label = ImageLabel::new(scene, &date_label(first, fmt)?);
    let last_label = ImageLabel::new(scene, &date_label(last, fmt)?);
    let axis = Axis {
        first,
        last,
        left: IMAGE_PAD + gutter.max(first_label.width / 2.0 + 12.0) + PLOT_PAD,
        width: 740.0_f32.max((first_label.width + last_label.width) / 2.0 + 36.0),
        first_label,
        last_label,
    };
    let top = IMAGE_PAD
        + title.height()
        + if title.height() > 0.0 { 12.0 } else { 0.0 }
        + if chart.top_axis { AXIS_HEIGHT } else { 0.0 };
    let mut y = top;
    let mut section_rows = Vec::new();
    let mut sections = chart.sections.iter().zip(sections).peekable();
    let mut rows = Vec::new();
    for (i, (task, (label, clickable))) in chart.tasks.iter().zip(captions).enumerate() {
        while sections.peek().is_some_and(|((at, _), _)| *at == i) {
            let (_, label) = sections.next().unwrap();
            let height = label.height() + 12.0;
            section_rows.push(SectionRow { label, y });
            y += height;
        }
        let [start, end] = times[i];
        let height = (label.height() + 12.0).max(TASK_HEIGHT);
        rows.push(TaskRow {
            label,
            style: task_style(&palette, &task.tags, clickable, options, ink),
            center: y + height / 2.0,
            height,
            start_x: axis.x(start),
            end_x: axis.x(end),
            position: axis.x(start + (end - start) / 2),
        });
        y += height;
    }
    for (_, label) in sections {
        let height = label.height() + 12.0;
        section_rows.push(SectionRow { label, y });
        y += height;
    }
    let plot = Plot {
        axis,
        top,
        bottom: y + if rows.is_empty() { TASK_HEIGHT } else { 0.0 },
    };
    let has_vert = chart.tasks.iter().any(|task| task.tags.vert);
    let has_milestone = chart.tasks.iter().any(|task| task.tags.milestone);
    let has_today = chart.today_marker.is_some() && (first..=last).contains(&chart.now);
    let show_exclusions =
        last > first && i128::from(last - first) <= plot.axis.width as i128 * i128::from(DAY);
    let keys: Vec<_> = [
        (dependencies.iter().any(|d| !d.until), Key::After),
        (dependencies.iter().any(|d| d.until), Key::Until),
        (has_milestone, Key::Milestone),
        (has_vert, Key::Vertical),
        (has_today, Key::Today),
        (
            chart.calendar.has_exclusions(),
            Key::Excluded(show_exclusions),
        ),
    ]
    .into_iter()
    .filter_map(|(present, key)| present.then_some(key))
    .collect();
    let key_width = keys
        .iter()
        .map(|key| scene.measure(key.text()) + 54.0)
        .sum::<f32>();
    let bottom = plot.bottom
        + AXIS_HEIGHT
        + if keys.is_empty() {
            0.0
        } else {
            IMAGE_LINE + 12.0
        };
    let description_height =
        description.height() + notes.iter().map(ImageLabel::height).sum::<f32>();
    let width = (plot.axis.right() + PLOT_PAD.max(plot.axis.last_label.width / 2.0))
        .max(IMAGE_PAD + title.width.max(description.width).max(key_width))
        .max(IMAGE_PAD + notes.iter().map(|label| label.width).fold(0.0, f32::max))
        + IMAGE_PAD;
    scene.resize(width, bottom + description_height + IMAGE_PAD)?;

    title.draw(scene, IMAGE_PAD, IMAGE_PAD, palette.title.fg);
    for (i, row) in rows.iter().enumerate() {
        if i % 2 == 1 {
            scene.rect(
                IMAGE_PAD,
                row.center - row.height / 2.0,
                plot.axis.right() - IMAGE_PAD,
                row.height,
                0.0,
                Some(fade(ink, 0.035)),
                None,
            );
        }
    }
    for section in &section_rows {
        scene.rect(
            IMAGE_PAD,
            section.y,
            plot.axis.right() - IMAGE_PAD,
            section.label.height() + 12.0,
            0.0,
            Some(fade(ink, 0.085)),
            None,
        );
        section
            .label
            .draw(scene, IMAGE_PAD + 4.0, section.y + 6.0, palette.title.fg);
    }
    plot.grid(
        &chart,
        fmt,
        scene,
        grid,
        (show_exclusions && chart.calendar.has_exclusions()).then_some(excluded),
    )?;
    for dependency in &dependencies {
        plot.dependency(scene, dependency, &chart, &rows, connector);
    }
    for (task, row) in chart.tasks.iter().zip(&rows) {
        row.draw(scene, &task.tags, vert);
    }
    for (task, row) in chart.tasks.iter().zip(&rows) {
        if task.tags.vert {
            line(
                scene,
                (row.position, top),
                (row.position, plot.bottom),
                Some(vert),
                1.5,
            );
        }
    }
    if has_today {
        let x = plot.axis.x(chart.now);
        line(
            scene,
            (x, top),
            (x, plot.bottom),
            Some(today),
            chart.today_width,
        );
    }
    if rows.is_empty() {
        scene.text(IMAGE_PAD + 10.0, y + 8.0, "No tasks", None);
    }
    let mut x = IMAGE_PAD;
    let y = plot.bottom + AXIS_HEIGHT;
    for key in keys {
        let label = key.text();
        match key {
            Key::After => {
                scene.path(
                    &format!("M {x} {} h 22 m -5 -4 l 5 4 -5 4", y + 11.0),
                    None,
                    Some(connector),
                    1.4,
                );
            }
            Key::Until => {
                scene.path(
                    &format!(
                        "M {x} {} h 4 m 4 0 h 4 m 4 0 h 6 m -5 -4 l 5 4 -5 4",
                        y + 11.0
                    ),
                    None,
                    Some(connector),
                    1.4,
                );
            }
            Key::Today => line(
                scene,
                (x + 10.0, y + 3.0),
                (x + 10.0, y + 18.0),
                Some(today),
                chart.today_width,
            ),
            Key::Milestone => diamond(scene, x + 10.0, y + 11.0, Some(fade(ink, 0.35)), Some(ink)),
            Key::Vertical => line(
                scene,
                (x + 10.0, y + 3.0),
                (x + 10.0, y + 18.0),
                Some(vert),
                1.5,
            ),
            Key::Excluded(draw) => {
                if draw {
                    scene.rect(x, y + 3.0, 22.0, 16.0, 1.0, Some(excluded), Some(grid));
                }
            }
        }
        scene.text(x + 30.0, y, label, None);
        x += scene.measure(label) + 54.0;
    }
    description.draw(scene, IMAGE_PAD, bottom, None);
    let mut y = bottom + description.height();
    for note in notes {
        note.draw(scene, IMAGE_PAD, y, None);
        y += note.height();
    }
    Ok(())
}
