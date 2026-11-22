use super::shared::board::{
    self, Captions, Pen, Rows, after_word, padded, paint_wrapped, theme_color, width,
};
use crate::{
    Options,
    color::{Layer, Paint, Rgb},
    text::wrap,
};
use rustc_hash::FxHashMap;

mod image;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        render(
            board::body(source, &["journey"])?,
            options,
            Pen::new(options),
        )
    }
}

struct Step {
    task: String,
    mood: f64,
    actors: String,
}

impl Step {
    fn actors(&self) -> impl Iterator<Item = &str> {
        self.actors.split(", ").filter(|name| !name.is_empty())
    }

    fn task_text(&self) -> Cow<'_, str> {
        if self.mood.fract() == 0.0 {
            Cow::Borrowed(&self.task)
        } else {
            format!("{} ({}/5)", self.task, self.mood).into()
        }
    }
}

#[derive(Default)]
struct Stage {
    name: String,
    steps: Vec<Step>,
}

impl Stage {
    fn heading(&self) -> &str {
        if self.name.is_empty() {
            "Step"
        } else {
            &self.name
        }
    }
}

fn stage_fill(options: &Options, index: usize) -> Option<Rgb> {
    theme_color(options, &format!("fillType{}", index % 8))
}

fn actor_colors<'a>(options: &Options, stages: &'a [Stage]) -> FxHashMap<&'a str, Rgb> {
    let colors: [_; 6] = std::array::from_fn(|i| theme_color(options, &format!("actor{i}")));
    if colors.iter().all(Option::is_none) {
        return FxHashMap::default();
    }
    let mut names: Vec<_> = stages
        .iter()
        .flat_map(|s| &s.steps)
        .flat_map(Step::actors)
        .collect();
    names.sort_unstable_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    names.dedup();
    names
        .into_iter()
        .zip(colors)
        .filter_map(|(name, color)| Some((name, color?)))
        .collect()
}

fn parse(body: &str) -> Result<(Captions, Vec<Stage>), String> {
    let mut captions = Captions::default();
    let mut stages: Vec<Stage> = Vec::new();
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let mut text = line.split("%%").next().unwrap().trim();
        if let Some((before, _)) = text.split_once(" #") {
            text = before.trim_end();
        }
        if text.is_empty() || text.starts_with('#') || captions.consume(text, &mut lines)? {
            continue;
        }
        if let Some(name) = after_word(text, "section").filter(|name| !name.contains(':')) {
            stages.push(Stage {
                name: name.into(),
                steps: vec![],
            });
            continue;
        }
        let (task, fields) = text
            .split_once(':')
            .ok_or_else(|| format!("expected 'task: score[: actors]': {text}"))?;
        let (score, actors) = fields.split_once(':').unwrap_or((fields, ""));
        let mood = score
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| (1.0..=5.0).contains(v))
            .ok_or_else(|| format!("journey score must be a number from 1 to 5: {text}"))?;
        if task.trim().is_empty() || actors.contains(':') || text.contains(';') {
            return Err(format!("invalid journey step: {text}"));
        }
        let actors = if actors.trim().is_empty() {
            String::new()
        } else {
            let names: Vec<_> = actors.split(',').map(str::trim).collect();
            if names.contains(&"") {
                return Err(format!("empty journey actor: {text}"));
            }
            names.join(", ")
        };
        if stages.is_empty() {
            stages.push(Stage::default());
        }
        stages.last_mut().unwrap().steps.push(Step {
            task: task.trim().into(),
            mood,
            actors,
        });
    }
    if stages.is_empty() {
        stages.push(Stage::default());
    }
    Ok((captions, stages))
}

fn position(score: f64, spacing: usize) -> usize {
    ((score - 1.0) * spacing as f64).round() as usize
}

fn score_line(from: f64, to: f64, spacing: usize, pen: Pen) -> String {
    let mut cells = vec![' '; 1 + 4 * spacing];
    let a = position(from, spacing);
    let b = position(to, spacing);
    if a == b {
        cells[a] = pen.ch('│');
    } else {
        let (lo, hi) = (a.min(b), a.max(b));
        cells[lo + 1..hi].fill(pen.ch('─'));
        cells[a] = pen.ch(if a < b { '╰' } else { '╯' });
        cells[b] = pen.ch(if a < b { '╮' } else { '╭' });
    }
    cells.into_iter().collect()
}

fn render(body: &str, options: &Options, pen: Pen) -> Result<String, String> {
    let (captions, stages) = parse(body)?;
    let limit = options.width;
    let compact = limit < 36;
    let has_actors = !compact
        && stages
            .iter()
            .flat_map(|s| &s.steps)
            .any(|s| !s.actors.is_empty());
    let task_natural = stages
        .iter()
        .flat_map(|s| {
            std::iter::once(width(&s.name)).chain(s.steps.iter().map(|t| width(&t.task_text())))
        })
        .max()
        .unwrap_or(0)
        .clamp(6, 68);
    let actor_natural = stages
        .iter()
        .flat_map(|s| &s.steps)
        .map(|s| width(&s.actors))
        .max()
        .unwrap_or(0)
        .clamp(6, 42);
    let spacing = if compact { 1 } else { 2 };
    let scale_w = 1 + 4 * spacing;
    let available = limit.saturating_sub(scale_w + if has_actors { 10 } else { 7 });
    let actor_w = if has_actors {
        actor_natural.min(
            (available / 3)
                .max(available.saturating_sub(task_natural))
                .max(6),
        )
    } else {
        0
    };
    let task_w = task_natural.min(available.saturating_sub(actor_w).max(8));
    let mut widths = vec![task_w, scale_w];
    if has_actors {
        widths.push(actor_w);
    }
    let [tl, tr, bl, br] = pen.corners;
    let rule = |left, between, right| {
        format!(
            "{left}{}{right}",
            widths
                .iter()
                .map(|w| pen.stroke('─', w + 2))
                .collect::<Vec<_>>()
                .join(&pen.ch(between).to_string())
        )
    };
    let row = |task: &str, mood: &str, who: &str| {
        let bar = pen.ch('│');
        let mut s = format!(
            "{bar} {} {bar} {} {bar}",
            padded(task, task_w),
            padded(mood, scale_w)
        );
        if has_actors {
            s.push_str(&format!(" {} {bar}", padded(who, actor_w)));
        }
        s
    };
    let mut output = Rows::new();
    let mut colors = Layer::default();
    let actor_colors = actor_colors(options, &stages);
    let face = Paint {
        fg: theme_color(options, "faceColor"),
        bg: None,
    };
    for (stage_index, stage) in stages.iter().enumerate() {
        let fill = Paint {
            fg: None,
            bg: stage_fill(options, stage_index),
        };
        if !output.is_empty() {
            output.push(String::new());
        }
        output.push(rule(tl, '┬', tr));
        for (i, name) in wrap(stage.heading(), task_w).into_iter().enumerate() {
            colors.paint(1, output.len(), task_w + 2, fill);
            output.push(row(
                &name,
                if i == 0 {
                    if compact { "Score" } else { "Score 1-5" }
                } else {
                    ""
                },
                if i == 0 { "Actors" } else { "" },
            ));
        }
        if !stage.steps.is_empty() {
            output.push(rule(pen.ch('├'), '┼', pen.ch('┤')));
        }
        for (i, step) in stage.steps.iter().enumerate() {
            let mut task = wrap(&step.task_text(), task_w);
            let task_lines = task.len();
            if compact && !step.actors.is_empty() {
                task.extend(wrap(&format!("@{}", step.actors), task_w));
            }
            let actors = if has_actors {
                wrap(&step.actors, actor_w)
            } else {
                vec![]
            };
            if !actor_colors.is_empty() && !step.actors.is_empty() {
                let names = step.actors().enumerate().flat_map(|(i, name)| {
                    let sep = match (i, compact) {
                        (0, true) => "@",
                        (0, false) => "",
                        _ => ", ",
                    };
                    [
                        (sep, Paint::default()),
                        (
                            name,
                            Paint {
                                fg: actor_colors.get(name).copied(),
                                bg: None,
                            },
                        ),
                    ]
                });
                let (x, y, lines) = if compact {
                    (2, output.len() + task_lines, &task[task_lines..])
                } else {
                    (task_w + scale_w + 8, output.len(), actors.as_slice())
                };
                paint_wrapped(&mut colors, x, y, lines, names);
            }
            colors.paint(
                task_w + 5 + position(step.mood, spacing),
                output.len(),
                1,
                face,
            );
            let mut scale = vec![' '; scale_w];
            for mark in scale.iter_mut().step_by(spacing) {
                *mark = pen.ch('·');
            }
            scale[position(step.mood, spacing)] = if step.mood.fract() == 0.0 {
                char::from_digit(step.mood as u32, 10).unwrap()
            } else {
                pen.ch('●')
            };
            let marker: String = scale.into_iter().collect();
            let next = stage.steps.get(i + 1);
            let hold = if next.is_some() {
                score_line(step.mood, step.mood, spacing, pen)
            } else {
                String::new()
            };
            for k in 0..task.len().max(actors.len()) {
                if k < task_lines {
                    colors.paint(1, output.len(), task_w + 2, fill);
                }
                output.push(row(
                    task.get(k).map_or("", String::as_str),
                    if k == 0 { &marker } else { &hold },
                    actors.get(k).map_or("", String::as_str),
                ));
            }
            if let Some(next) = next {
                output.push(row("", &score_line(step.mood, next.mood, spacing, pen), ""));
            }
        }
        output.push(rule(bl, '┴', br));
    }
    Ok(captions.prepend(colors.apply(&output.join("\n")), limit))
}
use std::borrow::Cow;
