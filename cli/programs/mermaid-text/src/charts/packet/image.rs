use super::{bits_per_row, fields, history};
use crate::{
    Options,
    charts::shared::history::{IMAGE_LINE, IMAGE_PAD, ImageLabel},
    graphics::Scene,
};

const CELL_PAD: f32 = 12.0;
const ROW_GAP: f32 = 12.0;
const MIN_PITCH: f32 = 25.0;
const MIN_HEIGHT: f32 = 2.0 * IMAGE_LINE + 2.0 * CELL_PAD;

struct Segment<'a> {
    text: &'a str,
    name: ImageLabel,
    start: usize,
    end: usize,
    start_label: String,
    end_label: String,
    continues_from: bool,
    continues_to: bool,
}

impl Segment<'_> {
    fn span(&self) -> usize {
        self.end - self.start
    }

    fn height(&self) -> f32 {
        self.name.height().max(IMAGE_LINE) + IMAGE_LINE + 2.0 * CELL_PAD
    }
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (rest, lines) = history::header(source, &["packet", "packet-beta"])?;
    let document = history::parse(rest, lines)?;
    let (fields, bits) = fields(&document.statements)?;
    let (title, description) = document.image_metadata(scene);
    let theme = |key| {
        options
            .color
            .then(|| options.config.theme_color(key))
            .flatten()
    };
    let border = theme("packet.blockStrokeColor").or(Some(scene.ink()));
    let fill = theme("packet.blockFillColor").or(options.theme_style().fill);
    let label = theme("packet.labelColor");
    let start_color = theme("packet.startByteColor");
    let end_color = theme("packet.endByteColor");

    let row_bits = bits_per_row(options);
    let word_bits = bits.min(row_bits) as f32;
    let row_count = bits.div_ceil(row_bits);
    let gutter = scene
        .measure("Offset")
        .max(scene.measure(&((row_count.saturating_sub(1) * row_bits).to_string())))
        + 16.0;
    let left = IMAGE_PAD + gutter;
    let top = IMAGE_PAD + title.height() + if title.height() > 0.0 { 14.0 } else { 0.0 };
    let footer = description.height() + IMAGE_PAD;
    scene.resize(
        left + word_bits * MIN_PITCH + IMAGE_PAD,
        top + row_count as f32 * (MIN_HEIGHT + ROW_GAP) + footer,
    )?;

    let mut pitch = MIN_PITCH;
    let mut words: Vec<Vec<Segment<'_>>> = (0..row_count).map(|_| Vec::new()).collect();
    for field in &fields {
        let mut start = field.start;
        while start < field.end {
            let row = start / row_bits;
            let end = field.end.min((row + 1) * row_bits);
            let span = end - start;
            let segment = Segment {
                text: &field.name,
                name: ImageLabel::wrapped(scene, &field.name, (3 * span).clamp(8, 36)),
                start,
                end,
                start_label: start.to_string(),
                end_label: if span > 1 {
                    (end - 1).to_string()
                } else {
                    String::new()
                },
                continues_from: start > field.start,
                continues_to: end < field.end,
            };
            let offset_width = scene.measure(&segment.start_label)
                + scene.measure(&segment.end_label)
                + if span > 1 { 12.0 } else { 0.0 };
            pitch =
                pitch.max((segment.name.width.max(offset_width) + 2.0 * CELL_PAD) / span as f32);
            words[row].push(segment);
            start = end;
        }
    }
    for segment in words.iter_mut().flatten() {
        if segment.name.height() == IMAGE_LINE {
            continue;
        }
        let available = pitch * segment.span() as f32 - 2.0 * CELL_PAD;
        let columns = ((available / 8.0) as usize).min(100);
        if columns > (3 * segment.span()).clamp(8, 36) {
            let wider = ImageLabel::wrapped(scene, segment.text, columns);
            if wider.width <= available && wider.height() < segment.name.height() {
                segment.name = wider;
            }
        }
    }
    let row_heights: Vec<_> = words
        .iter()
        .map(|word| word.iter().map(Segment::height).fold(MIN_HEIGHT, f32::max))
        .collect();
    let body_height = if words.is_empty() {
        IMAGE_LINE
    } else {
        IMAGE_LINE + row_heights.iter().sum::<f32>() + words.len() as f32 * ROW_GAP
    };
    let width = (gutter + word_bits * pitch)
        .max(title.width)
        .max(description.width)
        + 2.0 * IMAGE_PAD;
    scene.resize(width, top + body_height + footer)?;
    title.draw(
        scene,
        IMAGE_PAD,
        IMAGE_PAD,
        theme("packet.titleColor").or_else(|| theme("titleColor")),
    );
    if words.is_empty() {
        scene.text(IMAGE_PAD, top, "No fields", None);
    } else {
        scene.text(IMAGE_PAD, top, "Offset", start_color);
    }
    let mut y = top + IMAGE_LINE;
    for (row, (word, height)) in words.iter().zip(row_heights).enumerate() {
        let base = row * row_bits;
        let offset = base.to_string();
        scene.text(
            left - 16.0 - scene.measure(&offset),
            y + (height - IMAGE_LINE) / 2.0,
            &offset,
            start_color,
        );
        for segment in word {
            let x = left + (segment.start - base) as f32 * pitch;
            let w = segment.span() as f32 * pitch;
            scene.rect(x, y, w, height, 0.0, fill, border);
            let middle = x + w / 2.0;
            let start_x = if segment.span() == 1 {
                middle - scene.measure(&segment.start_label) / 2.0
            } else {
                x + CELL_PAD
            };
            scene.text(start_x, y + 5.0, &segment.start_label, start_color);
            scene.text(
                x + w - CELL_PAD - scene.measure(&segment.end_label),
                y + 5.0,
                &segment.end_label,
                end_color,
            );
            segment.name.center(
                scene,
                middle,
                y + IMAGE_LINE + (height - IMAGE_LINE - segment.name.height()) / 2.0,
                label,
            );
            for (edge, continued) in [
                (x + 3.0, segment.continues_from),
                (x + w - 10.0, segment.continues_to),
            ] {
                if continued {
                    let center_y = y + IMAGE_LINE + (height - IMAGE_LINE) / 2.0;
                    scene.path(
                        &format!("M {edge} {} l 7 5 -7 5", center_y - 5.0),
                        None,
                        border,
                        1.6,
                    );
                }
            }
        }
        y += height + ROW_GAP;
    }
    description.draw(scene, IMAGE_PAD, top + body_height, None);
    Ok(())
}
