use super::shared::sets::{Border, Card, body};
use crate::{Options, canvas::Canvas, color::Style, graphics::Scene};
use unicode_width::UnicodeWidthStr;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(draw_image);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (title, details) = parse(source, "Renderer: Rust text CLI")?;
        let width = (4 + details
            .iter()
            .map(|s| s.width())
            .max()
            .unwrap_or(0)
            .max(title.width()))
        .min(options.width.max(18))
        .max(18);
        let card = Card::new(
            &title,
            &details,
            width,
            Style {
                fill: None,
                ..style(options)
            },
        );
        let mut canvas = Canvas::new(width, card.height())?;
        card.draw(&mut canvas, 0, 0, width, Border::new(options))?;
        Ok(canvas.finish())
    }
}

fn style(options: &Options) -> Style {
    let primary = options.theme_style();
    Style {
        text: options
            .color
            .then(|| options.config.theme_color("titleColor"))
            .flatten()
            .or(primary.text),
        ..primary
    }
}

fn draw_image(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (title, details) = parse(source, "Renderer: Rust PNG CLI")?;
    let width = details
        .iter()
        .chain(std::iter::once(&title))
        .map(|text| scene.measure(text) + 28.0)
        .fold(220.0, f32::max)
        .min(580.0);
    let card =
        super::shared::sets::image::Card::new(scene, &title, &details, width, style(options));
    card.draw(scene, 18.0, 18.0);
    scene.resize(width + 36.0, card.height() + 36.0)
}

fn parse(source: &str, renderer: &str) -> Result<(String, Vec<String>), String> {
    let body = body(source, |header| header == "info")?;
    let mut show = false;
    let mut title = "Mermaid".to_owned();
    let mut details = vec![format!("mermaid-text {}", env!("CARGO_PKG_VERSION"))];
    let mut lines = body.lines().map(str::trim);
    while let Some(line) = lines.next() {
        let statement = line.split_once("%%").map_or(line, |(s, _)| s.trim_end());
        if statement.is_empty() {
            continue;
        }
        if statement == "showInfo" {
            show = true;
        } else if let Some(text) = line
            .strip_prefix("title ")
            .or_else(|| line.strip_prefix("title:"))
        {
            title = text.trim().to_owned();
        } else if let Some(text) = line
            .strip_prefix("accTitle:")
            .or_else(|| line.strip_prefix("accDescr:"))
        {
            details.push(text.trim().to_owned());
        } else if let Some(mut rest) = line
            .strip_prefix("accDescr")
            .and_then(|s| s.trim_start().strip_prefix('{'))
        {
            loop {
                if let Some((last, tail)) = rest.split_once('}') {
                    if !tail.trim().is_empty() {
                        return Err(format!("unexpected text after accDescr: {tail}"));
                    }
                    if !last.trim().is_empty() {
                        details.push(last.trim().to_owned());
                    }
                    break;
                }
                if !rest.trim().is_empty() {
                    details.push(rest.trim().to_owned());
                }
                rest = lines.next().ok_or("unterminated info accDescr block")?;
            }
        } else {
            return Err(format!("unknown info statement: {line}"));
        }
    }
    if show {
        details.push(renderer.into());
    }
    Ok((title, details))
}
