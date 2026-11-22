use tiny_skia::{IntSize, Transform};

pub(super) const MAX_EDGE: u32 = 8192;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) enum ImageSize {
    Natural,
    Width(u16),
    Height(u16),
    Exact(u16, u16),
    Fit(u32, u32),
}

impl ImageSize {
    pub(super) fn viewport(self, natural: IntSize) -> Result<(IntSize, Transform), String> {
        let w = f64::from(natural.width());
        let h = f64::from(natural.height());
        let fit = |width: u32, height: u32| {
            let scale = (f64::from(width.min(MAX_EDGE)) / w)
                .min(f64::from(height.min(MAX_EDGE)) / h)
                .min((MAX_PIXELS as f64 / (w * h)).sqrt());
            ((w * scale).floor(), (h * scale).floor())
        };
        let (width, height) = match self {
            Self::Natural => fit(natural.width(), natural.height()),
            Self::Width(width) => (f64::from(width), h * f64::from(width) / w),
            Self::Height(height) => (w * f64::from(height) / h, f64::from(height)),
            Self::Exact(width, height) => (f64::from(width), f64::from(height)),
            Self::Fit(width, height) => fit(width, height),
        };
        let size = raster_size(
            width.round().max(1.0) as u32,
            height.round().max(1.0) as u32,
        )?;
        let (width, height) = (size.width() as f32, size.height() as f32);
        let (w, h) = (w as f32, h as f32);
        let scale = (width / w).min(height / h);
        let transform = Transform::from_row(
            scale,
            0.0,
            0.0,
            scale,
            (width - w * scale) / 2.0,
            (height - h * scale) / 2.0,
        );
        Ok((size, transform))
    }
}

pub(super) fn layout_size(width: f32, height: f32) -> Result<IntSize, String> {
    if width <= 0.0 || height <= 0.0 || !super::path::valid_coordinates(&[width, height]) {
        return Err("diagram layout exceeds the supported drawing coordinates".into());
    }
    let (w, h) = (width.ceil() as u32, height.ceil() as u32);
    IntSize::from_wh(w, h).ok_or_else(|| "invalid diagram layout dimensions".into())
}

fn raster_size(width: u32, height: u32) -> Result<IntSize, String> {
    if width > MAX_EDGE || height > MAX_EDGE {
        return Err("PNG dimensions must be positive and at most 8192 × 8192".into());
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("PNG exceeds the 16,777,216 pixel limit".into());
    }
    IntSize::from_wh(width, height).ok_or_else(|| "invalid PNG dimensions".into())
}
