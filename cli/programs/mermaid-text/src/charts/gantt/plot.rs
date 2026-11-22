use std::fmt::Write;

use chrono::{Datelike, NaiveDate, Weekday};
use rustc_hash::FxHashMap;
use unicode_width::UnicodeWidthStr;

use super::{Chart, DAY, Options, Tags, datetime};
use crate::color::{Layer, Paint, Rgb, Style};

pub(super) fn axis_format(source: &str) -> Result<String, String> {
    if source.is_empty() || source.len() > 4096 {
        return Err("invalid gantt axisFormat length".into());
    }
    let mut pattern = String::new();
    let mut chars = source.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            pattern.push(c);
            continue;
        }
        let mut c = chars.next().ok_or("incomplete gantt axisFormat")?;
        let padding = if matches!(c, '-' | '_' | '0') {
            let p = c;
            c = chars.next().ok_or("incomplete gantt axisFormat")?;
            Some(p)
        } else {
            None
        };
        match c {
            'L' if padding.is_none() => pattern.push_str("%3f"),
            'f' if padding.is_none() => pattern.push_str("%6f"),
            'x' if padding.is_none() => pattern.push_str("%m/%d/%Y"),
            'X' if padding.is_none() => pattern.push_str("%H:%M:%S"),
            'c' if padding.is_none() => pattern.push_str("%a %b %e %H:%M:%S %Y"),
            'Z' if padding.is_none() => pattern.push_str("%z"),
            c if "aAbBdeHIjmMSpUuwWVgGyY%sq".contains(c) => {
                pattern.push('%');
                if let Some(p) = padding {
                    pattern.push(p);
                }
                pattern.push(c);
            }
            _ => return Err(format!("unsupported gantt axisFormat specifier `%{c}`")),
        }
    }
    if chrono::format::StrftimeItems::new(&pattern).any(|i| i == chrono::format::Item::Error) {
        return Err(format!("invalid gantt axisFormat `{source}`"));
    }
    Ok(pattern)
}

#[derive(Clone, Copy)]
pub(super) enum Interval {
    Fixed(i64),
    Weeks(i64),
    Months(i64),
}

impl Interval {
    pub fn parse(s: &str) -> Result<Self, String> {
        let invalid = || format!("invalid gantt tickInterval `{s}`");
        let i = s.find(|c: char| !c.is_ascii_digit()).ok_or_else(invalid)?;
        let n: i64 = s[..i].parse().ok().filter(|&n| n > 0).ok_or_else(invalid)?;
        let unit = match &s[i..] {
            "millisecond" => 1,
            "second" => 1000,
            "minute" => 60_000,
            "hour" => 3_600_000,
            "day" => DAY,
            "week" => return Ok(Self::Weeks(n.checked_mul(7 * DAY).ok_or_else(invalid)?)),
            "month" => return Ok(Self::Months(n)),
            "year" => return Ok(Self::Months(n.checked_mul(12).ok_or_else(invalid)?)),
            "decade" => return Ok(Self::Months(n.checked_mul(120).ok_or_else(invalid)?)),
            _ => return Err(invalid()),
        };
        Ok(Self::Fixed(n.checked_mul(unit).ok_or_else(invalid)?))
    }
    pub(super) fn automatic(target: i64) -> Self {
        for step in [
            1,
            2,
            5,
            10,
            20,
            50,
            100,
            250,
            500,
            1000,
            2000,
            5000,
            15_000,
            30_000,
            60_000,
            300_000,
            900_000,
            1_800_000,
            3_600_000,
            3_600_000 * 3,
            3_600_000 * 6,
            3_600_000 * 12,
            DAY,
            2 * DAY,
        ] {
            if step >= target {
                return Self::Fixed(step);
            }
        }
        if target <= 7 * DAY {
            return Self::Weeks(7 * DAY);
        }
        if target <= 14 * DAY {
            return Self::Weeks(14 * DAY);
        }
        for n in [1, 3, 6, 12, 24, 60, 120, 300, 600, 1200] {
            if n * 30 * DAY >= target {
                return Self::Months(n);
            }
        }
        Self::Months(12 * (target / (365 * DAY)).max(1))
    }
    pub(super) fn ceil(self, ms: i64, weekday: Weekday) -> Option<i64> {
        fn ceil_step(value: i64, base: i64, step: i64) -> Option<i64> {
            let offset = (value - base).rem_euclid(step);
            if offset == 0 {
                Some(value)
            } else {
                value.checked_add(step - offset)
            }
        }
        match self {
            Self::Fixed(step) => ceil_step(ms, 0, step),
            Self::Weeks(step) => ceil_step(
                ms,
                (i64::from(weekday.num_days_from_monday()) - 3) * DAY,
                step,
            ),
            Self::Months(step) => {
                let day = datetime(ms).ok()?;
                let mut month = i64::from(day.year()) * 12 + i64::from(day.month0());
                if day.day() > 1 || ms.rem_euclid(DAY) != 0 {
                    month += 1;
                }
                month = ceil_step(month, 0, step)?;
                Some(
                    NaiveDate::from_ymd_opt(
                        month.div_euclid(12).try_into().ok()?,
                        (month.rem_euclid(12) + 1) as u32,
                        1,
                    )?
                    .and_hms_opt(0, 0, 0)?
                    .and_utc()
                    .timestamp_millis(),
                )
            }
        }
    }
}

struct Axis {
    first: i64,
    last: i64,
    cols: usize,
}

impl Axis {
    fn column(&self, ms: i64) -> usize {
        if self.first == self.last {
            return self.cols / 2;
        }
        let span = i128::from(self.last - self.first);
        ((i128::from(ms - self.first) * (self.cols - 1) as i128 + span / 2) / span) as usize
    }
    fn time(&self, x: usize) -> i64 {
        self.first
            + (i128::from(self.last - self.first) * x as i128 / (self.cols - 1) as i128) as i64
    }
}

pub(super) fn date_label(ms: i64, fmt: &str) -> Result<String, String> {
    let mut s = String::new();
    write!(s, "{}", datetime(ms)?.format(fmt))
        .map_err(|_| "invalid gantt axisFormat".to_owned())?;
    Ok(s)
}

struct Grid {
    background: Vec<char>,
    tick_cols: Vec<bool>,
    markers: Vec<Option<Paint>>,
    labels: String,
}

impl Grid {
    fn new(chart: &Chart, axis: &Axis, fmt: &str, ascii: bool) -> Result<Self, String> {
        let n = axis.cols;
        let first_label = date_label(axis.first, fmt)?;
        let last_label = date_label(axis.last, fmt)?;
        let label_width = first_label.width().max(last_label.width());
        let interval = chart.ticks.unwrap_or_else(|| {
            let spacing = (axis.last - axis.first) / (n / (label_width + 3)).max(1) as i64;
            Interval::automatic(if fmt == "%Y-%m-%d" {
                spacing.max(DAY)
            } else {
                spacing
            })
        });
        let mut ticks = vec![None; n];
        for i in 0..n {
            // One calendar lookup per screen column even with millisecond ticks
            // on an axis spanning years.
            if let Some(t) = interval.ceil(axis.time(i), chart.weekday)
                && t >= axis.first
                && t <= axis.last
            {
                ticks[axis.column(t)].get_or_insert(t);
            }
        }
        let a = axis.column(axis.first);
        let b = axis.column(axis.last);
        ticks[a] = Some(axis.first);
        ticks[b] = Some(axis.last);
        let mut labels = String::new();
        let mut used = 0;
        for (x, t) in ticks.iter().enumerate() {
            let Some(ms) = *t else { continue };
            let text = date_label(ms, fmt)?;
            if x != a && x != b && (text == first_label || text == last_label) {
                continue;
            }
            let width = text.width();
            let left = x.saturating_sub(width / 2).min(n.saturating_sub(width));
            if (used > 0 && left <= used)
                || (x != a && x != b && left + width + 1 >= n - last_label.width())
            {
                continue;
            }
            labels.push_str(&" ".repeat(left - used));
            labels.push_str(&text);
            used = left + width;
        }
        let mut background = vec![' '; n];
        if a != b {
            ticks[a] = None;
            ticks[b] = None;
        }
        for (x, c) in background.iter_mut().enumerate() {
            if ticks[x].is_some() {
                *c = if ascii { ':' } else { '┊' };
            } else if axis.last - axis.first < n as i64 * DAY
                && chart.calendar.excluded(axis.time(x))?
            {
                *c = if ascii { '.' } else { '░' };
            }
        }
        Ok(Self {
            background,
            tick_cols: ticks.iter().map(Option::is_some).collect(),
            markers: vec![None; n],
            labels,
        })
    }
    fn rail(&self, junction: char, ascii: bool) -> String {
        self.tick_cols
            .iter()
            .zip(&self.markers)
            .map(|(&t, m)| match (ascii, t || m.is_some()) {
                (true, true) => '+',
                (true, false) => '-',
                (false, false) => '─',
                (false, true) => junction,
            })
            .collect()
    }
}

fn ink(fg: Option<Rgb>) -> Paint {
    Paint { fg, bg: None }
}

#[derive(Default)]
pub(super) struct Palette {
    task: Style,
    active: Style,
    crit: Style,
    done: Style,
    outside: Option<Rgb>,
    dark: Option<Rgb>,
    clickable: Option<Rgb>,
    pub(super) grid: Paint,
    pub(super) excluded: Paint,
    pub(super) title: Paint,
    pub(super) today: Paint,
    pub(super) vert: Paint,
}

impl Palette {
    pub(super) fn new(chart: &Chart, options: &Options) -> Self {
        if !options.color {
            return Self::default();
        }
        let color = |key| options.config.theme_color(key);
        let style = |fill, stroke| Style {
            fill: color(fill),
            stroke: color(stroke),
            text: None,
        };
        Self {
            task: options.theme_style().overlay(Style {
                text: color("taskTextColor").or(color("taskTextLightColor")),
                ..style("taskBkgColor", "taskBorderColor")
            }),
            active: style("activeTaskBkgColor", "activeTaskBorderColor"),
            crit: style("critBkgColor", "critBorderColor"),
            done: style("doneTaskBkgColor", "doneTaskBorderColor"),
            outside: color("taskTextOutsideColor"),
            dark: color("taskTextDarkColor"),
            clickable: color("taskTextClickableColor"),
            grid: ink(color("gridColor").or(options.theme_line())),
            excluded: Paint {
                fg: None,
                bg: color("excludeBkgColor"),
            },
            title: ink(color("titleColor")),
            today: ink(chart.today_stroke.or(color("todayLineColor"))),
            vert: ink(color("vertLineColor")),
        }
    }

    pub(super) fn task_style(&self, tags: &Tags, clickable: bool) -> Style {
        let status = if tags.active {
            self.active
        } else if tags.done {
            self.done
        } else if tags.crit {
            self.crit
        } else {
            Style::default()
        };
        let mut style = self.task.overlay(status);
        if tags.crit {
            style.stroke = self.crit.stroke.or(style.stroke);
        }
        let label = if tags.active && !tags.done {
            self.dark.or(self.outside)
        } else {
            self.outside.or(self.dark)
        };
        style.text = self
            .clickable
            .filter(|_| clickable)
            .or(label)
            .or(style.text);
        style
    }
}

#[derive(Default)]
struct Output {
    text: String,
    row: usize,
    layer: Layer,
}

impl Write for Output {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.row += s.bytes().filter(|&b| b == b'\n').count();
        self.text.push_str(s);
        Ok(())
    }
}

impl Output {
    fn paint(&mut self, x: usize, width: usize, paint: Paint) {
        self.layer.paint(x, self.row, width, paint);
    }

    fn paint_label(&mut self, text: &str, color: Paint) {
        for (dy, line) in text.lines().enumerate() {
            self.layer.paint(0, self.row + dy, line.width(), color);
        }
    }

    fn paint_grid(&mut self, left: usize, grid: &Grid, colors: &Palette, body: Option<&[char]>) {
        for (x, &background) in grid.background.iter().enumerate() {
            if body.is_some_and(|cells| cells[x] != background && !matches!(cells[x], '+' | '╋'))
            {
                continue;
            }
            if let Some(marker) = grid.markers[x] {
                self.paint(left + x, 1, marker);
            } else if grid.tick_cols[x] {
                self.paint(left + x, 1, colors.grid);
            } else if body.is_some() && background != ' ' {
                self.paint(left + x, 1, colors.excluded);
            }
        }
    }
}

fn caption(name: &str, tags: &Tags) -> String {
    let mut label = name.to_owned();
    let flags: Vec<_> = [
        ("crit", tags.crit),
        ("active", tags.active),
        ("done", tags.done),
    ]
    .into_iter()
    .filter_map(|(s, set)| set.then_some(s))
    .collect();
    if !flags.is_empty() {
        write!(label, " [{}]", flags.join(", ")).unwrap();
    }
    label
}

fn check_output(out: &Output) -> Result<(), String> {
    if out.text.len() > 8 * 1024 * 1024 {
        return Err("gantt output exceeds 8 MiB".into());
    }
    Ok(())
}

pub(super) fn time_domain<'a>(chart: &'a Chart<'_>, times: &[[i64; 2]]) -> (i64, i64, &'a str) {
    let first = times
        .iter()
        .map(|[start, _]| *start)
        .min()
        .unwrap_or(chart.now - chart.now.rem_euclid(DAY));
    let last = times.iter().map(|[_, end]| *end).max().unwrap_or(first);
    let same_day = first.div_euclid(DAY) == last.div_euclid(DAY);
    let auto_fmt =
        if first.rem_euclid(DAY) == 0 && last.rem_euclid(DAY) == 0 || last - first >= 7 * DAY {
            "%Y-%m-%d"
        } else if !same_day {
            "%m-%d %H:%M"
        } else if first % 1000 != 0 || last % 1000 != 0 || last - first < 1000 {
            "%H:%M:%S.%3f"
        } else if first % 60_000 != 0 || last % 60_000 != 0 || last - first < 60_000 {
            "%H:%M:%S"
        } else {
            "%H:%M"
        };
    (first, last, chart.axis.as_deref().unwrap_or(auto_fmt))
}

pub(super) fn captions(chart: &Chart) -> Vec<(String, bool)> {
    let mut notes = FxHashMap::<&str, Vec<usize>>::default();
    for (i, (ids, _)) in chart.notes.iter().enumerate() {
        for &id in ids {
            notes.entry(id).or_default().push(i + 1);
        }
    }
    chart
        .tasks
        .iter()
        .map(|t| {
            let mut text = caption(&t.label, &t.tags);
            let refs = notes.get(t.id.as_str());
            if let Some(refs) = refs {
                for i in refs {
                    write!(text, " [{i}]").unwrap();
                }
            }
            (text, refs.is_some())
        })
        .collect()
}

pub(super) fn draw(chart: &Chart, times: &[[i64; 2]], options: &Options) -> Result<String, String> {
    let ascii = options.ascii;
    let (first, last, fmt) = time_domain(chart, times);
    let labels = captions(chart);
    let longest = labels
        .iter()
        .flat_map(|(s, _)| s.lines())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let axis_min = 16.max(date_label(first, fmt)?.width() + date_label(last, fmt)?.width() + 3);
    let stacked = longest + 6 + axis_min > options.width;
    let gutter = if stacked { 2 } else { longest + 4 };
    let cols = options.width.saturating_sub(gutter + 2).max(axis_min);
    let axis = Axis { first, last, cols };
    let mut grid = Grid::new(chart, &axis, fmt, ascii)?;
    let colors = Palette::new(chart, options);
    let mut annotations = vec![];
    for (i, t) in chart.tasks.iter().enumerate() {
        if t.tags.vert {
            let [start, end] = times[i];
            annotations.push((
                axis.column(start + (end - start) / 2),
                labels[i].0.as_str(),
                '┃',
                colors.vert,
            ));
        }
    }
    if let Some(glyph) = chart.today_marker
        && (first..=last).contains(&chart.now)
    {
        annotations.push((axis.column(chart.now), "today", glyph, colors.today));
    }
    let prefix = " ".repeat(gutter);
    let mut out = Output::default();
    if !chart.title.is_empty() {
        out.paint_label(&chart.title, colors.title);
        writeln!(out, "{}\n", chart.title).unwrap();
    }
    for (x, label, glyph, paint) in annotations {
        grid.markers[x] = Some(paint);
        grid.background[x] = if ascii { '|' } else { glyph };
        let text = label.replace('\n', " ");
        let mark = if ascii { 'v' } else { '▼' };
        if x + 2 + text.width() > cols && text.width() + 1 < x {
            out.paint(gutter + x - text.width(), text.width() + 2, paint);
            writeln!(
                out,
                "{prefix} {}{text} {mark}",
                " ".repeat(x - text.width() - 1)
            )
            .unwrap();
        } else {
            out.paint(gutter + 1 + x, text.width() + 2, paint);
            writeln!(out, "{prefix} {}{mark} {text}", " ".repeat(x)).unwrap();
        }
        check_output(&out)?;
    }
    let border = if ascii { '|' } else { '│' };
    let frame = |l, j, r| {
        format!(
            "{}{}{}",
            if ascii { '+' } else { l },
            grid.rail(j, ascii),
            if ascii { '+' } else { r }
        )
    };
    let [tl, tr, bl, br] = options.corners.resolve(ascii, ['┌', '┐', '└', '┘']);
    let top = frame(tl, '┬', tr);
    let divider = frame('├', '┼', '┤');
    let bottom = frame(bl, '┴', br);
    if chart.top_axis {
        writeln!(out, "{prefix} {}", grid.labels).unwrap();
    }
    let mut sections = chart.sections.iter().peekable();
    for i in 0..=chart.tasks.len() {
        let mut section_printed = false;
        while sections.peek().is_some_and(|(at, _)| *at == i) {
            let (_, name) = sections.next().unwrap();
            out.paint_label(name, colors.title);
            let rail = if i == 0 && !section_printed {
                &top
            } else {
                &divider
            };
            if !stacked && !name.contains('\n') && name.width() < gutter {
                out.paint_grid(gutter + 1, &grid, &colors, None);
                writeln!(out, "{name}{}{rail}", " ".repeat(gutter - name.width())).unwrap();
            } else {
                writeln!(out, "{name}").unwrap();
                out.paint_grid(gutter + 1, &grid, &colors, None);
                writeln!(out, "{prefix}{rail}").unwrap();
            }
            section_printed = true;
            check_output(&out)?;
        }
        if i == 0 && !section_printed {
            out.paint_grid(gutter + 1, &grid, &colors, None);
            writeln!(out, "{prefix}{top}").unwrap();
        }
        if i == chart.tasks.len() {
            if chart.tasks.is_empty() {
                out.paint_grid(gutter + 1, &grid, &colors, Some(&grid.background));
                writeln!(
                    out,
                    "{prefix}{border}{}{border}",
                    grid.background.iter().collect::<String>()
                )
                .unwrap();
            }
            break;
        }
        let t = &chart.tasks[i];
        if t.tags.vert {
            continue;
        }
        let mut row = grid.background.clone();
        let [start, end] = times[i];
        let x = axis.column(start);
        let bar = if t.tags.milestone {
            let mid = axis.column(start + (end - start) / 2);
            row[mid] = if ascii { '*' } else { '◆' };
            mid..mid + 1
        } else {
            let fill = if ascii {
                if t.tags.crit {
                    '!'
                } else if t.tags.active {
                    '='
                } else if t.tags.done {
                    '%'
                } else {
                    '#'
                }
            } else if t.tags.crit {
                '▓'
            } else if t.tags.active {
                '▰'
            } else if t.tags.done {
                '▒'
            } else {
                '█'
            };
            let right = axis.column(end).max(x + 1);
            for (col, c) in row.iter_mut().enumerate().take(right).skip(x) {
                *c = if grid.markers[col].is_some() {
                    if ascii { '+' } else { '╋' }
                } else {
                    fill
                };
            }
            x..right
        };
        let (label, clickable) = &labels[i];
        let style = colors.task_style(&t.tags, *clickable);
        if stacked {
            for text in label.split('\n') {
                out.paint(gutter + 1, text.width(), ink(style.text));
                writeln!(
                    out,
                    "{prefix}{border}{text}{}{border}",
                    " ".repeat(cols.saturating_sub(text.width()))
                )
                .unwrap();
                check_output(&out)?;
            }
        }
        let left = gutter + 1 + bar.start;
        out.paint(
            left,
            bar.len(),
            Paint {
                fg: None,
                bg: style.fill,
            },
        );
        if style.fill.is_some() {
            let mut col = left;
            for run in row[bar.clone()].chunk_by(|a, b| a == b) {
                if run[0] == '█' {
                    out.paint(col, run.len(), ink(style.fill));
                }
                col += run.len();
            }
        }
        out.paint(left, 1, style.border());
        out.paint(left + bar.len() - 1, 1, style.border());
        out.paint_grid(gutter + 1, &grid, &colors, Some(&row));
        let body: String = row.into_iter().collect();
        if stacked {
            writeln!(out, "{prefix}{border}{body}{border}").unwrap();
        } else {
            let mut lines = label.split('\n');
            let text = lines.next().unwrap();
            out.paint(2, text.width(), ink(style.text));
            writeln!(
                out,
                "  {text}{}{border}{body}{border}",
                " ".repeat(gutter - 2 - text.width()),
            )
            .unwrap();
            for text in lines {
                out.paint(2, text.width(), ink(style.text));
                writeln!(
                    out,
                    "  {text}{}{border}{}{border}",
                    " ".repeat(gutter - 2 - text.width()),
                    " ".repeat(cols)
                )
                .unwrap();
                check_output(&out)?;
            }
        }
        check_output(&out)?;
    }
    out.paint_grid(gutter + 1, &grid, &colors, None);
    writeln!(out, "{prefix}{bottom}").unwrap();
    write!(out, "{prefix} {}", grid.labels).unwrap();
    if !chart.description.is_empty() {
        write!(out, "\n\n{}", chart.description).unwrap();
    }
    if !chart.notes.is_empty() {
        writeln!(out).unwrap();
        for (i, (_, text)) in chart.notes.iter().enumerate() {
            write!(out, "\n[{}] {text}", i + 1).unwrap();
        }
    }
    check_output(&out)?;
    Ok(out.layer.apply(&out.text))
}
