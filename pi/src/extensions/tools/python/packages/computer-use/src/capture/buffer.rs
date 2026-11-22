//! Preserve compositor pixel words while exposing RGB values and an optional PNG view.

use std::io::Cursor;
use std::sync::Arc;

use image::{DynamicImage, ImageFormat, Rgb, RgbImage, RgbaImage, imageops};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use wayland_client::protocol::wl_shm::Format;

use super::Rect;
use crate::wayland::Frame;
use crate::{logging, wait};

/// An orientation-corrected region with unchanged compositor pixel words and bit depth.
/// Rows are tightly packed; output padding is removed. Created only by capture().
#[pyclass(module = "computer_use", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PixelBuffer {
    /// Four opaque bytes per pixel; RgbaImage is used only for lossless pixel reordering.
    pixels: Arc<RgbaImage>,

    /// Wayland's native-endian packed pixel format, retained through cropping.
    format: Format,
}

#[pymethods]
impl PixelBuffer {
    /// Return packed pixel bytes without image encoding or channel conversion.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.pixels.as_raw())
    }

    /// Region width and height in unscaled, orientation-corrected pixels.
    #[getter]
    pub(super) fn size(&self) -> (u32, u32) {
        self.pixels.dimensions()
    }

    /// Byte distance between rows; all supported formats occupy four bytes per pixel.
    #[getter]
    fn stride(&self) -> u32 {
        self.pixels.width() * 4
    }

    /// wl_shm format name; channel order describes a native-endian u32, not byte order.
    #[getter]
    fn format(&self) -> String {
        format!("{:?}", self.format).to_ascii_lowercase()
    }

    /// Number of bits in each RGB channel, independent of alpha or unused bits.
    #[getter]
    fn channel_bits(&self) -> u8 {
        match self.format {
            Format::Argb8888 | Format::Xrgb8888 | Format::Abgr8888 | Format::Xbgr8888 => 8,
            _ => 10,
        }
    }

    /// Return rows of (R, G, B) tuples at native bit depth, without scaling or PNG encoding.
    fn rgb(&self, py: Python<'_>) -> PyResult<Vec<Vec<(u16, u16, u16)>>> {
        let buffer = self.clone();
        logging::result(
            "capture.buffer.rgb",
            wait::compute(py, move || {
                Ok(buffer
                    .pixels
                    .rows()
                    .map(|row| row.map(|pixel| buffer.decode_rgb(pixel.0)).collect())
                    .collect())
            }),
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "PixelBuffer(size={:?}, format={:?}, stride={})",
            self.size(),
            self.format(),
            self.stride()
        )
    }
}

impl PixelBuffer {
    pub(super) fn from_frame(bytes: &[u8], frame: &Frame, transform: u32) -> PyResult<Self> {
        let format = frame
            .format
            .ok_or_else(|| wait::runtime("missing capture format"))?;
        if !matches!(
            format,
            Format::Argb8888
                | Format::Xrgb8888
                | Format::Abgr8888
                | Format::Xbgr8888
                | Format::Argb2101010
                | Format::Xrgb2101010
                | Format::Abgr2101010
                | Format::Xbgr2101010
        ) {
            return Err(wait::runtime(format!(
                "unsupported capture pixel format: {format:?}"
            )));
        }
        if (frame.stride as u64) < frame.width as u64 * 4 {
            return Err(wait::runtime(
                "capture stride is smaller than one pixel row",
            ));
        }
        // Treat the four-byte words as opaque pixels so imageops can reorder them
        // without quantizing 10-bit channels or overwriting alpha/unused bits.
        let mut pixels = RgbaImage::new(frame.width, frame.height);
        for (source, target) in bytes
            .chunks_exact(frame.stride as usize)
            .zip(pixels.as_mut().chunks_exact_mut(frame.width as usize * 4))
        {
            target.copy_from_slice(&source[..target.len()]);
        }
        if frame.y_invert {
            imageops::flip_vertical_in_place(&mut pixels);
        }
        // Wayland rotation is clockwise in image coordinates; reflection follows
        // rotation, as in grim's output-to-composite transformation.
        pixels = match transform & 3 {
            1 => imageops::rotate90(&pixels),
            2 => imageops::rotate180(&pixels),
            3 => imageops::rotate270(&pixels),
            _ => pixels,
        };
        if transform & 4 != 0 {
            imageops::flip_horizontal_in_place(&mut pixels);
        }
        Ok(Self {
            pixels: Arc::new(pixels),
            format,
        })
    }

    /// Crop in upright full-output pixels; capture() has already checked positive extents.
    pub(super) fn crop(self, (x, y, width, height): Rect) -> PyResult<Self> {
        let (full_width, full_height) = self.size();
        if x.checked_add(width)
            .is_none_or(|right| right > full_width as i64)
            || y.checked_add(height)
                .is_none_or(|bottom| bottom > full_height as i64)
        {
            return Err(wait::invalid(
                "rect is outside the full orientation-corrected capture",
            ));
        }
        if (x, y, width, height) == (0, 0, full_width as i64, full_height as i64) {
            return Ok(self);
        }
        Ok(Self {
            pixels: Arc::new(
                imageops::crop_imm(
                    self.pixels.as_ref(),
                    x as u32,
                    y as u32,
                    width as u32,
                    height as u32,
                )
                .to_image(),
            ),
            format: self.format,
        })
    }

    fn decode_rgb(&self, pixel: [u8; 4]) -> (u16, u16, u16) {
        let word = u32::from_ne_bytes(pixel);
        let (r, g, b) = match self.format {
            Format::Argb8888 | Format::Xrgb8888 => (word >> 16, word >> 8, word),
            Format::Abgr8888 | Format::Xbgr8888 => (word, word >> 8, word >> 16),
            Format::Argb2101010 | Format::Xrgb2101010 => (word >> 20, word >> 10, word),
            _ => (word, word >> 10, word >> 20),
        };
        let mask = (1 << self.channel_bits()) - 1;
        ((r & mask) as u16, (g & mask) as u16, (b & mask) as u16)
    }

    /// Encode an opaque 8-bit RGB preview; native words remain untouched in self.
    pub(super) fn png(&self, size: (u32, u32)) -> PyResult<Vec<u8>> {
        let maximum = (1_u32 << self.channel_bits()) - 1;
        let channel = |value: u16| ((value as u32 * 255 + maximum / 2) / maximum) as u8;
        let pixels = RgbImage::from_fn(self.pixels.width(), self.pixels.height(), |x, y| {
            let (r, g, b) = self.decode_rgb(self.pixels.get_pixel(x, y).0);
            Rgb([channel(r), channel(g), channel(b)])
        });
        let mut image = DynamicImage::ImageRgb8(pixels);
        if size != self.size() {
            image = image.resize_exact(size.0, size.1, imageops::FilterType::Lanczos3);
        }
        let mut png = Cursor::new(Vec::new());
        image
            .write_to(&mut png, ImageFormat::Png)
            .map_err(wait::runtime)?;
        Ok(png.into_inner())
    }
}
