use super::{Captions, theme_color};
use crate::{Options, color::Rgb, graphics::Scene};

pub const PAD: f32 = 20.0;
pub const LINE: f32 = 20.0;

pub struct Label {
    text: String,
    widths: Vec<f32>,
    pub width: f32,
    pub height: f32,
}

impl Label {
    pub fn new(scene: &Scene, text: &str) -> Self {
        let text = text.replace('\t', "    ");
        let widths: Vec<_> = text.split('\n').map(|line| scene.measure(line)).collect();
        Self {
            height: widths.len() as f32 * LINE,
            width: widths.iter().copied().fold(0.0, f32::max),
            text,
            widths,
        }
    }

    pub fn draw(&self, scene: &mut Scene, x: f32, y: f32, color: Option<Rgb>) {
        for (row, line) in self.text.split('\n').enumerate() {
            scene.text(x, y + row as f32 * LINE, line, color);
        }
    }

    pub fn center(&self, scene: &mut Scene, x: f32, y: f32, color: Option<Rgb>) {
        for (row, (line, width)) in self.text.split('\n').zip(&self.widths).enumerate() {
            scene.text(x - width / 2.0, y + row as f32 * LINE, line, color);
        }
    }
}

pub fn tint(color: Rgb, opacity: u8) -> Rgb {
    let [r, g, b, alpha] = color.rgba();
    Rgb::from_rgba(r, g, b, (u16::from(alpha) * u16::from(opacity) / 255) as u8)
}

pub fn text_on_fill(scene: &Scene, options: &Options, fill: Option<Rgb>) -> Option<Rgb> {
    theme_color(options, "textColor").or_else(|| fill.map(|fill| contrast_ink(scene, fill)))
}

fn contrast_ink(scene: &Scene, fill: Rgb) -> Rgb {
    let [r, g, b, a] = fill.rgba();
    let paper = scene.paper().rgba();
    let luminance = [r, g, b]
        .into_iter()
        .zip(paper)
        .zip([0.2126, 0.7152, 0.0722])
        .map(|((fg, bg), weight)| {
            let a = f32::from(a) / 255.0;
            let value = (f32::from(fg) * a + f32::from(bg) * (1.0 - a)) / 255.0;
            weight
                * if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
        })
        .sum::<f32>();
    let level = if luminance < 0.179 { 255 } else { 0 };
    Rgb::from_rgba(level, level, level, 255)
}

pub struct Caption {
    labels: Vec<(Label, Option<Rgb>)>,
    pub width: f32,
    pub height: f32,
}

impl Caption {
    pub fn new(scene: &Scene, captions: &Captions, options: &Options) -> Self {
        let title_paint = theme_color(options, "titleColor");
        let mut labels = Vec::new();
        for (text, paint) in [
            (&captions.title, title_paint),
            (&captions.accessible_title, title_paint),
            (&captions.description, None),
        ] {
            if !text.is_empty() {
                labels.push((Label::new(scene, text), paint));
            }
        }
        Self {
            width: labels
                .iter()
                .map(|(label, _)| label.width)
                .fold(0.0, f32::max),
            height: labels.iter().map(|(label, _)| label.height + 8.0).sum(),
            labels,
        }
    }

    pub fn draw(&self, scene: &mut Scene) {
        let mut y = PAD;
        for (label, paint) in &self.labels {
            label.draw(scene, PAD, y, *paint);
            y += label.height + 8.0;
        }
    }
}
