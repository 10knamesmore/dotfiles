use super::{Branch, Direction, Event, Mark, branch_order};
use crate::{
    Options,
    charts::shared::history::{Strokes, width},
    color::{Layer, Paint, Rgb},
    text::wrap,
};
use std::{cmp::Reverse, collections::BinaryHeap};

pub(super) struct PickTracks {
    pub(super) count: Vec<usize>,
    pub(super) track: Vec<usize>,
}

impl PickTracks {
    pub(super) fn plan(events: &[Event], lanes: usize) -> Self {
        let mut picks: Vec<_> = events
            .iter()
            .enumerate()
            .filter_map(|(end, e)| Some((e.picked?, end)))
            .collect();
        picks.sort_unstable();
        let mut busy = vec![BinaryHeap::<Reverse<(usize, usize)>>::new(); lanes];
        let mut count = vec![0; lanes];
        let mut track = vec![0; events.len()];
        for (start, end) in picks {
            let lane = events[start].lane;
            let n = if let Some(&Reverse((last, n))) = busy[lane].peek()
                && last < start
            {
                busy[lane].pop();
                n
            } else {
                count[lane] += 1;
                count[lane]
            };
            busy[lane].push(Reverse((end, n)));
            track[end] = n;
        }
        Self { count, track }
    }
}

struct Layout {
    time: Vec<usize>,
    lanes: Vec<usize>,
    direction: Direction,
    columns: usize,
    rows: usize,
    labels: Vec<Label>,
}

struct Label {
    x: usize,
    y: usize,
    text: String,
    branch: Option<usize>,
}

fn horizontal(
    branches: &[Branch],
    order: &[usize],
    events: &[Event],
    tracks: &PickTracks,
    options: &Options,
) -> Layout {
    let margin = branches
        .iter()
        .flat_map(|b| b.name.split('\n'))
        .map(width)
        .max()
        .unwrap()
        + 2;
    let mut time = Vec::new();
    let mut x = margin + 2;
    let mut depths: Vec<_> = branches
        .iter()
        .map(|b| b.name.split('\n').count() - 1)
        .collect();
    let mut texts = Vec::new();
    for e in events {
        time.push(x);
        let lines = wrap(&e.caption(), (options.width / 3).clamp(10, 36));
        if e.mark.is_some() {
            depths[e.lane] = depths[e.lane].max(lines.len());
        }
        x += lines.iter().map(|s| width(s)).max().unwrap_or(0).max(1) + 2;
        if e.mark.is_some() && (e.from.is_some() || e.picked.is_some()) {
            x += 2;
        }
        texts.push(lines);
    }
    let mut lanes = vec![0; branches.len()];
    let mut labels = Vec::new();
    let mut y = 0;
    for &b in order {
        y += tracks.count[b];
        lanes[b] = y;
        for (dy, name) in branches[b].name.split('\n').enumerate() {
            labels.push(Label {
                x: 0,
                y: y + dy,
                text: name.to_owned(),
                branch: Some(b),
            });
        }
        y += 2 + depths[b];
    }
    for (i, (e, lines)) in events.iter().zip(texts).enumerate() {
        for (dy, line) in lines.into_iter().enumerate().filter(|(_, s)| !s.is_empty()) {
            let pad = if e.from.is_some() || e.picked.is_some() {
                2
            } else {
                0
            };
            labels.push(Label {
                x: time[i] + pad,
                y: lanes[e.lane] + 1 + dy,
                text: line,
                branch: None,
            });
        }
    }
    Layout {
        time,
        lanes,
        direction: Direction::Right,
        columns: x,
        rows: y - 1,
        labels,
    }
}

fn vertical(
    branches: &[Branch],
    order: &[usize],
    events: &[Event],
    tracks: &PickTracks,
    direction: Direction,
    options: &Options,
) -> Layout {
    let mut lanes = vec![0; branches.len()];
    let mut labels = Vec::new();
    let mut x = 0;
    let mut header = 1;
    let wrap_name = (options.width / (branches.len() + 1)).clamp(10, 24);
    for &b in order {
        let lines = wrap(&branches[b].name, wrap_name);
        let size = lines.iter().map(|s| width(s)).max().unwrap_or(0).max(1);
        header = header.max(lines.len());
        let lane = x + size / 2;
        lanes[b] = lane;
        for (dy, line) in lines.into_iter().enumerate() {
            labels.push(Label {
                x: lane - width(&line) / 2,
                y: dy,
                text: line,
                branch: Some(b),
            });
        }
        x = (x + size).max(lane + tracks.count[b] + 1) + 2;
    }
    let wrap_label = options.width.saturating_sub(x).clamp(10, 80);
    let mut columns = x;
    let mut time = vec![0; events.len()];
    let mut y = header + 1;
    for n in 0..events.len() {
        let i = if direction == Direction::Up {
            events.len() - 1 - n
        } else {
            n
        };
        let e = &events[i];
        let lines = wrap(&e.caption(), wrap_label);
        time[i] = y;
        let step = lines.len() + 1;
        for (k, line) in lines.into_iter().enumerate().filter(|(_, s)| !s.is_empty()) {
            columns = columns.max(x + width(&line));
            labels.push(Label {
                x,
                y: y + k,
                text: line,
                branch: None,
            });
        }
        y += step;
    }
    Layout {
        time,
        lanes,
        direction,
        columns,
        rows: y,
        labels,
    }
}

impl Layout {
    fn along(
        &self,
        strokes: &mut Strokes,
        lane: usize,
        span: (usize, usize),
        dotted: bool,
        color: Option<Rgb>,
    ) {
        strokes.line(
            self.direction == Direction::Right,
            lane,
            span,
            dotted,
            color,
        );
    }
    fn across(
        &self,
        strokes: &mut Strokes,
        time: usize,
        span: (usize, usize),
        dotted: bool,
        color: Option<Rgb>,
    ) {
        strokes.line(
            self.direction != Direction::Right,
            time,
            span,
            dotted,
            color,
        );
    }
}

#[derive(Clone, Copy)]
struct BranchColors {
    line: Option<Rgb>,
    label: Paint,
    highlight: Option<Rgb>,
}

pub(super) fn draw(
    branches: &[Branch],
    events: &[Event],
    direction: Direction,
    options: &Options,
) -> Result<String, String> {
    let theme = options.theme_style();
    let theme_color = |key: &str| {
        if options.color {
            options.config.theme_color(key)
        } else {
            None
        }
    };
    let line = options.theme_line();
    let palette: [_; 8] = std::array::from_fn(|i| {
        let color = theme_color(&format!("git{i}"));
        BranchColors {
            line: color.or(line),
            label: Paint {
                fg: theme_color(&format!("gitBranchLabel{i}")).or(theme.text),
                bg: color,
            },
            highlight: theme_color(&format!("gitInv{i}")),
        }
    });
    if events.is_empty() {
        let mut colors = Layer::default();
        colors.paint(0, 0, 4, palette[0].label);
        colors.paint(
            6,
            0,
            2,
            Paint {
                fg: palette[0].line,
                bg: None,
            },
        );
        let plain = if options.ascii {
            "main  --"
        } else {
            "main  ──"
        };
        return Ok(colors.apply(plain));
    }
    let tracks = PickTracks::plan(events, branches.len());
    let order = branch_order(branches);
    let mut colors = vec![palette[0]; branches.len()];
    for (i, &branch) in order.iter().enumerate() {
        colors[branch] = palette[i % palette.len()];
    }
    let layout = match direction {
        Direction::Auto | Direction::Right => {
            let wide = horizontal(branches, &order, events, &tracks, options);
            if direction == Direction::Auto && wide.columns > options.width {
                vertical(branches, &order, events, &tracks, Direction::Down, options)
            } else {
                wide
            }
        }
        dir => vertical(branches, &order, events, &tracks, dir, options),
    };
    let mut strokes = Strokes::new(layout.columns, layout.rows)?;
    let right = layout.direction == Direction::Right;
    let up = layout.direction == Direction::Up;
    for (n, b) in branches.iter().enumerate() {
        let mut start = layout.time[b.first];
        if n == 0 {
            start = if up { start + 1 } else { start - 1 };
        }
        let end = layout.time[b.last];
        layout.along(
            &mut strokes,
            layout.lanes[n],
            (start, if up { end - 1 } else { end + 1 }),
            false,
            colors[n].line,
        );
    }
    for (i, e) in events.iter().enumerate() {
        let lane = layout.lanes[e.lane];
        let t = layout.time[i];
        if let Some(b) = e.from {
            let branch = if e.mark.is_some() { b } else { e.lane };
            layout.across(
                &mut strokes,
                t,
                (layout.lanes[b], lane),
                false,
                colors[branch].line,
            );
        }
        if let Some(start) = e.picked {
            let source = layout.lanes[events[start].lane];
            let track = if right {
                source - tracks.track[i]
            } else {
                source + tracks.track[i]
            };
            let t0 = layout.time[start];
            let color = colors[e.lane].line;
            layout.across(&mut strokes, t0, (source, track), true, color);
            layout.along(&mut strokes, track, (t0, t), true, color);
            layout.across(&mut strokes, t, (track, lane), true, color);
        }
    }
    let mut canvas = strokes.draw(options.ascii, '╪')?;
    for (i, e) in events.iter().enumerate() {
        if let Some(mark) = e.mark {
            let t = layout.time[i];
            let lane = layout.lanes[e.lane];
            let (x, y) = if right { (t, lane) } else { (lane, t) };
            canvas.put(x, y, mark.glyph(options.ascii))?;
            let color = colors[e.lane];
            let fg = match mark {
                Mark::Highlight => color.highlight.or(color.line),
                _ => color.line,
            };
            canvas.paint(x, y, 1, Paint { fg, bg: None });
        }
    }
    let commit_label = Paint {
        fg: theme_color("commitLabelColor").or(theme.text),
        bg: theme_color("commitLabelBackground"),
    };
    for label in &layout.labels {
        canvas.text(label.x, label.y, &label.text)?;
        let paint = label.branch.map_or(commit_label, |b| colors[b].label);
        canvas.paint(label.x, label.y, width(&label.text), paint);
    }
    Ok(canvas.finish().trim_end().to_owned())
}
