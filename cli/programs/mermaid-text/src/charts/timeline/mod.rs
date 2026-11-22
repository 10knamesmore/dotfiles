mod image;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::Options;
use crate::color::{Layer, Paint, Rgb, Style};

struct Timeline<'a> {
    title: Option<&'a str>,
    direction: Direction,
    periods: Vec<Period<'a>>,
    sections: Vec<Section<'a>>,
}

enum Direction {
    LeftToRight,
    TopDown,
}

struct Section<'a> {
    label: &'a str,
    periods: Vec<Period<'a>>,
}

struct Period<'a> {
    label: &'a str,
    events: Vec<&'a str>,
}

struct Glyphs {
    section: &'static str,
    period: &'static str,
    spine: &'static str,
    branch: &'static str,
    end: &'static str,
    forward: &'static str,
}

struct CardLine {
    text: String,
    label_column: usize,
}

#[derive(Default)]
struct Output {
    text: String,
    row: usize,
    layer: Layer,
}

impl Output {
    fn newline(&mut self) {
        self.text.push('\n');
        self.row += 1;
    }

    fn append(&mut self, x: usize, line: &CardLine, style: Style) {
        let width = line.text.width();
        self.stroke(x, line.label_column, style.stroke);
        self.layer.paint(
            x + line.label_column,
            self.row,
            width.saturating_sub(line.label_column),
            style.ink(),
        );
        self.text.push_str(&line.text);
    }

    fn stroke(&mut self, x: usize, width: usize, fg: Option<Rgb>) {
        self.layer.paint(x, self.row, width, Paint { fg, bg: None });
    }

    fn card(&mut self, lines: &[CardLine], style: Style) {
        for line in lines {
            self.append(0, line, style);
            self.newline();
        }
    }
}

struct Palette {
    slots: Vec<Style>,
}

impl Palette {
    fn new(options: &Options) -> Self {
        if !options.color {
            return Self {
                slots: vec![Style::default()],
            };
        }
        let count = options
            .config
            .theme_value("THEME_COLOR_LIMIT")
            .and_then(|v| v.as_u64())
            .filter(|n| (1..=64).contains(n))
            .unwrap_or(12);
        let base = options.theme_style();
        let line = options.theme_line();
        let label = options.config.theme_color("scaleLabelColor");
        let slots = (0..count)
            .map(|i| {
                let color = |key| options.config.theme_color(&format!("{key}{i}"));
                base.overlay(Style {
                    fill: color("cScale"),
                    text: color("cScaleLabel").or(label),
                    stroke: color("cScaleInv").or(line),
                })
            })
            .collect();
        Self { slots }
    }

    fn slot(&self, index: usize) -> Style {
        self.slots[index % self.slots.len()]
    }
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &crate::Options) -> Result<String, String> {
        let timeline = parse(source)?;
        let glyphs = if options.ascii {
            Glyphs {
                section: ">",
                period: "o",
                spine: "|",
                branch: "|-",
                end: "`-",
                forward: " > ",
            }
        } else {
            Glyphs {
                section: "◆",
                period: "●",
                spine: "│",
                branch: "├─",
                end: "└─",
                forward: " → ",
            }
        };
        let mut output = Output::default();
        let palette = Palette::new(options);
        if let Some(title) = timeline.title {
            let mut rows = Vec::new();
            label_rows(&mut rows, title, "", "", options.width);
            output.card(&rows, Style::default());
            output.newline();
        }
        if timeline.periods.is_empty() && timeline.sections.is_empty() {
            output.text.push_str("No periods");
            return Ok(output.text);
        }
        match timeline.direction {
            Direction::LeftToRight => {
                draw_horizontal(
                    &mut output,
                    &timeline.periods,
                    &glyphs,
                    options.width,
                    &|i| palette.slot(i),
                );
                if !timeline.periods.is_empty() && !timeline.sections.is_empty() {
                    output.newline();
                }
                for (index, section) in timeline.sections.iter().enumerate() {
                    if index > 0 {
                        output.newline();
                    }
                    let style = palette.slot(index);
                    let mut rows = Vec::new();
                    label_rows(
                        &mut rows,
                        section.label,
                        &format!("{}  ", glyphs.section),
                        "   ",
                        options.width,
                    );
                    output.card(&rows, style);
                    draw_horizontal(
                        &mut output,
                        &section.periods,
                        &glyphs,
                        options.width,
                        &|_| style,
                    );
                }
            }
            Direction::TopDown => {
                draw_vertical(
                    &mut output,
                    &timeline.periods,
                    "",
                    !timeline.sections.is_empty(),
                    &glyphs,
                    options.width,
                    &|i| palette.slot(i),
                );
                for (index, section) in timeline.sections.iter().enumerate() {
                    let more_sections = index + 1 < timeline.sections.len();
                    let indent = format!("{}  ", if more_sections { glyphs.spine } else { " " });
                    let style = palette.slot(index);
                    let mut rows = Vec::new();
                    label_rows(
                        &mut rows,
                        section.label,
                        &format!("{}  ", glyphs.section),
                        &indent,
                        options.width,
                    );
                    output.card(&rows, style);
                    draw_vertical(
                        &mut output,
                        &section.periods,
                        &indent,
                        false,
                        &glyphs,
                        options.width,
                        &|_| style,
                    );
                    if more_sections {
                        output.stroke(0, 1, style.stroke);
                        output.text.push_str(glyphs.spine);
                        output.newline();
                    }
                }
            }
        }
        output.text.pop();
        Ok(output.layer.apply(&output.text))
    }
}

fn draw_horizontal(
    output: &mut Output,
    periods: &[Period<'_>],
    glyphs: &Glyphs,
    width: usize,
    style: &impl Fn(usize) -> Style,
) {
    const GAP: usize = 3;
    let per_band = ((width + GAP) / (18 + GAP)).clamp(1, 4);
    for (index, band) in periods.chunks(per_band).enumerate() {
        if index > 0 {
            output.newline();
        }
        let preferred = if band.len() == 1 {
            width
        } else {
            ((width + GAP) / band.len() - GAP).min(28)
        };
        let cards: Vec<_> = band
            .iter()
            .map(|period| period_rows(period, "", false, glyphs, preferred))
            .collect();
        let height = cards.iter().map(Vec::len).max().unwrap_or(0);
        let columns: Vec<_> = cards
            .iter()
            .map(|card| card.iter().map(|line| line.text.width()).max().unwrap_or(0))
            .collect();
        for row in 0..height {
            let start = output.text.len();
            let mut x = 0;
            for (column, card) in cards.iter().enumerate() {
                let style = style(index * per_band + column);
                let mut width = 0;
                if let Some(line) = card.get(row) {
                    output.append(x, line, style);
                    width = line.text.width();
                }
                if column + 1 < cards.len() {
                    output
                        .text
                        .extend(std::iter::repeat_n(' ', columns[column] - width));
                    x += columns[column];
                    if row == 0 {
                        output.stroke(x + 1, 1, style.stroke);
                    }
                    output
                        .text
                        .push_str(if row == 0 { glyphs.forward } else { "   " });
                    x += GAP;
                }
            }
            output
                .text
                .truncate(start + output.text[start..].trim_end().len());
            output.newline();
        }
    }
}

fn draw_vertical(
    output: &mut Output,
    periods: &[Period<'_>],
    indent: &str,
    followed_by_section: bool,
    glyphs: &Glyphs,
    width: usize,
    style: &impl Fn(usize) -> Style,
) {
    for (index, period) in periods.iter().enumerate() {
        let has_next = index + 1 < periods.len() || followed_by_section;
        let style = style(index);
        output.card(&period_rows(period, indent, has_next, glyphs, width), style);
        if has_next {
            output.stroke(0, indent.width() + 1, style.stroke);
            output.text.push_str(indent);
            output.text.push_str(glyphs.spine);
            output.newline();
        }
    }
}

fn period_rows(
    period: &Period<'_>,
    indent: &str,
    has_next: bool,
    glyphs: &Glyphs,
    width: usize,
) -> Vec<CardLine> {
    let mut rows = Vec::new();
    let rail = if has_next { glyphs.spine } else { " " };
    let continuation = format!("{indent}{rail}  ");
    label_rows(
        &mut rows,
        period.label,
        &format!("{indent}{}  ", glyphs.period),
        &continuation,
        width,
    );
    for (index, event) in period.events.iter().enumerate() {
        let has_next_event = index + 1 < period.events.len();
        let branch = if has_next_event {
            glyphs.branch
        } else {
            glyphs.end
        };
        let spine = if has_next_event { glyphs.spine } else { " " };
        label_rows(
            &mut rows,
            event,
            &format!("{continuation}{branch} "),
            &format!("{continuation}{spine}  "),
            width,
        );
    }
    rows
}

fn parse(source: &str) -> Result<Timeline<'_>, String> {
    let mut lines = source.lines().enumerate().filter_map(|(index, line)| {
        let line = line.trim();
        (!line.is_empty() && !line.starts_with("%%") && !line.starts_with('#'))
            .then_some((index + 1, line))
    });
    let (header_line, header) = lines.next().ok_or("expected a timeline header")?;
    let suffix = header
        .strip_prefix("timeline")
        .filter(|rest| rest.is_empty() || rest.starts_with([' ', '\t']))
        .ok_or("expected a timeline header")?
        .trim_start();
    let (word, rest) = suffix.split_once([' ', '\t']).unwrap_or((suffix, ""));
    let (direction, first_statement) = if word.eq_ignore_ascii_case("TD") {
        (Direction::TopDown, rest.trim_start())
    } else if word.eq_ignore_ascii_case("LR") {
        (Direction::LeftToRight, rest.trim_start())
    } else {
        (Direction::LeftToRight, suffix)
    };
    let first_statement = (!first_statement.is_empty()
        && !first_statement.starts_with("%%")
        && !first_statement.starts_with('#'))
    .then_some((header_line, first_statement));

    let mut timeline = Timeline {
        title: None,
        direction,
        periods: Vec::new(),
        sections: Vec::new(),
    };
    for (line_number, line) in first_statement.into_iter().chain(lines) {
        let error = |message: &str| format!("timeline line {line_number}: {message}");
        let (keyword, rest) = line.split_once([' ', '\t']).unwrap_or((line, ""));
        let tail = rest.trim();
        let metadata = line.split([' ', '\t', ':', '{']).next().unwrap_or(line);
        if keyword.eq_ignore_ascii_case("title") {
            if tail.is_empty() {
                return Err(error("expected text after title"));
            }
            if timeline.title.replace(tail).is_some() {
                return Err(error("duplicate title"));
            }
        } else if keyword.eq_ignore_ascii_case("section") {
            if tail.is_empty() || tail.contains(':') {
                return Err(error("expected a section name without ':'"));
            }
            timeline.sections.push(Section {
                label: tail,
                periods: Vec::new(),
            });
        } else if metadata.eq_ignore_ascii_case("accTitle")
            || metadata.eq_ignore_ascii_case("accDescr")
        {
            return Err(error("accessibility metadata is not supported"));
        } else if line.starts_with(':') {
            let periods = timeline
                .sections
                .last_mut()
                .map_or(&mut timeline.periods, |section| &mut section.periods);
            let Some(period) = periods.last_mut() else {
                return Err(error(
                    "event continuation requires a preceding period in the same section",
                ));
            };
            parse_events(line, &mut period.events).map_err(error)?;
        } else {
            let (label, suffix) = line
                .find(':')
                .map_or((line, ""), |start| line.split_at(start));
            let label = label.trim_end();
            if label.contains('#') {
                return Err(error("'#' inside a period label is not supported"));
            }
            let mut events = Vec::new();
            if !suffix.is_empty() {
                parse_events(suffix, &mut events).map_err(error)?;
            }
            timeline
                .sections
                .last_mut()
                .map_or(&mut timeline.periods, |section| &mut section.periods)
                .push(Period { label, events });
        }
    }
    Ok(timeline)
}

fn parse_events<'a>(text: &'a str, events: &mut Vec<&'a str>) -> Result<(), &'static str> {
    let body = text.strip_prefix(':').unwrap();
    if !body.starts_with([' ', '\t']) {
        return Err("expected ': ' followed by an event");
    }
    // After an event starts, colons without following whitespace belong to its text.
    let mut start = 0;
    for (index, _) in body.match_indices(':') {
        if body[index + 1..].starts_with([' ', '\t']) {
            let event = body[start..index].trim();
            if event.is_empty() {
                return Err("expected an event after ':'");
            }
            events.push(event);
            start = index + 1;
        }
    }
    let event = body[start..].trim();
    if event.is_empty() {
        return Err("expected an event after ':'");
    }
    events.push(event);
    Ok(())
}

fn label_rows(
    rows: &mut Vec<CardLine>,
    label: &str,
    first: &str,
    continuation: &str,
    width: usize,
) {
    let columns = width.saturating_sub(first.width()).max(12);
    let mut prefix = first;
    let mut write_line = |line: &str| {
        let lead = if line.is_empty() {
            prefix.trim_end()
        } else {
            prefix
        };
        rows.push(CardLine {
            text: format!("{lead}{line}"),
            label_column: lead.width(),
        });
        prefix = continuation;
    };
    let label = label.replace('\t', " ");
    for part in label_parts(&label) {
        wrap_line(part, columns, &mut write_line);
    }
}

fn label_parts(label: &str) -> impl Iterator<Item = &str> {
    let mut remainder = Some(label);
    std::iter::from_fn(move || {
        let rest = remainder.take()?;
        for (index, _) in rest.match_indices('<') {
            let suffix = &rest[index..];
            if let Some(tag) = ["<br>", "<br/>", "<br />"].into_iter().find(|tag| {
                suffix
                    .get(..tag.len())
                    .is_some_and(|head| head.eq_ignore_ascii_case(tag))
            }) {
                let part = rest[..index].trim();
                remainder = Some(&suffix[tag.len()..]);
                return Some(part);
            }
        }
        Some(rest.trim())
    })
}

fn wrap_line(text: &str, limit: usize, emit: &mut impl FnMut(&str)) {
    if text.width() <= limit {
        emit(text);
        return;
    }

    let mut line = String::new();
    let mut columns = 0;
    for word in text.split_word_bounds() {
        let word_width = word.width();
        if columns + word_width > limit && !line.is_empty() {
            emit(line.trim_end());
            line.clear();
            columns = 0;
        }
        if line.is_empty() && word.trim().is_empty() {
            continue;
        }
        if word_width <= limit {
            line.push_str(word);
            columns += word_width;
            continue;
        }
        for grapheme in word.graphemes(true) {
            let grapheme_width = grapheme.width();
            if columns + grapheme_width > limit && !line.is_empty() {
                emit(&line);
                line.clear();
                columns = 0;
            }
            line.push_str(grapheme);
            columns += grapheme_width;
        }
    }
    if !line.is_empty() {
        emit(line.trim_end());
    }
}
