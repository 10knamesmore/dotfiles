use super::shared::board::{
    self, Pen, Rows, indent, paint, panel, theme_color,
    tree::{Forest, Node, branches},
    width,
};
use crate::{
    Options,
    canvas::Canvas,
    color::{Paint, Style},
    text::wrap,
};

mod image;

const HEADERS: &[&str] = &["ishikawa", "ishikawa-beta"];

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let body = board::body(source, HEADERS)?;
        render(body, options, Pen::new(options))
    }
}

struct FishPanel {
    rows: Rows,
    guide_widths: Vec<usize>,
}

impl FishPanel {
    fn width(&self) -> usize {
        width(&self.rows[0])
    }
    fn height(&self) -> usize {
        self.rows.len()
    }

    fn draw(&self, canvas: &mut Canvas, x: usize, y: usize, style: Style) -> Result<(), String> {
        paint(canvas, x, y, &self.rows)?;
        canvas.paint_box(x, y, self.width(), self.height(), style);
        if !self.guide_widths.is_empty() {
            let first_cause = y + self.height() - 1 - self.guide_widths.len();
            canvas.paint(x, first_cause - 1, self.width(), style.border());
            for (dy, &prefix) in self.guide_widths.iter().enumerate() {
                canvas.paint(x + 2, first_cause + dy, prefix, style.border());
            }
        }
        Ok(())
    }
}

struct FishPen {
    pen: Pen,
    style: Style,
}

impl FishPen {
    fn stroke(&self) -> Paint {
        Paint {
            fg: self.style.stroke,
            bg: None,
        }
    }
    fn put(&self, canvas: &mut Canvas, x: usize, y: usize, ch: char) -> Result<(), String> {
        canvas.put(x, y, self.pen.ch(ch))?;
        canvas.paint(x, y, 1, self.stroke());
        Ok(())
    }
}

struct Fish<'a> {
    effect: &'a str,
    forest: Forest<&'a str>,
}

impl<'a> Fish<'a> {
    fn categories(&self) -> impl Iterator<Item = (usize, &[Node<&'a str>])> {
        let nodes = &self.forest.nodes;
        let mut start = 0;
        nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, node)| (node.depth == 0).then_some(i))
            .chain(std::iter::once(nodes.len()))
            .filter_map(move |end| {
                let from = std::mem::replace(&mut start, end);
                (from < end).then(|| (from, &nodes[from..end]))
            })
    }
}

fn parse(body: &str) -> Result<Fish<'_>, String> {
    let mut lines = body
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("%%"));
    let effect = lines.next().ok_or("ishikawa needs an effect")?.trim();
    let mut forest = Forest::default();
    for line in lines {
        let text = line.trim_start();
        forest.push(indent(&line[..line.len() - text.len()]), text.trim_end())?;
    }
    Ok(Fish { effect, forest })
}

fn fish_style(options: &Options) -> Style {
    Style {
        stroke: options.theme_line(),
        fill: theme_color(options, "mainBkg"),
        text: None,
    }
}

fn render(body: &str, options: &Options, pen: Pen) -> Result<String, String> {
    let fish = parse(body)?;
    let limit = options.width;
    let head = FishPanel {
        rows: panel(&[wrap(fish.effect, (limit / 3).clamp(10, 32))], 0, pen),
        guide_widths: vec![],
    };
    let head_w = head.width();
    let categories: Vec<_> = fish.categories().collect();
    let pairs = categories.len().div_ceil(2);
    let horizontal_space = limit.saturating_sub(head_w + 6) / pairs.max(1);
    let horizontal = horizontal_space >= 17 || pairs == 0;
    let two_sides = limit >= 43;
    let inner = if horizontal {
        horizontal_space.saturating_sub(6)
    } else if two_sides {
        limit.saturating_sub(9) / 2 - 4
    } else {
        limit.saturating_sub(10)
    }
    .clamp(8, 44);
    let mut bones = Vec::new();
    for (_, nodes) in categories {
        let (category, causes) = nodes.split_first().unwrap();
        let mut groups = vec![wrap(category.label, inner)];
        let mut guide_widths = Vec::new();
        if !causes.is_empty() {
            groups.push(branches(
                causes,
                1,
                true,
                inner,
                pen,
                |label, _, x, columns| {
                    let lines = wrap(label, columns);
                    guide_widths.extend(std::iter::repeat_n(x, lines.len()));
                    lines
                },
            ));
        }
        bones.push(FishPanel {
            rows: panel(&groups, 0, pen),
            guide_widths,
        });
    }
    let pen = FishPen {
        pen,
        style: fish_style(options),
    };
    if horizontal {
        horizontal_fish(&head, &bones, pen)
    } else {
        vertical_fish(&head, &bones, two_sides, pen)
    }
}

fn horizontal_fish(head: &FishPanel, bones: &[FishPanel], pen: FishPen) -> Result<String, String> {
    let above = bones
        .iter()
        .step_by(2)
        .map(FishPanel::height)
        .max()
        .unwrap_or(0);
    let below = bones
        .iter()
        .skip(1)
        .step_by(2)
        .map(FishPanel::height)
        .max()
        .unwrap_or(0);
    let mut stations = Vec::new();
    let mut x = 0;
    for pair in bones.chunks(2) {
        let w = pair.iter().map(FishPanel::width).max().unwrap();
        stations.push(x + w / 2);
        x += w + 2;
    }
    let head_x = x + 4;
    let spine = (above + usize::from(above > 0)).max(head.height() / 2);
    let h = spine + (below + 2).max(head.height() - head.height() / 2);
    let mut canvas = Canvas::new(head_x + head.width(), h)?;
    canvas.text(0, spine, &pen.pen.stroke('─', head_x - 1))?;
    canvas.paint(0, spine, head_x - 1, pen.stroke());
    pen.put(&mut canvas, head_x - 1, spine, '▶')?;
    head.draw(&mut canvas, head_x, spine - head.height() / 2, pen.style)?;
    for (mid, pair) in stations.into_iter().zip(bones.chunks(2)) {
        for (i, p) in pair.iter().enumerate() {
            let top = i == 0;
            let y = if top {
                spine - p.height() - 1
            } else {
                spine + 2
            };
            p.draw(&mut canvas, mid - p.width() / 2, y, pen.style)?;
            pen.put(
                &mut canvas,
                mid,
                if top { spine - 2 } else { spine + 2 },
                if top { '┬' } else { '┴' },
            )?;
            pen.put(
                &mut canvas,
                mid + 1,
                if top { spine - 1 } else { spine + 1 },
                if top { '╲' } else { '╱' },
            )?;
        }
        pen.put(
            &mut canvas,
            mid + 2,
            spine,
            if pair.len() == 2 { '┼' } else { '┴' },
        )?;
    }
    Ok(canvas.finish().trim_end().to_owned())
}

fn vertical_fish(
    head: &FishPanel,
    bones: &[FishPanel],
    two_sides: bool,
    pen: FishPen,
) -> Result<String, String> {
    let n = if two_sides { 2 } else { 1 };
    let spine = bones.iter().step_by(n).map(FishPanel::width).max().unwrap() + 3;
    let right = if two_sides {
        bones
            .iter()
            .skip(1)
            .step_by(2)
            .map(|p| p.width() + 4)
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    let head_w = head.width();
    let head_x = spine.saturating_sub(if right == 0 { head_w - 2 } else { head_w / 2 });
    let w = (spine + 1 + right).max(head_x + head.width());
    let mut y = 0;
    let mut stations = Vec::new();
    for pair in bones.chunks(n) {
        stations.push(y + 1);
        y += pair.iter().map(FishPanel::height).max().unwrap().max(5) + 1;
    }
    let mut canvas = Canvas::new(w, y + 1 + head.height())?;
    for row in 0..y {
        pen.put(&mut canvas, spine, row, '│')?;
    }
    for (mid, pair) in stations.into_iter().zip(bones.chunks(n)) {
        for (i, p) in pair.iter().enumerate() {
            let left = i == 0;
            let x = if left {
                spine - 3 - p.width()
            } else {
                spine + 4
            };
            p.draw(&mut canvas, x, mid - 1, pen.style)?;
            let port = if left { spine - 4 } else { spine + 4 };
            pen.put(&mut canvas, port, mid, if left { '├' } else { '┤' })?;
            pen.put(
                &mut canvas,
                if left { spine - 3 } else { spine + 3 },
                mid,
                '─',
            )?;
            for offset in 1..=2 {
                let x = if left {
                    spine - 3 + offset
                } else {
                    spine + 3 - offset
                };
                pen.put(&mut canvas, x, mid + offset, if left { '╲' } else { '╱' })?;
            }
        }
        pen.put(&mut canvas, spine, mid + 3, '◆')?;
    }
    pen.put(&mut canvas, spine, y, '▼')?;
    head.draw(&mut canvas, head_x, y + 1, pen.style)?;
    Ok(canvas.finish())
}
