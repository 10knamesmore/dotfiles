mod image;

use super::shared::history::{self, Strokes, quoted, width};
use crate::Options;
use crate::color::{Paint, with_base};
use crate::text::wrap;
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);
    const TITLE_COLORS: &'static [&'static str] = &["packet.titleColor", "titleColor"];

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (rest, lines) = history::header(source, &["packet", "packet-beta"])?;
        let mut document = history::parse(rest, lines)?;
        let drawing = render(&document.statements, options)?;
        if !document.title.is_empty()
            && options.color
            && let Some(fg) = options.config.theme_color("packet.titleColor")
        {
            document.title = with_base(
                &document.title,
                Paint {
                    fg: Some(fg),
                    bg: None,
                },
            )?
            .into();
        }
        Ok(document.finish(&drawing))
    }
}

const MAX_BITS: usize = 131_072;

struct Field {
    start: usize,
    end: usize, // exclusive
    name: String,
}

fn number(s: &str) -> Result<usize, String> {
    let s = s.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid bit number '{s}'"));
    }
    s.parse()
        .map_err(|_| format!("bit number out of range: {s}"))
}

fn field(s: &str, next: usize) -> Result<Field, String> {
    let (range, text) = s
        .split_once(':')
        .ok_or("expected bit range: \"field name\"")?;
    let range = range.trim();
    let (start, end) = if let Some(count) = range.strip_prefix('+') {
        (next, next.checked_add(number(count)?))
    } else {
        let (a, b) = range.split_once('-').unwrap_or((range, range));
        (number(a)?, number(b)?.checked_add(1))
    };
    let end = end.ok_or("bit range overflow")?;
    if start != next {
        return Err(format!("field starts at bit {start}; expected bit {next}"));
    }
    if end <= start {
        return Err("field must contain at least one bit".into());
    }
    if end > MAX_BITS {
        return Err(format!("packet exceeds {MAX_BITS} bits"));
    }
    let (name, rest) = quoted(text.trim())?;
    if !rest.is_empty() {
        return Err(format!("unexpected text after field name: {rest}"));
    }
    Ok(Field { start, end, name })
}

struct Cell {
    left: usize,
    right: usize,
    lines: Vec<String>,
}

fn fields(statements: &[(usize, String)]) -> Result<(Vec<Field>, usize), String> {
    let mut fields = Vec::new();
    let mut bits = 0;
    for (n, s) in statements {
        let f = field(s, bits).map_err(|e| format!("packet line {n}: {e}"))?;
        bits = f.end;
        fields.push(f);
    }
    Ok((fields, bits))
}

fn bits_per_row(options: &Options) -> usize {
    options
        .config
        .packet
        .bits_per_row
        .map_or(32, |width| usize::from(width.get()))
}

fn render(statements: &[(usize, String)], options: &Options) -> Result<String, String> {
    let (fields, bits) = fields(statements)?;
    let theme = |key| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let border = theme("packet.blockStrokeColor");
    let fill = Paint {
        fg: None,
        bg: theme("packet.blockFillColor"),
    };
    let label = Paint {
        fg: theme("packet.labelColor"),
        bg: None,
    };
    let start_bit = Paint {
        fg: theme("packet.startByteColor"),
        bg: None,
    };
    let end_bit = Paint {
        fg: theme("packet.endByteColor"),
        bg: None,
    };
    let natural = if options.ascii {
        ['+'; 4]
    } else {
        ['┌', '┐', '└', '┘']
    };
    let corners = options.corners.resolve(options.ascii, natural);
    if fields.is_empty() {
        let [tl, tr, bl, br] = corners;
        return with_base(
            &format!("{tl}{tr}\n{bl}{br}"),
            Paint {
                fg: border,
                bg: None,
            },
        );
    }
    let row_bits = bits_per_row(options);
    let word_bits = bits.min(row_bits);
    let gutter = ((bits - 1) / row_bits * row_bits).to_string().len().max(3) + 2;
    let mut pitch = (options.width.saturating_sub(gutter + 1) / word_bits).clamp(1, 8);
    let mut words: Vec<Vec<(usize, usize, &str)>> =
        (0..bits.div_ceil(row_bits)).map(|_| Vec::new()).collect();
    for f in &fields {
        let widest = f.name.graphemes(true).map(width).max().unwrap_or(0);
        let mut bit = f.start;
        while bit < f.end {
            let row = bit / row_bits;
            let end = f.end.min((row + 1) * row_bits);
            let span = end - bit;
            pitch = pitch.max((widest + 1).div_ceil(span));
            words[row].push((bit % row_bits, span, &f.name));
            bit = end;
        }
    }
    let columns = gutter + word_bits * pitch + 1;
    let mut y = 1;
    let mut rows = Vec::new();
    let mut label_bytes = 0;
    for word in words {
        let mut cells = Vec::new();
        let mut height = 1;
        for (offset, span, name) in word {
            label_bytes += name.len();
            if label_bytes > 16 * 1024 * 1024 {
                return Err(
                    "packet labels exceed 16 MiB after splitting fields across rows".into(),
                );
            }
            let left = gutter + offset * pitch;
            let right = left + span * pitch;
            let lines = wrap(name, right - left - 1);
            height = height.max(lines.len());
            cells.push(Cell { left, right, lines });
        }
        rows.push((y, height, cells));
        y += 1 + height;
        if (y + 1).saturating_mul(columns) > 4 * 1024 * 1024 {
            return Err("packet exceeds four million layout cells".into());
        }
    }
    let mut strokes = Strokes::new(columns, y + 1)?;
    for (y, h, cells) in &rows {
        let bottom = y + h + 1;
        for c in cells {
            strokes.horizontal(c.left, c.right, *y, border);
            strokes.horizontal(c.left, c.right, bottom, border);
            strokes.vertical(c.left, *y, bottom, border);
            strokes.vertical(c.right, *y, bottom, border);
        }
    }
    let mut canvas = strokes.draw_with_corners(options.ascii, '┼', corners)?;
    canvas.text(0, 0, "bit")?;
    canvas.paint(0, 0, 3, start_bit);
    let mut last_end = 0;
    for bit in 0..word_bits {
        if bit % 4 == 0 || bit + 1 == word_bits {
            let number = bit.to_string();
            let x = (gutter + bit * pitch).min(columns - number.len());
            if x >= last_end {
                canvas.text(x, 0, &number)?;
                let paint = if bit != 0 && bit + 1 == word_bits {
                    end_bit
                } else {
                    start_bit
                };
                canvas.paint(x, 0, number.len(), paint);
                last_end = x + number.len() + 1;
            }
        }
    }
    for (i, (y, h, cells)) in rows.iter().enumerate() {
        let offset = (i * row_bits).to_string();
        let (x, row) = (gutter - 2 - offset.len(), y + 1 + (h - 1) / 2);
        canvas.text(x, row, &offset)?;
        canvas.paint(x, row, offset.len(), start_bit);
        for c in cells {
            let inner = c.right - c.left - 1;
            if fill.bg.is_some() {
                for row in (y + 1)..=(y + h) {
                    canvas.paint(c.left + 1, row, inner, fill);
                }
            }
            let top = y + 1 + (h - c.lines.len()) / 2;
            for (k, line) in c.lines.iter().enumerate() {
                let columns = width(line);
                let x = c.left + 1 + (inner - columns) / 2;
                canvas.text(x, top + k, line)?;
                canvas.paint(x, top + k, columns, label);
            }
        }
    }
    Ok(canvas.finish())
}
