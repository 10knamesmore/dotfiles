mod path;
mod size;

pub(crate) use size::ImageSize;
use size::layout_size;

use std::cell::RefCell;
use std::mem::size_of;

use cosmic_text::{
    Attrs, Buffer, Color as TextColor, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
    fontdb::{Family, Query},
};
use tiny_skia::{
    Color, ColorU8, FillRule, IntSize, Paint, Path, Pixmap, PremultipliedColorU8, Stroke, Transform,
};

use crate::{Options, color::Rgb};

const MAX_SCENE_BYTES: usize = 4 * 1024 * 1024;
const MAX_LABEL_BYTES: usize = 8 * 1024;
const MAX_METADATA_LINES: usize = 64;
const MAX_PATH_BYTES: usize = 512 * 1024;
const SEGMENT_BYTES: usize = size_of::<u8>() + 3 * size_of::<tiny_skia::Point>();
const TEXT_MARGIN: f32 = 16.0;
const WHITE: Rgb = Rgb::from_rgba(255, 255, 255, 255);
const BLACK: Rgb = Rgb::from_rgba(0, 0, 0, 255);
const DARK_PAPER: Rgb = Rgb::from_rgba(32, 32, 32, 255);

thread_local! {
    static FONTS: RefCell<FontSystem> = RefCell::new(load_fonts());
}

fn load_fonts() -> FontSystem {
    let mut fonts = FontSystem::new();
    for name in ["Arial", "Noto Sans", "DejaVu Sans", "Liberation Sans"] {
        if fonts
            .db()
            .query(&Query {
                families: &[Family::Name(name)],
                ..Query::default()
            })
            .is_some()
        {
            fonts.db_mut().set_sans_serif_family(name);
            break;
        }
    }
    fonts
}

enum Primitive {
    Path {
        path: Path,
        fill: Option<Rgb>,
        stroke: Option<Rgb>,
        width: f32,
    },
    Text {
        x: f32,
        y: f32,
        label: String,
        color: Rgb,
    },
}

pub(crate) struct Scene {
    monochrome: bool,
    size: Option<IntSize>,
    primitives: Vec<Primitive>,
    remaining: usize,
    error: Option<&'static str>,
    ink: Rgb,
    paper: Rgb,
    title_color: Rgb,
}

impl Scene {
    pub(crate) fn new(options: &Options) -> Result<Self, String> {
        if FONTS.with_borrow(|fonts| fonts.db().is_empty()) {
            return Err("PNG rendering requires a system font".into());
        }
        let theme = |key| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let backdrop = if options.color { DARK_PAPER } else { WHITE };
        let paper = theme("background").unwrap_or(backdrop);
        let ink = theme("textColor").unwrap_or_else(|| contrast_ink(paper, backdrop));
        Ok(Self {
            monochrome: !options.color,
            size: None,
            primitives: Vec::new(),
            remaining: MAX_SCENE_BYTES,
            error: None,
            ink,
            paper,
            title_color: theme("titleColor").unwrap_or(ink),
        })
    }

    pub(crate) fn measure(&self, label: &str) -> f32 {
        let line = label.lines().next().unwrap_or_default();
        if line.len() > MAX_LABEL_BYTES {
            return size::MAX_EDGE as f32;
        }
        FONTS.with_borrow_mut(|fonts| {
            let buffer = TextStyle::LABEL.shape(fonts, line);
            buffer.layout_runs().next().map_or(0.0, |run| run.line_w)
        })
    }

    pub(crate) fn text(&mut self, x: f32, y: f32, label: &str, color: Option<Rgb>) {
        if label.is_empty() || self.error.is_some() {
            return;
        }
        if label.len() > MAX_LABEL_BYTES {
            self.error = Some("PNG label exceeds 8 KiB");
        } else if !path::valid_coordinates(&[x, y]) {
            self.error = Some("invalid PNG text position");
        } else if self.reserve(label.len()) {
            self.primitives.push(Primitive::Text {
                x,
                y,
                label: label.into(),
                color: color.unwrap_or(self.ink),
            });
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "all primitive painters share canvas coordinates and independent fill/stroke"
    )]
    pub(crate) fn rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        fill: Option<Rgb>,
        stroke: Option<Rgb>,
    ) {
        if self.error.is_some() || !self.has_room(10 * SEGMENT_BYTES) {
            return;
        }
        if !path::valid_coordinates(&[x, y, x + w, y + h, radius]) {
            self.error = Some("invalid PNG rectangle coordinates");
            return;
        }
        if let Some(path) = path::rectangle(x, y, w, h, radius) {
            self.shape(path, fill, stroke, 1.5);
        }
    }

    pub(crate) fn path(&mut self, d: &str, fill: Option<Rgb>, stroke: Option<Rgb>, width: f32) {
        if self.error.is_some() || !self.has_room(0) {
            return;
        }
        if d.len() > MAX_PATH_BYTES {
            self.error = Some("PNG path exceeds 512 KiB");
            return;
        }
        if stroke.is_some() && (!path::valid_coordinates(&[width]) || width < 0.0) {
            self.error = Some("invalid PNG stroke width");
            return;
        }
        let segment_limit = (self.remaining - size_of::<Primitive>()) / SEGMENT_BYTES;
        match path::svg(d, segment_limit) {
            Ok(Some(path)) => self.shape(path, fill, stroke, width),
            Ok(None) => {}
            Err(err) => self.error = Some(err),
        }
    }

    fn shape(&mut self, path: Path, fill: Option<Rgb>, stroke: Option<Rgb>, width: f32) {
        if self.reserve(path.verbs().len() * SEGMENT_BYTES) {
            self.primitives.push(Primitive::Path {
                path,
                fill,
                stroke,
                width,
            });
        }
    }

    fn has_room(&mut self, bytes: usize) -> bool {
        if bytes + size_of::<Primitive>() > self.remaining {
            self.error = Some("PNG scene exceeds its drawing budget");
            return false;
        }
        true
    }

    fn reserve(&mut self, bytes: usize) -> bool {
        if !self.has_room(bytes) {
            return false;
        }
        self.remaining -= bytes + size_of::<Primitive>();
        true
    }

    pub(crate) fn resize(&mut self, width: f32, height: f32) -> Result<(), String> {
        self.size = Some(layout_size(width, height)?);
        Ok(())
    }

    pub(crate) fn ink(&self) -> Rgb {
        self.ink
    }

    pub(crate) fn paper(&self) -> Rgb {
        self.paper
    }

    pub(crate) fn set_title_color(&mut self, color: Option<Rgb>) {
        if let Some(color) = color {
            self.title_color = color;
        }
    }

    pub(crate) fn png(
        self,
        title: Option<&str>,
        description: Option<&str>,
        requested: ImageSize,
    ) -> Result<Vec<u8>, String> {
        if let Some(err) = self.error {
            return Err(err.into());
        }
        let base = self.size.ok_or("PNG diagram has no dimensions")?;
        FONTS.with_borrow_mut(|fonts| {
            let title = margin_text(fonts, title, TextStyle::TITLE, "title")?;
            let description = margin_text(fonts, description, TextStyle::LABEL, "description")?;
            let natural = layout_size(
                (base.width() as f32)
                    .max(title.width)
                    .max(description.width),
                base.height() as f32 + title.height + description.height,
            )?;
            let (size, viewport) = requested.viewport(natural)?;
            let bytes = size.width() as usize * size.height() as usize * 4;
            let mut pixels = Vec::new();
            pixels
                .try_reserve_exact(bytes)
                .map_err(|err| format!("allocating PNG canvas: {err}"))?;
            pixels.resize(bytes, 0);
            let mut image =
                Pixmap::from_vec(pixels, size).ok_or("invalid PNG canvas dimensions")?;
            image.fill(sk_color(self.paper));
            let mut cache = SwashCache::new();

            if let Some(buffer) = title.buffer {
                draw_text(
                    &mut image,
                    fonts,
                    &mut cache,
                    &buffer,
                    (TEXT_MARGIN, TEXT_MARGIN),
                    viewport,
                    self.title_color,
                );
            }
            let transform = viewport.pre_translate(0.0, title.height);
            for primitive in self.primitives {
                match primitive {
                    Primitive::Path {
                        path,
                        fill,
                        stroke,
                        width,
                    } => {
                        if let Some(color) = fill.filter(|color| !color.is_transparent()) {
                            image.fill_path(
                                &path,
                                &paint(color),
                                FillRule::Winding,
                                transform,
                                None,
                            );
                        }
                        if let Some(color) =
                            stroke.filter(|color| width > 0.0 && !color.is_transparent())
                        {
                            let stroke = Stroke {
                                width,
                                ..Stroke::default()
                            };
                            image.stroke_path(&path, &paint(color), &stroke, transform, None);
                        }
                    }
                    Primitive::Text { x, y, label, color } => {
                        let y = y + title.height;
                        if x < natural.width() as f32
                            && y < natural.height() as f32
                            && !color.is_transparent()
                        {
                            let buffer = TextStyle::LABEL.shape(fonts, &label);
                            draw_text(
                                &mut image,
                                fonts,
                                &mut cache,
                                &buffer,
                                (x, y),
                                viewport,
                                color,
                            );
                        }
                    }
                }
            }
            if let Some(buffer) = description.buffer {
                let y = base.height() as f32 + title.height + TEXT_MARGIN;
                draw_text(
                    &mut image,
                    fonts,
                    &mut cache,
                    &buffer,
                    (TEXT_MARGIN, y),
                    viewport,
                    self.ink,
                );
            }
            // System emoji fonts can supply pre-colored bitmaps even for a gray text run.
            if self.monochrome {
                for pixel in image.data_mut().as_chunks_mut::<4>().0 {
                    let gray = (54 * u16::from(pixel[0])
                        + 183 * u16::from(pixel[1])
                        + 19 * u16::from(pixel[2])
                        + 128)
                        >> 8;
                    pixel[..3].fill(gray as u8);
                }
            }
            image
                .encode_png()
                .map_err(|err| format!("encoding PNG: {err}"))
        })
    }
}

fn contrast_ink(paper: Rgb, backdrop: Rgb) -> Rgb {
    let [r, g, b, opacity] = paper.rgba().map(|channel| f32::from(channel) / 255.0);
    let [br, bg, bb, _] = backdrop.rgba().map(|channel| f32::from(channel) / 255.0);
    let linear = |fg: f32, bg: f32| {
        let value = fg * opacity + bg * (1.0 - opacity);
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let light = 0.2126 * linear(r, br) + 0.7152 * linear(g, bg) + 0.0722 * linear(b, bb);
    if light < 0.179 { WHITE } else { BLACK }
}

fn sk_color(color: Rgb) -> Color {
    let [r, g, b, a] = color.rgba();
    Color::from_rgba8(r, g, b, a)
}

fn paint(color: Rgb) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(sk_color(color));
    paint.anti_alias = true;
    paint
}

struct TextStyle {
    metrics: Metrics,
    weight: Weight,
}

impl TextStyle {
    const LABEL: Self = Self {
        metrics: Metrics::new(16.0, 20.0),
        weight: Weight::NORMAL,
    };
    const TITLE: Self = Self {
        metrics: Metrics::new(20.0, 26.0),
        weight: Weight::BOLD,
    };

    fn shape(self, fonts: &mut FontSystem, text: &str) -> Buffer {
        let mut buffer = Buffer::new_empty(self.metrics);
        buffer.set_wrap(Wrap::None);
        buffer.set_text(
            text,
            &Attrs::new().weight(self.weight),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(fonts, false);
        buffer
    }
}

#[derive(Default)]
struct MarginText {
    buffer: Option<Buffer>,
    width: f32,
    height: f32,
}

fn margin_text(
    fonts: &mut FontSystem,
    text: Option<&str>,
    style: TextStyle,
    kind: &str,
) -> Result<MarginText, String> {
    let Some(text) = text else {
        return Ok(MarginText::default());
    };
    if text.len() > MAX_LABEL_BYTES
        || text.lines().take(MAX_METADATA_LINES + 1).count() > MAX_METADATA_LINES
    {
        return Err(format!("PNG {kind} exceeds 8 KiB or 64 lines"));
    }
    if text.trim().is_empty() {
        return Ok(MarginText::default());
    }
    let line_height = style.metrics.line_height;
    let buffer = style.shape(fonts, text);
    let (width, height) = buffer.layout_runs().fold((0.0f32, 0.0f32), |(w, h), run| {
        (w.max(run.line_w), h.max(run.line_top + line_height))
    });
    Ok(MarginText {
        buffer: Some(buffer),
        width: width + 2.0 * TEXT_MARGIN,
        height: height + 2.0 * TEXT_MARGIN,
    })
}

fn draw_text(
    image: &mut Pixmap,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
    buffer: &Buffer,
    (x, y): (f32, f32),
    viewport: Transform,
    color: Rgb,
) {
    let [r, g, b, opacity] = color.rgba();
    if opacity == 0 {
        return;
    }
    let width = image.width() as i32;
    let height = image.height() as i32;
    let pixels = image.pixels_mut();
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let glyph = glyph.physical(
                (
                    x * viewport.sx + viewport.tx,
                    (y + run.line_y) * viewport.sy + viewport.ty,
                ),
                viewport.sx,
            );
            cache.with_pixels(
                fonts,
                glyph.cache_key,
                TextColor::rgb(r, g, b),
                |dx, dy, color| {
                    let (x, y) = (glyph.x + dx, glyph.y + dy);
                    if (0..width).contains(&x) && (0..height).contains(&y) {
                        let [r, g, b, coverage] = color.as_rgba();
                        let a = multiply_255(coverage, opacity);
                        let source = ColorU8::from_rgba(r, g, b, a).premultiply();
                        let pixel = &mut pixels[(y * width + x) as usize];
                        let over = |fg: u8, bg| fg.saturating_add(multiply_255(bg, 255 - a));
                        *pixel = PremultipliedColorU8::from_rgba(
                            over(source.red(), pixel.red()),
                            over(source.green(), pixel.green()),
                            over(source.blue(), pixel.blue()),
                            over(a, pixel.alpha()),
                        )
                        .unwrap();
                    }
                },
            );
        }
    }
}

fn multiply_255(a: u8, b: u8) -> u8 {
    ((u16::from(a) * u16::from(b) + 127) / 255) as u8
}
