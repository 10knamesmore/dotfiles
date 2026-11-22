use super::{Stage, actor_colors, board, parse, stage_fill, theme_color};
use crate::{Options, color::Rgb, graphics::Scene};
use board::image::{Caption, LINE, Label, PAD, text_on_fill, tint};

const GAP: f32 = 14.0;
const CARD_GAP: f32 = 6.0;
const HEADER: f32 = 36.0;
const INSET: f32 = 10.0;
const SCORE_SPACING: f32 = 30.0;

struct Task {
    label: Label,
    score: Label,
    mood: f32,
    actors: Vec<(Label, Option<Rgb>)>,
    width: f32,
}

struct Section {
    heading: Label,
    tasks: Vec<Task>,
    width: f32,
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (captions, stages) = parse(board::body(source, &["journey"])?)?;
    let caption = Caption::new(scene, &captions, options);
    let actors = actor_colors(options, &stages);
    let sections: Vec<_> = stages
        .iter()
        .map(|stage: &Stage| {
            let heading = Label::new(scene, stage.heading());
            let tasks: Vec<_> = stage
                .steps
                .iter()
                .map(|step| {
                    let label = Label::new(scene, &step.task);
                    let score = Label::new(scene, &step.mood.to_string());
                    let actors: Vec<_> = step
                        .actors()
                        .map(|name| (Label::new(scene, name), actors.get(name).copied()))
                        .collect();
                    let width = actors
                        .iter()
                        .map(|(name, _)| name.width + 12.0)
                        .fold(label.width.max(score.width), f32::max)
                        + 2.0 * INSET;
                    Task {
                        label,
                        score,
                        mood: step.mood as f32,
                        actors,
                        width: width.max(86.0),
                    }
                })
                .collect();
            let width = (tasks.iter().map(|task| task.width + CARD_GAP).sum::<f32>() - CARD_GAP)
                .max(heading.width + 2.0 * INSET);
            Section {
                heading,
                tasks,
                width,
            }
        })
        .collect();
    let max_actors = sections
        .iter()
        .flat_map(|section| section.tasks.iter().map(|task| task.actors.len()))
        .max();
    let heading_y = PAD + caption.height;
    let tasks_y = heading_y + HEADER + 6.0;
    let tasks_height = max_actors.map_or(0.0, |count| {
        LINE + 2.0 * INSET
            + if count == 0 {
                0.0
            } else {
                5.0 + count as f32 * LINE
            }
    });
    let x_start = PAD + max_actors.map_or(0.0, |_| scene.measure("Score") + GAP);
    let x_end = x_start + sections.iter().map(|s| s.width + GAP).sum::<f32>() - GAP;
    let score_top = tasks_y + tasks_height + 34.0;
    let score_y = |mood: f32| score_top + (5.0 - mood) * SCORE_SPACING;
    let height = if max_actors.is_some() {
        score_y(1.0) + 16.0
    } else {
        heading_y + HEADER
    };
    scene.resize((x_end + PAD).max(caption.width + 2.0 * PAD), height + PAD)?;
    caption.draw(scene);

    let ink = scene.ink();
    let guide = tint(ink, 50);
    let line = options.theme_line().unwrap_or_else(|| tint(ink, 175));
    let face = theme_color(options, "faceColor");
    let face_text = text_on_fill(scene, options, face);
    if max_actors.is_some() {
        scene.text(PAD, score_top - 32.0, "Score", None);
        for score in 1..=5 {
            let y = score_y(score as f32);
            let label = score.to_string();
            scene.text(
                x_start - 12.0 - scene.measure(&label),
                y - LINE / 2.0,
                &label,
                None,
            );
            scene.path(
                &format!("M {x_start} {y} H {x_end}"),
                None,
                Some(guide),
                1.0,
            );
        }
    }

    let mut x = x_start;
    for (i, section) in sections.into_iter().enumerate() {
        let fill = stage_fill(options, i);
        scene.rect(
            x,
            heading_y,
            section.width,
            HEADER,
            7.0,
            fill.or(Some(tint(ink, 29))),
            None,
        );
        section.heading.center(
            scene,
            x + section.width / 2.0,
            heading_y + (HEADER - LINE) / 2.0,
            text_on_fill(scene, options, fill),
        );
        let spare_per_task = (section.width
            - section
                .tasks
                .iter()
                .map(|t| t.width + CARD_GAP)
                .sum::<f32>()
            + CARD_GAP)
            / section.tasks.len().max(1) as f32;
        let mut task_x = x;
        let mut scores = Vec::new();
        for task in &section.tasks {
            let width = task.width + spare_per_task;
            let mid = task_x + width / 2.0;
            let y = score_y(task.mood);
            scene.rect(
                task_x,
                tasks_y,
                width,
                tasks_height,
                7.0,
                Some(tint(ink, 10)),
                Some(guide),
            );
            task.label.center(scene, mid, tasks_y + INSET, None);
            for (row, (actor, paint)) in task.actors.iter().enumerate() {
                let y = tasks_y + INSET + LINE + 5.0 + row as f32 * LINE;
                scene.rect(
                    task_x + INSET,
                    y + 7.0,
                    6.0,
                    6.0,
                    3.0,
                    paint.or(Some(line)),
                    None,
                );
                actor.draw(scene, task_x + INSET + 12.0, y, *paint);
            }
            let start_y = tasks_y + tasks_height;
            scene.path(&format!("M {mid} {start_y} V {y}"), None, Some(guide), 1.0);
            scores.push((mid, y, &task.score));
            task_x += width + CARD_GAP;
        }
        for [(x1, y1, _), (x2, y2, _)] in scores.array_windows() {
            scene.path(&format!("M {x1} {y1} L {x2} {y2}"), None, Some(line), 2.0);
        }
        for (mid, y, score) in scores {
            let width = (score.width + 10.0).max(24.0);
            scene.rect(
                mid - width / 2.0,
                y - 12.0,
                width,
                24.0,
                12.0,
                face.or(Some(scene.paper())),
                face.or(Some(line)),
            );
            score.center(scene, mid, y - LINE / 2.0, face_text);
        }
        x += section.width + GAP;
    }
    Ok(())
}
