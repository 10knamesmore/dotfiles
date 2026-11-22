use std::collections::BTreeSet;
use std::f64::consts::TAU;

use unicode_width::UnicodeWidthStr;

use crate::Options;
use crate::color::{Layer, Paint, Rgb};

mod image;

struct Pie<'a> {
    title: Option<&'a str>,
    accessible_title: Option<&'a str>,
    description: Option<&'a str>,
    show_data: bool,
    slices: Vec<Slice<'a>>,
}

impl Pie<'_> {
    fn max_value(&self) -> f64 {
        self.slices
            .iter()
            .map(|slice| slice.value)
            .fold(0.0, f64::max)
    }
}

struct Slice<'a> {
    label: String,
    raw_value: &'a str,
    value: f64,
}

struct PieColors {
    slices: Vec<Option<Rgb>>,
    stroke: Option<Rgb>,
    outer_stroke: Option<Rgb>,
    title: Option<Rgb>,
    legend: Option<Rgb>,
    section_text: Option<Rgb>,
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);
    const TITLE_COLORS: &'static [&'static str] = &["pieTitleTextColor"];

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let pie = parse(source)?;
        let transparent = Rgb::parse("transparent");
        let opaque = |color: Option<Rgb>| color.filter(|&color| Some(color) != transparent);
        let theme = |key: &str| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let stroke = theme("pieStrokeColor");
        let colors = PieColors {
            slices: (1..=pie.slices.len())
                .map(|i| opaque(theme(&format!("pie{i}"))))
                .collect(),
            stroke: opaque(stroke),
            outer_stroke: opaque(theme("pieOuterStrokeColor").or(stroke)),
            title: theme("pieTitleTextColor"),
            legend: theme("pieLegendTextColor"),
            section_text: theme("pieSectionTextColor"),
        };
        let max_value = pie.max_value();
        let mut sectors = Vec::new();
        let mut markers = vec![String::new(); pie.slices.len()];
        let positive_colors = pie
            .slices
            .iter()
            .zip(&colors.slices)
            .filter_map(|(slice, &color)| (slice.value > 0.0).then_some(color));
        let solid = distinguishable(positive_colors.clone());
        let positive_count = positive_colors.count();
        let palette = if options.ascii {
            "#+.@xo%=*~:;"
        } else {
            "█░▒▓╳◇╱▚○╲▞◆"
        };
        let marker_width = if solid || positive_count <= palette.chars().count() {
            1
        } else {
            (positive_count - 1).max(1).ilog(36) as usize + 1
        };
        let mut total = 0.0;
        for (index, slice) in pie.slices.iter().enumerate() {
            if slice.value > 0.0 {
                total += slice.value / max_value;
                markers[index] = if solid {
                    if options.ascii { "#" } else { "█" }.to_owned()
                } else if positive_count <= palette.chars().count() {
                    palette.chars().nth(sectors.len()).unwrap().to_string()
                } else {
                    marker_code(sectors.len(), marker_width)
                };
                sectors.push((total, index));
            }
        }

        let chart_columns = if pie.slices.is_empty() {
            20
        } else {
            options.width.clamp(20, 36) / 4 * 4
        };
        let (chart, visible, mut plot_colors) = draw(
            &sectors,
            &markers,
            marker_width,
            chart_columns,
            options.ascii,
            solid,
            &colors,
        );
        let swatch_width = if solid { 3 } else { marker_width };
        let legend = legend(&pie, &markers, &visible, swatch_width, max_value, total);
        let legend_columns = legend
            .iter()
            .map(|line| line.text.width())
            .max()
            .unwrap_or(0);
        let beside = !legend.is_empty()
            && legend.len() <= chart.len()
            && chart_columns + 3 + legend_columns <= options.width;
        let span = if beside {
            chart_columns + 3 + legend_columns
        } else {
            chart_columns
        };

        let mut heading = String::new();
        let mut heading_colors = Layer::default();
        let titles = [
            pie.title,
            pie.accessible_title
                .filter(|&title| Some(title) != pie.title),
        ];
        for (row, title) in titles
            .into_iter()
            .flatten()
            .filter(|title| !title.is_empty())
            .enumerate()
        {
            let title = title.replace('\t', " ");
            let x = span.saturating_sub(title.width()) / 2;
            heading.push_str(&" ".repeat(x));
            heading.push_str(&title);
            heading.push('\n');
            heading_colors.paint(
                x,
                row,
                title.width(),
                Paint {
                    fg: colors.title,
                    bg: None,
                },
            );
        }
        if !heading.is_empty() {
            heading.push('\n');
        }
        let mut plot = String::new();
        let legend_top = chart.len().saturating_sub(legend.len()) / 2;
        for (row, line) in chart.iter().enumerate() {
            plot.push_str(line);
            if beside && let Some(entry) = row.checked_sub(legend_top).and_then(|i| legend.get(i)) {
                plot.push_str(&" ".repeat(chart_columns + 3 - line.width()));
                plot.push_str(&entry.text);
                entry.paint(
                    &mut plot_colors,
                    chart_columns + 3,
                    row,
                    swatch_width,
                    &colors,
                );
            }
            plot.push('\n');
        }
        if pie.slices.is_empty() {
            let label = "No slices";
            plot.push_str(&" ".repeat((chart_columns - label.width()) / 2));
            plot.push_str(label);
            plot.push('\n');
        } else if !beside {
            plot.push('\n');
            for (row, line) in legend.iter().enumerate() {
                plot.push_str(&line.text);
                plot.push('\n');
                line.paint(
                    &mut plot_colors,
                    0,
                    chart.len() + 1 + row,
                    swatch_width,
                    &colors,
                );
            }
        }
        plot.pop();
        let mut result = heading_colors.apply(&heading);
        result.push_str(&plot_colors.apply(&plot));
        if let Some(description) = pie
            .description
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            result.push_str("\n\n");
            result.push_str(&crate::text::wrap(description, options.width.max(12)).join("\n"));
        }
        Ok(result)
    }
}

fn parse(source: &str) -> Result<Pie<'_>, String> {
    let mut input = Input {
        rest: source,
        line: 1,
    };
    input.skip_layout();
    if !input.keyword("pie") {
        return Err("expected a pie header".into());
    }
    input.skip_horizontal();
    let mut pie = Pie {
        title: None,
        accessible_title: None,
        description: None,
        show_data: input.keyword("showData"),
        slices: Vec::new(),
    };
    let mut labels = BTreeSet::new();
    loop {
        input.skip_layout();
        if input.rest.is_empty() {
            break;
        }
        let line = input.line;
        let error = |message: &str| format!("pie line {line}: {message}");
        if input.keyword("title") {
            if pie.title.replace(input.line_content()).is_some() {
                return Err(error("duplicate title"));
            }
        } else if let Some(tail) = input.rest.strip_prefix("accTitle") {
            input.rest = tail
                .trim_start_matches([' ', '\t'])
                .strip_prefix(':')
                .ok_or_else(|| error("expected ':' after accTitle"))?;
            pie.accessible_title = Some(input.line_content());
        } else if let Some(tail) = input.rest.strip_prefix("accDescr") {
            pie.description = Some(input.description(tail).map_err(error)?);
        } else if input.rest.starts_with(['"', '\'']) {
            let label = input.quoted_label().map_err(error)?;
            input.skip_horizontal();
            if !input.rest.starts_with(':') {
                return Err(format!(
                    "pie line {}: expected ':' after slice label",
                    input.line
                ));
            }
            input.advance(1);
            let raw_value = input.line_content();
            let value = parse_value(raw_value)
                .map_err(|message| format!("pie line {}: {message}", input.line))?;
            if !labels.insert(label.clone()) {
                return Err(error("duplicate slice label"));
            }
            pie.slices.push(Slice {
                label,
                raw_value,
                value,
            });
        } else {
            return Err(error(
                "expected a title or a quoted slice label followed by ':' and a value",
            ));
        }
    }
    Ok(pie)
}

struct Input<'a> {
    rest: &'a str,
    line: usize,
}

impl<'a> Input<'a> {
    fn advance(&mut self, bytes: usize) {
        let (consumed, rest) = self.rest.split_at(bytes);
        self.line += consumed.bytes().filter(|&byte| byte == b'\n').count();
        self.rest = rest;
    }

    fn skip_horizontal(&mut self) {
        self.rest = self.rest.trim_start_matches([' ', '\t']);
    }

    fn skip_layout(&mut self) {
        loop {
            self.advance(
                self.rest.len() - self.rest.trim_start_matches([' ', '\t', '\r', '\n']).len(),
            );
            if !self.rest.starts_with("%%") {
                break;
            }
            self.line_content();
        }
    }

    fn keyword(&mut self, keyword: &str) -> bool {
        if let Some(rest) = self.rest.strip_prefix(keyword)
            && (rest.is_empty() || rest.starts_with([' ', '\t', '\r', '\n']))
        {
            self.rest = rest;
            return true;
        }
        false
    }

    fn line_content(&mut self) -> &'a str {
        let (line, rest) = self
            .rest
            .split_at(self.rest.find('\n').unwrap_or(self.rest.len()));
        self.rest = rest;
        line.split_once("%%").map_or(line, |(text, _)| text).trim()
    }

    fn description(&mut self, tail: &'a str) -> Result<&'a str, &'static str> {
        if let Some(rest) = tail.trim_start_matches([' ', '\t']).strip_prefix(':') {
            self.rest = rest;
            return Ok(self.line_content());
        }
        let block = tail
            .trim_start_matches([' ', '\t', '\r', '\n'])
            .strip_prefix('{')
            .ok_or("expected ':' or '{' after accDescr")?;
        let (description, rest) = block.split_once('}').ok_or("unclosed accDescr block")?;
        self.advance(self.rest.len() - rest.len());
        if !self.line_content().is_empty() {
            return Err("unexpected text after accDescr block");
        }
        Ok(description)
    }

    fn quoted_label(&mut self) -> Result<String, &'static str> {
        let quote = self.rest.as_bytes()[0] as char;
        let mut label = String::new();
        let mut chars = self.rest[1..].char_indices();
        while let Some((index, character)) = chars.next() {
            if character == quote {
                self.advance(index + 2);
                return Ok(label);
            }
            let character = if character == '\\' {
                match chars.next().map(|(_, escaped)| escaped) {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('b' | 'f' | 'r' | 'v' | '0' | '\n' | '\r') => {
                        return Err("unsupported control character escape in slice label");
                    }
                    Some(escaped) => escaped,
                    None => break,
                }
            } else {
                character
            };
            if character.is_control() && !matches!(character, '\t' | '\n') {
                return Err("unexpected control character in slice label");
            }
            label.push(character);
        }
        Err("unclosed quoted slice label")
    }
}

fn parse_value(raw: &str) -> Result<f64, &'static str> {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    let all_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let valid = match digits.split_once('.') {
        Some((whole, fraction)) => all_digits(whole) && all_digits(fraction),
        None => all_digits(digits) && (digits == "0" || !digits.starts_with('0')),
    };
    if !valid {
        return Err("expected a non-negative integer or decimal value");
    }
    let nonzero = digits.bytes().any(|byte| matches!(byte, b'1'..=b'9'));
    if raw.starts_with('-') && nonzero {
        return Err("slice values must be non-negative");
    }
    let value: f64 = raw.parse().map_err(|_| "invalid slice value")?;
    if !value.is_finite() || (value == 0.0 && nonzero) {
        return Err("slice value is outside the representable numeric range");
    }
    Ok(value)
}

fn marker_code(mut index: usize, width: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut code = vec![b'0'; width];
    for digit in code.iter_mut().rev() {
        *digit = DIGITS[index % DIGITS.len()];
        index /= DIGITS.len();
    }
    String::from_utf8(code).unwrap()
}

fn distinguishable(colors: impl Iterator<Item = Option<Rgb>>) -> bool {
    let mut swatches: Vec<[i32; 3]> = Vec::new();
    for color in colors {
        let Some(color) = color else {
            return false;
        };
        let hex = color.hex();
        let rgb = [1, 3, 5].map(|i| i32::from_str_radix(&hex[i..i + 2], 16).unwrap());
        // Keep textures when adjacent terminal fills could be hard to tell apart.
        if swatches.iter().any(|other| {
            rgb.iter()
                .zip(other)
                .map(|(a, b)| (a - b).pow(2))
                .sum::<i32>()
                < 48 * 48
        }) {
            return false;
        }
        swatches.push(rgb);
    }
    !swatches.is_empty()
}

fn radius_squared(x: f64, y: f64) -> f64 {
    x * x + y * y
}

fn outline_cell(x: f64, y: f64, dx: f64, dy: f64, ascii: bool) -> char {
    if ascii {
        return if (0.77..=1.0).contains(&radius_squared(x, y)) {
            '#'
        } else {
            ' '
        };
    }
    let mut dots = 0;
    for (j, bits) in [[1, 8], [2, 16], [4, 32], [64, 128]]
        .into_iter()
        .enumerate()
    {
        for (i, bit) in bits.into_iter().enumerate() {
            let sx = x + (i as f64 - 0.5) * dx / 2.0;
            let sy = y + (j as f64 - 1.5) * dy / 4.0;
            if (0.88..=1.0).contains(&radius_squared(sx, sy)) {
                dots |= bit;
            }
        }
    }
    if dots == 0 {
        ' '
    } else {
        char::from_u32(0x2800 + dots).unwrap()
    }
}

fn filled_cell(x: f64, y: f64, dx: f64, dy: f64) -> Option<(char, f64, f64)> {
    const QUADRANTS: [char; 16] = [
        ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
    ];
    let mut mask = 0usize;
    let (mut sx, mut sy) = (0.0, 0.0);
    for (i, (ox, oy)) in [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)]
        .into_iter()
        .enumerate()
    {
        let (qx, qy) = (x + ox * dx, y + oy * dy);
        if radius_squared(qx, qy) <= 1.0 {
            mask |= 1 << i;
            sx += qx;
            sy += qy;
        }
    }
    if mask == 0 {
        return None;
    }
    let count = mask.count_ones();
    let (nx, ny) = ((x * dx).abs(), (y * dy).abs());
    let shape = if matches!(count, 1 | 3)
        && nx.max(ny) <= nx.min(ny) * 2.0
        && (radius_squared(x, y) - 1.0).abs() < nx.min(ny) * 0.8
    {
        // Where the circumference crosses the middle diagonally, join both cell corners.
        match (x < 0.0, y < 0.0) {
            (true, true) => '◢',
            (true, false) => '◥',
            (false, true) => '◣',
            (false, false) => '◤',
        }
    } else {
        QUADRANTS[mask]
    };
    // Use the part inside the circle to pick the color of a partial cell.
    Some((shape, sx / f64::from(count), sy / f64::from(count)))
}

fn draw(
    sectors: &[(f64, usize)],
    markers: &[String],
    marker_width: usize,
    columns: usize,
    ascii: bool,
    solid: bool,
    colors: &PieColors,
) -> (Vec<String>, Vec<bool>, Layer) {
    let cells = columns / marker_width;
    let rows = columns * 2 / 5;
    let mut chart = Vec::with_capacity(rows);
    let mut visible = vec![false; markers.len()];
    let mut layer = Layer::default();
    let blank = " ".repeat(marker_width);
    let total = sectors.last().map_or(0.0, |&(end, _)| end);

    // Approximate a circle in cells with a 2:5 width-to-height ratio.
    let x_at = |cell: f64| 2.0 * cell / cells as f64 - 1.0;
    let y_at = |row: f64| 2.0 * row / rows as f64 - 1.0;
    let dx = 2.0 / cells as f64;
    let dy = 2.0 / rows as f64;
    let sector_at = |x: f64, y: f64| {
        // atan2(x, -y) starts at twelve o'clock and increases clockwise.
        let angle = (x.atan2(-y) / TAU).rem_euclid(1.0);
        let sector = sectors.partition_point(|&(end, _)| end <= angle * total);
        sectors
            .get(sector)
            .or_else(|| sectors.last())
            .map(|&(_, i)| i)
    };
    for row in 0..rows {
        let mut line = String::new();
        for cell in 0..cells {
            let x = x_at(cell as f64 + 0.5);
            let y = y_at(row as f64 + 0.5);
            let paint = |layer: &mut Layer, fg| {
                layer.paint(
                    cell * marker_width,
                    row,
                    marker_width,
                    Paint { fg, bg: None },
                );
            };
            if total == 0.0 {
                let outline = outline_cell(x, y, dx, dy, ascii);
                if outline != ' ' {
                    paint(&mut layer, colors.outer_stroke);
                }
                line.push(outline);
                continue;
            }

            let (shape, x, y, edge) = if solid && !ascii {
                let Some((shape, x, y)) = filled_cell(x, y, dx, dy) else {
                    line.push(' ');
                    continue;
                };
                (Some(shape), x, y, shape != '█')
            } else {
                if radius_squared(x, y) > 1.0 {
                    line.push_str(&blank);
                    continue;
                }
                let edge = radius_squared(x + dx.copysign(x), y) > 1.0
                    || radius_squared(x, y + dy.copysign(y)) > 1.0;
                (None, x, y, edge)
            };

            let index = sector_at(x, y).unwrap();
            let (next_x, next_y) = if x.abs() * dy < y.abs() * dx {
                (x + dx, y)
            } else {
                (x, y + dy)
            };
            let mut divider = false;
            let fg = if edge && colors.outer_stroke.is_some() {
                colors.outer_stroke
            } else if colors.stroke.is_some()
                && !edge
                && radius_squared(next_x, next_y) <= 1.0
                && sector_at(next_x, next_y) != Some(index)
            {
                divider = true;
                colors.stroke
            } else {
                colors.slices[index]
            };
            paint(&mut layer, fg);
            if solid && divider {
                line.push(if ascii { '.' } else { '·' });
            } else if let Some(shape) = shape {
                line.push(shape);
            } else {
                line.push_str(&markers[index]);
            }
            visible[index] |= !solid || !divider && fg == colors.slices[index];
        }
        chart.push(line.trim_end().to_owned());
    }
    (chart, visible, layer)
}

struct LegendLine {
    text: String,
    marker: Option<(usize, usize)>,
}

impl LegendLine {
    fn paint(
        &self,
        layer: &mut Layer,
        x: usize,
        y: usize,
        marker_width: usize,
        colors: &PieColors,
    ) {
        let start = x + marker_width + 2;
        layer.paint(
            start,
            y,
            self.text.width().saturating_sub(marker_width + 2),
            Paint {
                fg: colors.legend,
                bg: None,
            },
        );
        if let Some((index, width)) = self.marker {
            layer.paint(
                x + marker_width - width,
                y,
                width,
                Paint {
                    fg: colors.slices[index],
                    bg: None,
                },
            );
            layer.paint(
                start,
                y,
                6,
                Paint {
                    fg: colors.section_text,
                    bg: None,
                },
            );
        }
    }
}

fn legend(
    pie: &Pie<'_>,
    markers: &[String],
    visible: &[bool],
    marker_width: usize,
    max_value: f64,
    total: f64,
) -> Vec<LegendLine> {
    let mut lines = Vec::new();
    for (index, slice) in pie.slices.iter().enumerate() {
        let percent = percentage(slice.value, max_value, total);
        let marker = if slice.value == 0.0 {
            "-".to_owned()
        } else {
            markers[index].repeat(marker_width / markers[index].width())
        };
        let label = slice.label.replace('\t', " ");
        let mut label_lines = label.split('\n');
        lines.push(LegendLine {
            text: format!(
                "{marker:>marker_width$}  {percent:>6}  {}",
                label_lines.next().unwrap_or_default()
            ),
            marker: Some((index, if slice.value > 0.0 { marker.width() } else { 0 })),
        });
        for line in label_lines {
            lines.push(LegendLine {
                text: format!("{}{line}", " ".repeat(marker_width + 10)),
                marker: None,
            });
        }
        let line = &mut lines.last_mut().unwrap().text;
        if pie.show_data {
            line.push_str(&format!(" [{}]", slice.raw_value));
        }
        if slice.value > 0.0 && !visible[index] {
            line.push_str(" (no visible cell)");
        }
    }
    lines
}

fn percentage(value: f64, max_value: f64, total: f64) -> String {
    if total == 0.0 {
        "n/a".to_owned()
    } else if value == 0.0 {
        "0%".to_owned()
    } else {
        let percent = (value / max_value) / total * 100.0;
        if percent < 0.01 {
            "<0.01%".to_owned()
        } else {
            let percent = format!("{percent:.2}");
            format!("{}%", percent.trim_end_matches('0').trim_end_matches('.'))
        }
    }
}
