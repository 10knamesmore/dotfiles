use super::{DOMAINS, Diagram, Domain, Region};
use crate::{
    Options,
    canvas::Canvas,
    color::{Paint, with_base},
    text::wrap,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

struct Line {
    marker: &'static str,
    text: String,
}

impl Line {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            marker: "",
            text: text.into(),
        }
    }
}

fn content(domain: &Domain, region: &Region, ascii: bool) -> Vec<Line> {
    let bullet = if ascii { "* " } else { "• " };
    let arrow = if ascii { "-> " } else { "→ " };
    let mut lines = vec![Line::plain(domain.heading)];
    if !domain.decisions.is_empty() {
        lines.push(Line::plain(domain.decisions.join(if ascii {
            " -> "
        } else {
            " → "
        })));
    }
    lines.push(Line::plain(domain.practice));
    if !region.items.is_empty() {
        lines.push(Line::plain(""));
        for item in &region.items {
            lines.push(Line {
                marker: bullet,
                text: item.clone(),
            });
        }
    }
    if !region.outgoing.is_empty() {
        lines.push(Line::plain(""));
        for (to, label) in &region.outgoing {
            let text = match label {
                Some(label) => format!("{}: {label}", to.heading),
                None => to.heading.to_owned(),
            };
            lines.push(Line {
                marker: arrow,
                text,
            });
        }
    }
    lines
}

fn width(s: &str) -> usize {
    s.graphemes(true).map(UnicodeWidthStr::width).sum()
}

fn natural_width(lines: &[Line]) -> usize {
    lines
        .iter()
        .map(|l| width(l.marker) + width(&l.text))
        .max()
        .unwrap_or(0)
}

struct PanelRow {
    text: String,
    heading: bool,
    arrow_width: usize,
}

fn rows(lines: &[Line], columns: usize) -> Vec<PanelRow> {
    let mut rows = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let padding = " ".repeat(width(line.marker));
        for (i, text) in wrap(&line.text, columns - padding.len())
            .into_iter()
            .enumerate()
        {
            let prefix = if i == 0 { line.marker } else { &padding };
            rows.push(PanelRow {
                text: format!("{prefix}{text}"),
                heading: index == 0,
                arrow_width: if i == 0 && matches!(line.marker, "→ " | "-> ") {
                    width(line.marker.trim_end())
                } else {
                    0
                },
            });
        }
    }
    rows
}

pub(super) fn draw(diagram: &Diagram<'_>, options: &Options) -> Result<String, String> {
    let panels: [_; 5] =
        std::array::from_fn(|i| content(&DOMAINS[i], &diagram.regions[i], options.ascii));
    let columns = panels
        .iter()
        .map(|lines| natural_width(lines))
        .max()
        .unwrap()
        .min(options.width.saturating_sub(7) / 2)
        .max(13);
    let board_width = 2 * columns + 7;
    let center_columns = natural_width(&panels[4]).min(columns);
    let [complex, complicated, chaotic, clear, confusion] =
        std::array::from_fn(|i| rows(&panels[i], if i == 4 { center_columns } else { columns }));
    let middle = board_width / 2;
    let center_left = middle - (center_columns + 3).div_ceil(2);
    let center_right = board_width - 1 - center_left;
    let center_top = 1 + complex.len().max(complicated.len());
    let center_bottom = center_top + confusion.len() + 1;
    let divider = (center_bottom + center_top) / 2;
    let bottom = center_bottom + 1 + chaotic.len().max(clear.len());
    let right = board_width - 1;
    let mut c = Canvas::new(board_width, bottom + 1)?;
    let color = |key: &str| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let fg = |fg| Paint { fg, bg: None };
    let boundary = fg(color("cynefin.boundaryColor").or(options.theme_line()));
    let cliff = fg(color("cynefin.cliffColor"));
    let heading = fg(color("cynefin.labelColor").or(options.theme_style().text));
    let text = fg(color("cynefin.textColor"));
    let arrow = fg(color("cynefin.arrowColor").or(options.theme_line()));
    let fills = DOMAINS.map(|d| Paint {
        fg: None,
        bg: color(&format!("cynefin.{}Bg", d.keyword)),
    });
    for y in 1..bottom {
        let lower = usize::from(y > divider) * 2;
        let center = (center_top..=center_bottom).contains(&y);
        let (left_edge, right_edge) = if center {
            (center_left, center_right)
        } else {
            (middle, middle)
        };
        if y != divider {
            c.paint(1, y, left_edge - 1, fills[lower]);
            c.paint(right_edge + 1, y, right - right_edge - 1, fills[lower + 1]);
        }
        if center_top < y && y < center_bottom {
            c.paint(center_left + 1, y, center_right - center_left - 1, fills[4]);
        }
    }
    let mut border = |x, y, ch| {
        let glyph = match (options.ascii, ch) {
            (false, _) => ch,
            (true, '─') => '-',
            (true, '│' | '║') => '|',
            (true, _) => '+',
        };
        let paint = if x == middle && y >= center_bottom {
            cliff
        } else {
            boundary
        };
        c.paint(x, y, 1, paint);
        c.put(x, y, glyph)
    };
    for y in 0..=bottom {
        border(0, y, '│')?;
        border(right, y, '│')?;
        if y < center_top {
            border(middle, y, '│')?;
        } else if y > center_bottom {
            border(middle, y, '║')?;
        } else {
            border(center_left, y, '│')?;
            border(center_right, y, '│')?;
        }
    }
    for x in 0..=right {
        border(x, 0, '─')?;
        border(x, bottom, '─')?;
        if (center_left..=center_right).contains(&x) {
            border(x, center_top, '─')?;
            border(x, center_bottom, '─')?;
        } else {
            border(x, divider, '─')?;
        }
    }
    for (left, right, top, bottom, natural) in [
        (0, right, 0, bottom, ['┌', '┐', '└', '┘']),
        (
            center_left,
            center_right,
            center_top,
            center_bottom,
            ['╭', '╮', '╰', '╯'],
        ),
    ] {
        let [tl, tr, bl, br] = options.corners.resolve(options.ascii, natural);
        for (x, y, ch) in [
            (left, top, tl),
            (right, top, tr),
            (left, bottom, bl),
            (right, bottom, br),
        ] {
            border(x, y, ch)?;
        }
    }
    for (x, y, ch) in [
        (middle, 0, '┬'),
        (middle, bottom, '╨'),
        (0, divider, '├'),
        (right, divider, '┤'),
        (center_left, divider, '┤'),
        (center_right, divider, '├'),
        (middle, center_top, '┴'),
        (middle, center_bottom, '╥'),
    ] {
        border(x, y, ch)?;
    }
    for (x, y, lines) in [
        (2, 1, complex),
        (middle + 2, 1, complicated),
        (center_left + 2, center_top + 1, confusion),
        (2, center_bottom + 1, chaotic),
        (middle + 2, center_bottom + 1, clear),
    ] {
        for (i, line) in lines.iter().enumerate() {
            c.text(x, y + i, &line.text)?;
            c.paint(
                x + line.arrow_width,
                y + i,
                width(&line.text) - line.arrow_width,
                if line.heading { heading } else { text },
            );
            c.paint(x, y + i, line.arrow_width, arrow);
        }
    }
    let mut output = Vec::new();
    for title in [diagram.title, diagram.accessible_title] {
        if !title.is_empty() {
            for line in wrap(title, board_width) {
                output.push(format!(
                    "{}{}",
                    " ".repeat(board_width.saturating_sub(width(&line)) / 2),
                    with_base(&line, heading)?
                ));
            }
        }
    }
    output.push(c.finish());
    for line in &diagram.description {
        if !line.is_empty() {
            output.extend(wrap(line, board_width));
        }
    }
    Ok(output.join("\n"))
}
