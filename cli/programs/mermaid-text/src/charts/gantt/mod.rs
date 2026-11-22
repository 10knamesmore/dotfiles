mod dates;
mod image;
mod plot;

use std::time::SystemTime;

use chrono::{DateTime, NaiveDate, Utc, Weekday};
use rustc_hash::FxHashMap;

use crate::Options;
use crate::color::Rgb;
use dates::{Calendar, DAY, DateFormat, Span, datetime};
use plot::Interval;

#[derive(Default)]
struct Tags {
    active: bool,
    crit: bool,
    done: bool,
    milestone: bool,
    vert: bool,
}

struct Task<'a> {
    label: String,
    tags: Tags,
    id: String,
    start: Option<&'a str>,
    end: &'a str,
}

fn label(s: &str) -> String {
    s.replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br\t/>", "\n")
        .replace('\t', " ")
}

fn task<'a>(name: &str, data: &'a str, anonymous: &mut usize) -> Result<Task<'a>, String> {
    let fields: Vec<_> = data.split(',').map(str::trim).collect();
    let mut rest = fields.as_slice();
    let mut tags = Tags::default();
    while let Some((&first, more)) = rest.split_first() {
        match first {
            "active" => tags.active = true,
            "crit" => tags.crit = true,
            "done" => tags.done = true,
            "milestone" => tags.milestone = true,
            "vert" => tags.vert = true,
            _ => break,
        }
        rest = more;
    }
    let point = tags.milestone || tags.vert;
    if name.is_empty()
        || rest
            .iter()
            .enumerate()
            .any(|(i, s)| s.is_empty() && !(point && i + 1 == rest.len()))
    {
        return Err(format!("empty field in gantt task `{name}`"));
    }
    let (id, start, end) = match *rest {
        [start, ""] if point => (None, Some(start), "0d"),
        [id, start, ""] if point && !id.contains(char::is_whitespace) => {
            (Some(id), Some(start), "0d")
        }
        [end] => (None, None, end),
        [start, end] => (None, Some(start), end),
        [id, start, end] if !id.contains(char::is_whitespace) => (Some(id), Some(start), end),
        _ => return Err(format!("invalid gantt task fields for `{name}`: {data}")),
    };
    let id = id.map(str::to_owned).unwrap_or_else(|| {
        *anonymous += 1;
        format!("task{anonymous}")
    });
    Ok(Task {
        label: label(name),
        tags,
        id,
        start,
        end,
    })
}

struct Chart<'a> {
    title: String,
    description: String,
    sections: Vec<(usize, String)>,
    tasks: Vec<Task<'a>>,
    notes: Vec<(Vec<&'a str>, String)>,
    dates: DateFormat,
    calendar: Calendar,
    axis: Option<String>,
    ticks: Option<Interval>,
    weekday: Weekday,
    today_marker: Option<char>,
    today_stroke: Option<Rgb>,
    today_width: f32,
    today_opacity: f32,
    top_axis: bool,
    inclusive_end: bool,
    now: i64,
    today: NaiveDate,
}

struct Dependency {
    finish: usize,
    start: usize,
    until: bool,
}

struct Schedule {
    times: Vec<[i64; 2]>,
    dependencies: Vec<Dependency>,
}

fn without_comment(s: &str) -> &str {
    let s = s.split_once("%%").map_or(s, |(v, _)| v);
    s.split_once(" #").map_or(s, |(v, _)| v).trim()
}

fn click_note(value: &str) -> Result<(Vec<&str>, String), String> {
    let invalid = || format!("invalid gantt click statement: `{value}`");
    let (ids, mut actions) = value.split_once(char::is_whitespace).ok_or_else(invalid)?;
    let ids: Vec<_> = ids.split(',').collect();
    if ids.contains(&"") {
        return Err(invalid());
    }
    let mut descriptions = Vec::new();
    while !actions.trim().is_empty() {
        let (action, rest) = actions
            .trim_start()
            .split_once(char::is_whitespace)
            .ok_or_else(invalid)?;
        let rest = rest.trim_start();
        match action {
            "href" => {
                let (url, tail) = rest
                    .strip_prefix('"')
                    .and_then(|s| s.split_once('"'))
                    .ok_or_else(invalid)?;
                if url.is_empty() {
                    return Err(invalid());
                }
                if let Some(tooltip) = tail.trim_start().strip_prefix('"') {
                    let (tooltip, tail) = tooltip.split_once('"').ok_or_else(invalid)?;
                    descriptions.push(format!("{tooltip}: {url}"));
                    actions = tail;
                } else {
                    descriptions.push(url.to_owned());
                    actions = tail;
                }
            }
            "call" => {
                let open = rest.find('(').ok_or_else(invalid)?;
                let name = &rest[..open];
                if name.is_empty()
                    || !name
                        .chars()
                        .all(|c| c.is_alphanumeric() || "_.$".contains(c))
                {
                    return Err(invalid());
                }
                let mut quote = None;
                let mut escaped = false;
                let mut depth = 0;
                let mut close = None;
                for (i, c) in rest[open..].char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if let Some(q) = quote {
                        if c == '\\' {
                            escaped = true;
                        } else if c == q {
                            quote = None;
                        }
                    } else if c == '"' || c == '\'' {
                        quote = Some(c);
                    } else if c == '(' {
                        depth += 1;
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(open + i + 1);
                            break;
                        }
                    }
                }
                let (callback, tail) = rest.split_at(close.ok_or_else(invalid)?);
                descriptions.push(callback.to_owned());
                actions = tail;
            }
            _ => return Err(invalid()),
        }
    }
    if descriptions.is_empty() {
        return Err(invalid());
    }
    Ok((ids, descriptions.join("; ")))
}

impl<'a> Chart<'a> {
    fn parse(source: &'a str) -> Result<Self, String> {
        let now = DateTime::<Utc>::from(SystemTime::now());
        let mut chart = Self {
            title: String::new(),
            description: String::new(),
            sections: vec![],
            tasks: vec![],
            notes: vec![],
            dates: DateFormat::new("YYYY-MM-DD")?,
            calendar: Calendar::default(),
            axis: None,
            ticks: None,
            weekday: Weekday::Sun,
            today_marker: Some('┃'),
            today_stroke: None,
            today_width: 2.0,
            today_opacity: 1.0,
            top_axis: false,
            inclusive_end: false,
            now: now.timestamp_millis(),
            today: now.date_naive(),
        };
        let mut lines = source.lines().map(str::trim);
        if lines.next() != Some("gantt") {
            return Err("expected `gantt` header".into());
        }
        let mut anonymous = 0;
        let mut acc_title = "";
        let mut calendar_entries = vec![];
        let mut weekend = Weekday::Sat;
        while let Some(line) = lines.next() {
            if line.is_empty() || line.starts_with("%%") {
                continue;
            }
            if let Some(text) = line.strip_prefix("accTitle:") {
                acc_title = text.trim();
                continue;
            }
            if let Some(text) = line.strip_prefix("accDescr:") {
                chart.description = label(text.trim());
                continue;
            }
            let (key, value) = line
                .split_once(char::is_whitespace)
                .map_or((line, ""), |(k, v)| (k, v.trim()));
            match key {
                "title" => chart.title = label(value),
                "section" if !value.is_empty() => {
                    chart.sections.push((chart.tasks.len(), label(value)))
                }
                "accDescr" if value.starts_with('{') => {
                    let mut content = value[1..].trim_start().to_owned();
                    while !content.ends_with('}') {
                        let next = lines.next().ok_or("unterminated gantt accDescr block")?;
                        content.push('\n');
                        content.push_str(next);
                    }
                    content.pop();
                    chart.description = label(content.trim());
                }
                "dateFormat" if !value.is_empty() => {
                    chart.dates = DateFormat::new(without_comment(value))?
                }
                "axisFormat" if !value.is_empty() => {
                    chart.axis = Some(plot::axis_format(without_comment(value))?)
                }
                "tickInterval" => chart.ticks = Some(Interval::parse(without_comment(value))?),
                "weekday" => {
                    chart.weekday = without_comment(value)
                        .parse()
                        .map_err(|_| format!("invalid gantt weekday `{value}`"))?
                }
                "weekend" => {
                    weekend = match without_comment(value) {
                        "friday" => Weekday::Fri,
                        "saturday" => Weekday::Sat,
                        _ => return Err(format!("invalid gantt weekend `{value}`")),
                    }
                }
                "includes" | "excludes" if !value.is_empty() => {
                    calendar_entries.extend(
                        without_comment(value)
                            .split(|c: char| c.is_whitespace() || c == ',')
                            .filter(|v| !v.is_empty())
                            .map(|v| (key, v)),
                    );
                }
                "inclusiveEndDates" if value.is_empty() => chart.inclusive_end = true,
                "topAxis" if value.is_empty() => chart.top_axis = true,
                "todayMarker" if value == "off" => chart.today_marker = None,
                "todayMarker" => {
                    let mut visible = true;
                    let mut glyph = '│';
                    chart.today_stroke = None;
                    chart.today_width = 1.0;
                    chart.today_opacity = 1.0;
                    let mut depth = 0usize;
                    for property in value.trim_end_matches(';').split(|ch| match ch {
                        '(' => {
                            depth += 1;
                            false
                        }
                        ')' => {
                            depth = depth.saturating_sub(1);
                            false
                        }
                        ',' | ';' => depth == 0,
                        _ => false,
                    }) {
                        let (name, v) = property
                            .trim()
                            .split_once(':')
                            .ok_or("expected gantt todayMarker CSS declaration")?;
                        let v = v.trim();
                        match name.trim() {
                            "stroke" if !v.is_empty() => {
                                visible &= !matches!(v, "none" | "transparent");
                                chart.today_stroke = Rgb::parse(v);
                            }
                            "opacity" | "stroke-opacity" => {
                                let a: f32 =
                                    v.parse().map_err(|_| "invalid gantt todayMarker opacity")?;
                                if !(0.0..=1.0).contains(&a) {
                                    return Err("invalid gantt todayMarker opacity".into());
                                }
                                visible &= a > 0.0;
                                chart.today_opacity *= a;
                            }
                            "stroke-width" => {
                                let w: f32 = v
                                    .trim_end_matches("px")
                                    .parse()
                                    .map_err(|_| "invalid gantt todayMarker width")?;
                                if w < 0.0 || !w.is_finite() {
                                    return Err("invalid gantt todayMarker width".into());
                                }
                                visible &= w > 0.0;
                                chart.today_width = w;
                                if w > 1.0 {
                                    glyph = '┃';
                                }
                            }
                            _ => {
                                return Err(format!(
                                    "unsupported gantt todayMarker declaration `{property}`"
                                ));
                            }
                        }
                    }
                    chart.today_marker = visible.then_some(glyph);
                }
                "click" => chart.notes.push(click_note(value)?),
                "displayMode" => {
                    return Err(format!("unsupported text gantt directive: `{line}`"));
                }
                _ => {
                    let (name, fields) = line
                        .split_once(':')
                        .ok_or_else(|| format!("unknown gantt statement: `{line}`"))?;
                    chart
                        .tasks
                        .push(task(name.trim(), without_comment(fields), &mut anonymous)?);
                }
            }
        }
        if chart.title.is_empty() {
            chart.title = label(acc_title);
        }
        for (which, entry) in calendar_entries {
            let set = if which == "includes" {
                &mut chart.calendar.includes
            } else {
                &mut chart.calendar.excludes
            };
            set.add(entry, weekend, &chart.dates, chart.today)?;
        }
        Ok(chart)
    }

    fn schedule(&mut self) -> Result<Schedule, String> {
        // Each task has two graph vertices. A known start may resolve an `until`
        // dependency before its own finish has been calculated.
        enum Boundary {
            Fixed(i64),
            After(Vec<usize>),
            Until(Vec<usize>),
            Duration(usize, Span),
        }
        let mut ids = FxHashMap::default();
        for (i, t) in self.tasks.iter().enumerate() {
            if ids.insert(t.id.as_str(), i).is_some() {
                return Err(format!("duplicate gantt task ID `{}`", t.id));
            }
        }
        for (targets, _) in &self.notes {
            for target in targets {
                if !ids.contains_key(target) {
                    return Err(format!("unknown gantt click task `{target}`"));
                }
            }
        }
        let references = |s: &str, key: &str, end: usize| -> Result<Option<Vec<usize>>, String> {
            let mut words = s.split_whitespace();
            if words.next() != Some(key) {
                return Ok(None);
            }
            let refs: Vec<_> = words
                .map(|id| {
                    ids.get(id)
                        .map(|n| 2 * n + end)
                        .ok_or_else(|| format!("unknown gantt dependency `{id}`"))
                })
                .collect::<Result<_, _>>()?;
            if refs.is_empty() {
                return Err(format!("`{key}` needs at least one task ID"));
            }
            Ok(Some(refs))
        };
        let mut nodes = Vec::with_capacity(2 * self.tasks.len());
        for (i, t) in self.tasks.iter().enumerate() {
            let start = match t.start {
                None if i == 0 => Boundary::Fixed(self.now - self.now.rem_euclid(DAY)),
                None => Boundary::After(vec![i * 2 - 1]),
                Some(s) => match references(s, "after", 1)? {
                    Some(r) => Boundary::After(r),
                    None => Boundary::Fixed(self.dates.parse(s, self.today)?),
                },
            };
            let end = if let Some(r) = references(t.end, "until", 0)? {
                Boundary::Until(r)
            } else if let Some(span) = Span::parse(t.end) {
                Boundary::Duration(2 * i, span)
            } else {
                match self.dates.parse(t.end, self.today) {
                    Ok(ms) => Boundary::Fixed(ms + if self.inclusive_end { DAY } else { 0 }),
                    Err(_) if t.end == "0" => Boundary::Duration(2 * i, Span::Millis(0)),
                    Err(e) => {
                        return Err(format!(
                            "task `{}`: expected a duration, end date or until; {e}",
                            t.label
                        ));
                    }
                }
            };
            nodes.extend([start, end]);
        }
        let mut downstream = vec![Vec::new(); nodes.len()];
        let mut pending = vec![0; nodes.len()];
        let mut ready = vec![];
        let mut dependencies = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            let deps: &[usize] = match node {
                Boundary::Fixed(_) => &[],
                Boundary::After(ds) => {
                    dependencies.extend(ds.iter().map(|&d| Dependency {
                        finish: d / 2,
                        start: i / 2,
                        until: false,
                    }));
                    ds
                }
                Boundary::Until(ds) => {
                    dependencies.extend(ds.iter().map(|&d| Dependency {
                        finish: i / 2,
                        start: d / 2,
                        until: true,
                    }));
                    ds
                }
                Boundary::Duration(d, _) => std::slice::from_ref(d),
            };
            pending[i] = deps.len();
            if deps.is_empty() {
                ready.push(i);
            }
            for &d in deps {
                downstream[d].push(i);
            }
        }
        let mut times = vec![0; nodes.len()];
        while let Some(i) = ready.pop() {
            times[i] = match &nodes[i] {
                Boundary::Fixed(ms) => {
                    datetime(*ms)?;
                    *ms
                }
                Boundary::After(ds) => self
                    .calendar
                    .next_work_time(ds.iter().map(|&d| times[d]).max().unwrap())?,
                Boundary::Until(ds) => ds.iter().map(|&d| times[d]).min().unwrap(),
                Boundary::Duration(d, span) => self.calendar.finish(times[*d], *span)?,
            };
            for &n in &downstream[i] {
                pending[n] -= 1;
                if pending[n] == 0 {
                    ready.push(n);
                }
            }
        }
        if let Some(i) = pending.iter().position(|&p| p != 0) {
            return Err(format!(
                "cyclic gantt dependency at `{}`",
                self.tasks[i / 2].id
            ));
        }
        for (t, [start, end]) in self.tasks.iter().zip(times.as_chunks::<2>().0) {
            if end < start {
                return Err(format!(
                    "gantt task `{}` finishes before it starts",
                    t.label
                ));
            }
        }
        Ok(Schedule {
            times: times.as_chunks::<2>().0.to_vec(),
            dependencies,
        })
    }
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &crate::Options) -> Result<String, String> {
        let mut chart = Chart::parse(source)?;
        let schedule = chart.schedule()?;
        plot::draw(&chart, &schedule.times, options)
    }
}
