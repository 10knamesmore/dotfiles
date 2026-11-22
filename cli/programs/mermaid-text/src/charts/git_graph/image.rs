use super::{Direction, History, Mark, branch_order, history, lanes::PickTracks, orientation};
use crate::{
    Options,
    charts::shared::history::{IMAGE_PAD, ImageLabel, dashed_image_path},
    color::Rgb,
    graphics::Scene,
};

const TAG_GAP: f32 = 5.0;
const CHIP: f32 = 6.0;
const PICK_TRACK: f32 = 8.0;
const COLORS: [Rgb; 8] = [
    Rgb::from_rgba(89, 162, 230, 255),
    Rgb::from_rgba(102, 197, 158, 255),
    Rgb::from_rgba(240, 176, 94, 255),
    Rgb::from_rgba(197, 138, 223, 255),
    Rgb::from_rgba(240, 133, 141, 255),
    Rgb::from_rgba(101, 191, 200, 255),
    Rgb::from_rgba(218, 199, 103, 255),
    Rgb::from_rgba(165, 172, 237, 255),
];

#[derive(Clone, Copy)]
struct ChipColors {
    text: Option<Rgb>,
    fill: Option<Rgb>,
    stroke: Option<Rgb>,
}

impl ChipColors {
    fn draw(self, scene: &mut Scene, label: &ImageLabel, x: f32, y: f32) {
        scene.rect(
            x,
            y,
            label.width + 2.0 * CHIP,
            label.height() + CHIP,
            5.0,
            self.fill,
            self.stroke,
        );
        label.draw(scene, x + CHIP, y + CHIP / 2.0, self.text);
    }
}

struct BranchLabel {
    name: ImageLabel,
    colors: ChipColors,
    line: Rgb,
    highlight: Rgb,
}

struct CommitLabel {
    text: ImageLabel,
    tags: Vec<ImageLabel>,
}

impl CommitLabel {
    fn width(&self) -> f32 {
        self.tags
            .iter()
            .map(|tag| tag.width)
            .fold(self.text.width, f32::max)
            + 2.0 * CHIP
    }

    fn height(&self) -> f32 {
        self.text_height()
            + self
                .tags
                .iter()
                .map(|tag| tag.height() + CHIP + TAG_GAP)
                .sum::<f32>()
    }

    fn text_height(&self) -> f32 {
        self.text.height() + CHIP
    }
}

struct Layout {
    graph_top: f32,
    horizontal: bool,
    sign: f32,
    time: Vec<f32>,
    lanes: Vec<f32>,
    label_rail: f32,
    width: f32,
    bottom: f32,
}

impl Layout {
    fn plan(
        history: &History,
        direction: Direction,
        branches: &[BranchLabel],
        commits: &[Option<CommitLabel>],
        tracks: &PickTracks,
        graph_top: f32,
    ) -> Self {
        let branch_width = branches
            .iter()
            .map(|branch| branch.name.width + 2.0 * CHIP)
            .fold(0.0, f32::max);
        let branch_height = branches
            .iter()
            .map(|branch| branch.name.height() + CHIP)
            .fold(0.0, f32::max);
        let first_x = IMAGE_PAD + branch_width + 46.0;
        let horizontal_width = first_x
            + commits
                .iter()
                .map(|label| label.as_ref().map_or(48.0, |label| label.width() + 54.0))
                .sum::<f32>();
        let horizontal = direction == Direction::Right
            || (direction == Direction::Auto && horizontal_width < 1240.0);
        let sign = if direction == Direction::Up {
            -1.0
        } else {
            1.0
        };
        let order = branch_order(&history.branches);
        let mut lanes = vec![0.0; branches.len()];
        let mut time = Vec::with_capacity(commits.len());
        if horizontal {
            let mut depths: Vec<_> = branches
                .iter()
                .map(|branch| branch.name.height() + CHIP)
                .collect();
            for (event, label) in history.events.iter().zip(commits) {
                if let Some(label) = label {
                    depths[event.lane] = depths[event.lane].max(12.0 + label.height());
                }
            }
            let mut y = graph_top;
            for branch in order {
                let chip_half = (branches[branch].name.height() + CHIP) / 2.0;
                y += chip_half.max(12.0 + tracks.count[branch] as f32 * PICK_TRACK);
                lanes[branch] = y;
                y += depths[branch] + 17.0;
            }
            let mut x = first_x;
            for label in commits {
                time.push(x);
                x += label.as_ref().map_or(48.0, |label| label.width() + 54.0);
            }
            Self {
                graph_top,
                horizontal,
                sign,
                time,
                lanes,
                label_rail: 0.0,
                width: x.max(first_x + 24.0),
                bottom: y - 12.0,
            }
        } else {
            let mut x = IMAGE_PAD;
            for branch in order {
                let half = (branches[branch].name.width + 2.0 * CHIP).max(54.0) / 2.0;
                x += half.max(12.0 + tracks.count[branch] as f32 * PICK_TRACK);
                lanes[branch] = x;
                x += half + 24.0;
            }
            let mut labels_width = 0.0_f32;
            let mut y = graph_top + branch_height + 28.0;
            for row in 0..commits.len() {
                let index = if sign < 0.0 {
                    commits.len() - 1 - row
                } else {
                    row
                };
                let label = &commits[index];
                let height = label.as_ref().map_or(0.0, CommitLabel::height);
                let step = (height + 20.0).max(45.0);
                time.push(
                    y + label.as_ref().map_or(step / 2.0, |label| {
                        (step - height + label.text_height()) / 2.0
                    }),
                );
                y += step;
                if let Some(label) = label {
                    labels_width = labels_width.max(label.width());
                }
            }
            if sign < 0.0 {
                time.reverse();
            }
            Self {
                graph_top,
                horizontal,
                sign,
                time,
                lanes,
                label_rail: x,
                width: x + labels_width,
                bottom: y + 8.0,
            }
        }
    }

    fn point(&self, time: f32, lane: f32) -> (f32, f32) {
        if self.horizontal {
            (time, lane)
        } else {
            (lane, time)
        }
    }

    fn event(&self, time: usize, lane: usize) -> (f32, f32) {
        self.point(self.time[time], self.lanes[lane])
    }

    fn connection(&self, scene: &mut Scene, from: (f32, f32), to: (f32, f32), color: Rgb) {
        let (t0, l0) = from;
        let (t1, l1) = to;
        let (x0, y0) = self.point(t0, l0);
        let (x1, y1) = self.point(t1, l1);
        let d = if l0 == l1 || t0 == t1 {
            format!("M {x0} {y0} L {x1} {y1}")
        } else {
            let bend = (t1 - t0).abs().min(32.0);
            let (mx, my) = self.point(t1 - self.sign * bend, l0);
            let (ax, ay) = self.point(t1 - self.sign * bend / 2.0, l0);
            let (bx, by) = self.point(t1 - self.sign * bend / 2.0, l1);
            format!("M {x0} {y0} L {mx} {my} C {ax} {ay} {bx} {by} {x1} {y1}")
        };
        scene.path(&d, None, Some(color), 2.5);
    }

    fn cherry_pick(
        &self,
        scene: &mut Scene,
        source: (usize, usize),
        target: (usize, usize),
        track: usize,
        color: Rgb,
    ) {
        let (from, a) = (self.time[source.0], self.lanes[source.1]);
        let (to, b) = (self.time[target.0], self.lanes[target.1]);
        let lane = a - (10.0 + track as f32 * PICK_TRACK);
        let points = [
            self.point(from, a),
            self.point(from + 12.0 * self.sign, lane),
            self.point(to - 14.0 * self.sign, lane),
            self.point(to, b),
        ];
        dashed_image_path(scene, &points, color, 1.7);
    }
}

fn opacity(color: Rgb, opacity: u8) -> Rgb {
    let [r, g, b, a] = color.rgba();
    Rgb::from_rgba(r, g, b, (u16::from(a) * u16::from(opacity) / 255) as u8)
}

fn draw_mark(scene: &mut Scene, (x, y): (f32, f32), mark: Mark, branch: &BranchLabel) {
    let color = Some(branch.line);
    let paper = Some(scene.paper());
    match mark {
        Mark::Normal => scene.rect(x - 6.0, y - 6.0, 12.0, 12.0, 6.0, color, paper),
        Mark::Reverse => {
            scene.rect(x - 7.0, y - 7.0, 14.0, 14.0, 7.0, paper, color);
            scene.path(
                &format!("M {} {} l 8 8 m -8 0 l 8 -8", x - 4.0, y - 4.0),
                None,
                color,
                1.4,
            );
        }
        Mark::Highlight => scene.rect(
            x - 7.0,
            y - 7.0,
            14.0,
            14.0,
            2.0,
            Some(branch.highlight),
            color,
        ),
        Mark::Merge => {
            scene.rect(x - 8.0, y - 8.0, 16.0, 16.0, 8.0, paper, color);
            scene.rect(x - 4.0, y - 4.0, 8.0, 8.0, 4.0, color, None);
        }
        Mark::Pick => scene.path(
            &format!("M {x} {} l 8 8 -8 8 -8 -8 Z", y - 8.0),
            paper,
            color,
            2.0,
        ),
    }
}

struct Graph<'a> {
    history: &'a History,
    layout: Layout,
    branches: Vec<BranchLabel>,
    commits: Vec<Option<CommitLabel>>,
    tracks: PickTracks,
}

impl Graph<'_> {
    fn draw(&self, scene: &mut Scene, commit_chip: ChipColors, tag_chip: ChipColors) {
        let layout = &self.layout;
        let history = self.history;
        for (lane, (branch, label)) in history.branches.iter().zip(&self.branches).enumerate() {
            let position = layout.lanes[lane];
            let (x, y) = if layout.horizontal {
                (IMAGE_PAD, position - (label.name.height() + CHIP) / 2.0)
            } else {
                (position - label.name.width / 2.0 - CHIP, layout.graph_top)
            };
            label.colors.draw(scene, &label.name, x, y);
            if layout.time.is_empty() {
                if layout.horizontal {
                    let t = x + label.name.width + 2.0 * CHIP + 12.0;
                    layout.connection(scene, (t, position), (t + 26.0, position), label.line);
                } else {
                    let t = y + label.name.height() + CHIP + 6.0;
                    layout.connection(scene, (t, position), (t + 24.0, position), label.line);
                }
                continue;
            }
            let first = layout.time[branch.first];
            let last = branch
                .head
                .map(|head| &history.commits[head])
                .filter(|head| head.lane == lane)
                .map_or(first, |head| layout.time[head.time]);
            let guide_start = first - if lane == 0 { layout.sign * 18.0 } else { 0.0 };
            layout.connection(
                scene,
                (guide_start, position),
                (last + 17.0 * layout.sign, position),
                opacity(label.line, 115),
            );
        }
        for (time, event) in history
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.mark.is_none())
        {
            let from = event.from.unwrap();
            let (t, lane) = event.fork_from.map_or_else(
                || (layout.time[history.branches[from].first], from),
                |parent| {
                    let parent = &history.commits[parent];
                    (layout.time[parent.time], parent.lane)
                },
            );
            layout.connection(
                scene,
                (t, layout.lanes[lane]),
                (layout.time[time], layout.lanes[event.lane]),
                self.branches[event.lane].line,
            );
        }
        for commit in &history.commits {
            let lane = commit.lane;
            let event = &history.events[commit.time];
            let line = self.branches[lane].line;
            for (index, parent) in commit.parents.iter().enumerate() {
                let Some(parent) = *parent else { continue };
                let origin = &history.commits[parent];
                if index == 1 && event.picked.is_some() {
                    layout.cherry_pick(
                        scene,
                        (origin.time, origin.lane),
                        (commit.time, lane),
                        self.tracks.track[commit.time],
                        line,
                    );
                    continue;
                }
                let fork = history.branches[lane].first;
                let (from, from_lane) =
                    if index == 0 && history.events[fork].fork_from == Some(parent) {
                        (fork, lane)
                    } else {
                        (origin.time, origin.lane)
                    };
                let ink = if index == 1 {
                    self.branches[origin.lane].line
                } else {
                    line
                };
                layout.connection(
                    scene,
                    (layout.time[from], layout.lanes[from_lane]),
                    (layout.time[commit.time], layout.lanes[lane]),
                    ink,
                );
            }
        }
        for (time, (event, label)) in history.events.iter().zip(&self.commits).enumerate() {
            let (x, y) = layout.event(time, event.lane);
            let branch = &self.branches[event.lane];
            let (Some(mark), Some(label)) = (event.mark, label) else {
                scene.rect(
                    x - 3.0,
                    y - 3.0,
                    6.0,
                    6.0,
                    3.0,
                    Some(scene.paper()),
                    Some(branch.line),
                );
                continue;
            };
            draw_mark(scene, (x, y), mark, branch);
            let (x, mut y) = if layout.horizontal {
                (x + 10.0, y + 9.0)
            } else {
                (layout.label_rail, y - label.text_height() / 2.0)
            };
            commit_chip.draw(scene, &label.text, x, y);
            y += label.text.height() + CHIP + TAG_GAP;
            for tag in &label.tags {
                tag_chip.draw(scene, tag, x, y);
                y += tag.height() + CHIP + TAG_GAP;
            }
        }
    }
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (suffix, lines) = history::header(source, &["gitGraph"])?;
    let (direction, rest) = orientation(suffix)?;
    let document = history::parse(rest, lines)?;
    let history = History::parse(&document.statements, options)?;
    let (title, description) = document.image_metadata(scene);
    let theme = |key: &str| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let order = branch_order(&history.branches);
    let mut slots = vec![0; order.len()];
    for (slot, branch) in order.into_iter().enumerate() {
        slots[branch] = slot % COLORS.len();
    }
    let branches: Vec<_> = history
        .branches
        .iter()
        .zip(slots)
        .map(|(branch, slot)| {
            let fill = theme(&format!("git{slot}"));
            let line = fill.or(options.theme_line()).unwrap_or_else(|| {
                if options.color {
                    COLORS[slot]
                } else {
                    scene.ink()
                }
            });
            BranchLabel {
                name: ImageLabel::wrapped(scene, &branch.name, 24),
                colors: ChipColors {
                    text: theme(&format!("gitBranchLabel{slot}")),
                    fill: fill.or(Some(opacity(line, 35))),
                    stroke: Some(line),
                },
                line,
                highlight: theme(&format!("gitInv{slot}")).unwrap_or(line),
            }
        })
        .collect();
    let commits: Vec<_> = history
        .events
        .iter()
        .map(|event| {
            event.mark.map(|_| CommitLabel {
                text: ImageLabel::wrapped(scene, &event.label, 30),
                tags: event
                    .tags
                    .iter()
                    .map(|tag| ImageLabel::wrapped(scene, tag, 30))
                    .collect(),
            })
        })
        .collect();
    let top = IMAGE_PAD + title.height() + if title.height() > 0.0 { 12.0 } else { 0.0 };
    let tracks = PickTracks::plan(&history.events, history.branches.len());
    let layout = Layout::plan(&history, direction, &branches, &commits, &tracks, top);
    let bottom = layout.bottom;
    scene.resize(
        (layout.width + IMAGE_PAD).max(title.width.max(description.width) + 2.0 * IMAGE_PAD),
        bottom + IMAGE_PAD + description.height(),
    )?;
    title.draw(scene, IMAGE_PAD, IMAGE_PAD, theme("titleColor"));
    let graph = Graph {
        history: &history,
        layout,
        branches,
        commits,
        tracks,
    };
    graph.draw(
        scene,
        ChipColors {
            text: theme("commitLabelColor"),
            fill: theme("commitLabelBackground"),
            stroke: None,
        },
        ChipColors {
            text: theme("tagLabelColor"),
            fill: theme("tagLabelBackground").or(Some(opacity(scene.ink(), 26))),
            stroke: theme("tagLabelBorder").or(Some(opacity(scene.ink(), 115))),
        },
    );
    description.draw(scene, IMAGE_PAD, bottom, None);
    Ok(())
}
