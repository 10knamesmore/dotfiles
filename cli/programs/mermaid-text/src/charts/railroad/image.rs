use std::fmt::Write;

use crate::Options;
use crate::color::Rgb;
use crate::graphics::Scene;

use super::{Atom, Colors, Expr, Grammar};

const MARGIN: f32 = 18.0;
const GAP: f32 = 16.0;
const LABEL_HEIGHT: f32 = 22.0;
const ATOM_PAD: f32 = 14.0;
const BRANCH: f32 = 24.0;
const STROKE: f32 = 1.6;

type Point = (f32, f32);

pub(super) fn render(
    grammar: Grammar<'_>,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    let colors = Colors::new(options);
    let rules: Vec<_> = grammar
        .rules
        .into_iter()
        .map(|(name, expr)| (format!("{name}:"), Tile::layout(expr, scene)))
        .collect();
    let mut width = grammar.title.as_deref().map_or(0.0, |t| scene.measure(t));
    let mut height = MARGIN + if grammar.title.is_some() { 32.0 } else { 0.0 };
    for (name, tile) in &rules {
        width = width
            .max(scene.measure(name))
            .max(tile.width + 2.0 * BRANCH);
        height += LABEL_HEIGHT + tile.above + tile.below + MARGIN;
    }
    scene.resize(width + 2.0 * MARGIN, height)?;

    let mut top = MARGIN;
    if let Some(title) = grammar.title {
        scene.text(MARGIN, top, &title, colors.rule_name.fg);
        top += 32.0;
    }
    let mut painter = Painter {
        stroke: colors.line.fg.unwrap_or(scene.ink()),
        colors,
        scene,
    };
    for (name, tile) in rules {
        painter
            .scene
            .text(MARGIN, top, &name, painter.colors.rule_name.fg);
        let left = MARGIN + BRANCH;
        let right = left + tile.width;
        let y = top + LABEL_HEIGHT + tile.above;
        painter.track((0.0, 0.0), &[(MARGIN, y), (left, y)]);
        painter.track((0.0, 0.0), &[(right, y), (right + BRANCH, y)]);
        painter.station(MARGIN, y);
        painter.station(right + BRANCH, y);
        painter.arrow(right + BRANCH - 5.0, y, 1.0);
        let next = y + tile.below + MARGIN;
        tile.draw(&mut painter, (left, y));
        top = next;
    }
    Ok(())
}

// Every expression enters and exits at y=0; branches occupy space above or below it.
#[derive(Default)]
struct Tile {
    width: f32,
    above: f32,
    below: f32,
    marks: Vec<(Point, Mark)>,
}

enum Mark {
    Child(Tile),
    Track(Vec<Point>),
    Atom(String, Atom, f32),
    Caption(String),
    Arrow(f32),
    Frame(f32, f32),
}

impl Tile {
    fn place(&mut self, x: f32, y: f32, mark: Mark) {
        self.marks.push(((x, y), mark));
    }

    fn track(&mut self, points: impl Into<Vec<Point>>) {
        self.place(0.0, 0.0, Mark::Track(points.into()));
    }

    fn child(&mut self, x: f32, y: f32, child: Self) {
        self.above = self.above.max(child.above - y);
        self.below = self.below.max(y + child.below);
        self.place(x, y, Mark::Child(child));
    }

    fn layout(expr: Expr, scene: &Scene) -> Self {
        match expr {
            Expr::Atom(text, kind) => {
                let width = scene.measure(&text) + 2.0 * ATOM_PAD;
                let mut tile = Self {
                    width,
                    above: 17.0,
                    below: 17.0,
                    ..Self::default()
                };
                tile.place(0.0, 0.0, Mark::Atom(text, kind, width));
                tile
            }
            Expr::Sequence(items) => {
                let mut row = Self::default();
                for (i, item) in items.into_iter().enumerate() {
                    let child = Self::layout(item, scene);
                    let x = row.width;
                    if i > 0 {
                        row.track([(x, 0.0), (x + GAP, 0.0)]);
                        row.width += GAP;
                    }
                    let x = row.width;
                    row.width += child.width;
                    row.child(x, 0.0, child);
                }
                row
            }
            Expr::Choice(mut items, _) if items.len() == 1 => {
                Self::layout(items.pop().unwrap(), scene)
            }
            Expr::Choice(items, ordered) => {
                let branches = items
                    .into_iter()
                    .map(|item| Self::layout(item, scene))
                    .collect();
                Self::choice(branches, ordered, scene)
            }
            Expr::Repeat(item, min, max) => Self::layout(*item, scene).repeat(min, max, scene),
            Expr::Scope(item, caption, exclusion) => Self::layout(*item, scene).scope(
                caption,
                exclusion.map(|item| Self::layout(*item, scene)),
                scene,
            ),
        }
    }

    fn choice(branches: Vec<Self>, ordered: bool, scene: &Scene) -> Self {
        let prefix = if ordered {
            scene.measure(&format!("{}:", branches.len())) + 10.0
        } else {
            0.0
        };
        let inner = branches.iter().map(|tile| tile.width).fold(0.0, f32::max);
        let width = inner + 2.0 * BRANCH + prefix;
        let mut choice = Self {
            width,
            ..Self::default()
        };
        for (i, branch) in branches.into_iter().enumerate() {
            let y = if i == 0 {
                0.0
            } else {
                choice.below + GAP + branch.above.max(if ordered { LABEL_HEIGHT } else { 0.0 })
            };
            let x = BRANCH + prefix;
            if i == 0 {
                choice.track([(0.0, y), (x, y)]);
                choice.track([(x + branch.width, y), (width, y)]);
            } else {
                choice.track([(0.0, 0.0), (0.0, y), (x, y)]);
                choice.track([(x + branch.width, y), (width, y), (width, 0.0)]);
            }
            if ordered {
                choice.above = choice.above.max(LABEL_HEIGHT);
                choice.place(
                    BRANCH,
                    y - LABEL_HEIGHT,
                    Mark::Caption(format!("{}:", i + 1)),
                );
            }
            choice.child(x, y, branch);
        }
        choice
    }

    fn repeat(self, min: usize, max: Option<usize>, scene: &Scene) -> Self {
        if max == Some(0) {
            let caption = "(empty)";
            let width = scene.measure(caption) + 2.0 * GAP;
            let mut tile = Self {
                width,
                above: LABEL_HEIGHT + 3.0,
                ..Self::default()
            };
            tile.track([(0.0, 0.0), (width, 0.0)]);
            tile.place(GAP, -tile.above, Mark::Caption(caption.into()));
            return tile;
        }
        if min == 1 && max == Some(1) {
            return self;
        }
        let count = match (min, max) {
            (0 | 1, None | Some(1)) => String::new(),
            (n, Some(m)) if n == m => format!("{n} total"),
            (n, Some(m)) => format!("{n}..{m} total"),
            (n, None) => format!("{n}+ total"),
        };
        let count_width = scene.measure(&count);
        let width = self.width.max(count_width + GAP) + 2.0 * BRANCH;
        let mut repeat = Self {
            width,
            ..Self::default()
        };
        repeat.track([(0.0, 0.0), (BRANCH, 0.0)]);
        repeat.track([(BRANCH + self.width, 0.0), (width, 0.0)]);
        repeat.child(BRANCH, 0.0, self);
        if min == 0 {
            repeat.above += GAP;
            let y = -repeat.above;
            repeat.track([(0.0, 0.0), (0.0, y), (width, y), (width, 0.0)]);
            repeat.place(width / 2.0, y, Mark::Arrow(1.0));
        }
        if max != Some(1) {
            let y = repeat.below + GAP;
            repeat.track([(width, 0.0), (width, y), (0.0, y), (0.0, 0.0)]);
            repeat.place(width / 2.0, y, Mark::Arrow(-1.0));
            repeat.below = y + if count.is_empty() { 0.0 } else { LABEL_HEIGHT };
            repeat.place((width - count_width) / 2.0, y + 2.0, Mark::Caption(count));
        }
        repeat
    }

    fn scope(self, caption: &str, exclusion: Option<Self>, scene: &Scene) -> Self {
        let width = (self.width + 2.0 * GAP)
            .max(scene.measure(caption) + 2.0 * GAP)
            .max(exclusion.as_ref().map_or(0.0, |tile| {
                (tile.width + 2.0 * GAP).max(scene.measure("except") + 2.0 * GAP)
            }));
        let above = self.above + GAP + LABEL_HEIGHT;
        let mut scope = Self {
            width,
            above,
            below: self.below + GAP,
            ..Self::default()
        };
        let y = scope.below;
        if let Some(tile) = exclusion {
            scope.below += LABEL_HEIGHT + tile.above + tile.below + GAP;
            scope.place(GAP, y - 2.0, Mark::Caption("except".into()));
            let baseline = y + LABEL_HEIGHT + tile.above;
            scope.child(GAP, baseline, tile);
        }
        scope.place(0.0, -above, Mark::Frame(width, above + scope.below));
        scope.place(GAP, -above + 3.0, Mark::Caption(caption.into()));
        scope.track([(0.0, 0.0), (GAP, 0.0)]);
        scope.track([(GAP + self.width, 0.0), (width, 0.0)]);
        scope.child(GAP, 0.0, self);
        scope
    }

    fn draw(self, painter: &mut Painter<'_>, (left, base): Point) {
        for ((dx, dy), mark) in self.marks {
            let (x, y) = (left + dx, base + dy);
            match mark {
                Mark::Child(tile) => tile.draw(painter, (x, y)),
                Mark::Track(points) => painter.track((x, y), &points),
                Mark::Atom(text, kind, width) => painter.atom((x, y), &text, kind, width),
                Mark::Caption(label) => painter.scene.text(x, y, &label, None),
                Mark::Arrow(direction) => painter.arrow(x, y, direction),
                Mark::Frame(w, h) => {
                    painter
                        .scene
                        .rect(x, y, w, h, 5.0, None, Some(painter.stroke))
                }
            }
        }
    }
}

struct Painter<'a> {
    scene: &'a mut Scene,
    colors: Colors,
    stroke: Rgb,
}

impl Painter<'_> {
    fn track(&mut self, (x, y): Point, points: &[Point]) {
        let (start_x, start_y) = points[0];
        let mut d = format!("M{} {}", x + start_x, y + start_y);
        for bend in points.windows(3) {
            let (a, b) = bend[1];
            let near = |(u, v): Point| {
                let distance = (u - a).abs() + (v - b).abs();
                let ratio = (8.0 / distance).min(0.5);
                (x + a + (u - a) * ratio, y + b + (v - b) * ratio)
            };
            let (before, after) = (near(bend[0]), near(bend[2]));
            write!(
                d,
                "L{} {}Q{} {} {} {}",
                before.0,
                before.1,
                x + a,
                y + b,
                after.0,
                after.1
            )
            .unwrap();
        }
        let (end_x, end_y) = points.last().unwrap();
        write!(d, "L{} {}", x + end_x, y + end_y).unwrap();
        self.scene.path(&d, None, Some(self.stroke), STROKE);
    }

    fn atom(&mut self, (x, y): Point, text: &str, kind: Atom, width: f32) {
        let style = self.colors.atom(kind);
        let stroke = style.stroke.or(Some(self.stroke));
        let fill = style.fill.or(Some(self.scene.paper()));
        let (top, bottom, right) = (y - 17.0, y + 17.0, x + width);
        match kind {
            Atom::Special | Atom::Any => self.scene.path(
                &format!(
                    "M{} {top}H{}L{right} {y}L{} {bottom}H{}L{x} {y}Z",
                    x + 8.0,
                    right - 8.0,
                    right - 8.0,
                    x + 8.0
                ),
                fill,
                stroke,
                STROKE,
            ),
            _ => {
                let radius = if matches!(kind, Atom::Nonterminal) {
                    0.0
                } else {
                    17.0
                };
                self.scene.rect(x, top, width, 34.0, radius, fill, stroke);
            }
        }
        self.scene
            .text(x + ATOM_PAD, y - LABEL_HEIGHT / 2.0, text, style.text);
    }

    fn station(&mut self, x: f32, y: f32) {
        self.scene
            .rect(x - 4.0, y - 4.0, 8.0, 8.0, 4.0, Some(self.stroke), None);
    }

    fn arrow(&mut self, x: f32, y: f32, direction: f32) {
        let back = x - direction * 8.0;
        self.scene.path(
            &format!("M{x} {y}L{back} {}V{}Z", y - 4.0, y + 4.0),
            Some(self.stroke),
            None,
            0.0,
        );
    }
}
