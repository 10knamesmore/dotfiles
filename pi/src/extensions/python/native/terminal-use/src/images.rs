//! Kitty-graphics source images and placements captured for a screen snapshot.
//!
//! The terminal core stores fully decoded, uncompressed pixels; this module copies those pixels
//! out while the terminal is not mutated and encodes them as PNG for Python. It exposes only the
//! source image and its structured placements; it does not render pixels or resolve Unicode
//! placeholder positions, which the core does not expose.

use std::collections::BTreeMap;
use std::io::Cursor;

use libghostty_vt::Terminal;
use libghostty_vt::alloc::{Allocator, Bytes};
use libghostty_vt::kitty::graphics::{
    self, DecodePng, DecodedImage, ImageFormat, PlacementIterator,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pythonize::pythonize;
use serde::Serialize;

/// One source image together with the placements on the active screen that reference it.
#[derive(Debug)]
pub(crate) struct ImageSnapshot {
    /// Kitty image identifier; unique while the image is stored.
    pub(crate) image_id: u32,

    /// Source image width in pixels.
    pub(crate) width: u32,

    /// Source image height in pixels.
    pub(crate) height: u32,

    /// Pixel layout of `pixels`.
    pub(crate) format: ImagePixelFormat,

    /// Owned copy of the complete decoded source image.
    pub(crate) pixels: Vec<u8>,

    /// Placements referencing this image, ordered by placement ID.
    pub(crate) placements: Vec<PlacementSnapshot>,
}

/// Pixel layout of a stored Kitty image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImagePixelFormat {
    Rgb,
    Rgba,
    Gray,
    GrayAlpha,
}

/// Where and how one placement draws its image on the active screen.
///
/// All coordinates are in source image pixels or terminal cells; no viewport clipping is applied
/// to `source_rect`.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct PlacementSnapshot {
    /// Kitty placement identifier, unique per image.
    pub(crate) placement_id: u32,

    /// Whether the placement is a Unicode placeholder. Virtual placements have no resolved
    /// viewport position because the binding does not expose resolved placeholder locations.
    pub(crate) is_virtual: bool,

    /// Requested placement width in cells; 0 means the core derives it from the image.
    pub(crate) cell_columns: u32,

    /// Requested placement height in cells; 0 means the core derives it from the image.
    pub(crate) cell_rows: u32,

    /// Pixel offset of the image from the top-left of its anchor cell.
    pub(crate) pixel_offset: PixelOffset,

    /// Source rectangle in the image, resolved for protocol defaults and clamped to the image.
    pub(crate) source_rect: SourceRectSnapshot,

    /// Z-index used to order the placement against cell backgrounds and text.
    pub(crate) z_index: i32,

    /// Top-left position in viewport cells when the placement is visible and not virtual.
    pub(crate) viewport_position: Option<ViewportPosition>,
}

/// Pixel offset of a placement from its anchor cell.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct PixelOffset {
    /// Distance from the anchor cell's left edge in pixels.
    pub(crate) x: u32,

    /// Distance from the anchor cell's top edge in pixels.
    pub(crate) y: u32,
}

/// Resolved source rectangle of a placement in image pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct SourceRectSnapshot {
    /// Left edge in the source image.
    pub(crate) x: u32,

    /// Top edge in the source image.
    pub(crate) y: u32,

    /// Cropped width in source pixels.
    pub(crate) width: u32,

    /// Cropped height in source pixels.
    pub(crate) height: u32,
}

/// Top-left placement position in viewport cells.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct ViewportPosition {
    /// Column from the left edge of the viewport; never negative.
    pub(crate) col: i32,

    /// Row from the top edge of the viewport; negative when partially scrolled above it.
    pub(crate) row: i32,
}

/// Copy every placed image on the terminal's active screen.
///
/// Unplaced uploads are omitted because they are not referenced by any placement. The returned
/// images are ordered by image ID and each placement list by placement ID, so identical storage
/// contents always produce the same order.
pub(crate) fn extract(terminal: &Terminal<'_, '_>) -> Result<Vec<ImageSnapshot>, String> {
    let graphics = terminal
        .kitty_graphics()
        .map_err(|error| format!("kitty graphics are unavailable: {error}"))?;
    let mut iterator = PlacementIterator::new()
        .map_err(|error| format!("create placement iterator failed: {error}"))?;
    let mut placements = iterator
        .update(&graphics)
        .map_err(|error| format!("read placements failed: {error}"))?;

    let mut grouped: BTreeMap<u32, Vec<PlacementSnapshot>> = BTreeMap::new();
    while let Some(placement) = placements.next() {
        let image_id = placement
            .image_id()
            .map_err(|error| format!("read placement image id failed: {error}"))?;
        let image = graphics.image(image_id).ok_or_else(|| {
            format!("kitty placement references image {image_id} which is not stored")
        })?;
        let is_virtual = placement
            .is_virtual()
            .map_err(|error| format!("read placement virtual flag failed: {error}"))?;
        let source_rect = placement
            .source_rect(&image)
            .map_err(|error| format!("read placement source rect failed: {error}"))?;
        let viewport_position = if is_virtual {
            None
        } else {
            placement
                .viewport_pos(&image, terminal)
                .map_err(|error| format!("read placement viewport position failed: {error}"))?
                .map(|position| ViewportPosition {
                    col: position.col,
                    row: position.row,
                })
        };
        grouped
            .entry(image_id)
            .or_default()
            .push(PlacementSnapshot {
                placement_id: placement
                    .placement_id()
                    .map_err(|error| format!("read placement id failed: {error}"))?,
                is_virtual,
                cell_columns: placement
                    .columns()
                    .map_err(|error| format!("read placement columns failed: {error}"))?,
                cell_rows: placement
                    .rows()
                    .map_err(|error| format!("read placement rows failed: {error}"))?,
                pixel_offset: PixelOffset {
                    x: placement
                        .x_offset()
                        .map_err(|error| format!("read placement x offset failed: {error}"))?,
                    y: placement
                        .y_offset()
                        .map_err(|error| format!("read placement y offset failed: {error}"))?,
                },
                source_rect: SourceRectSnapshot {
                    x: source_rect.x,
                    y: source_rect.y,
                    width: source_rect.width,
                    height: source_rect.height,
                },
                z_index: placement
                    .z()
                    .map_err(|error| format!("read placement z-index failed: {error}"))?,
                viewport_position,
            });
    }

    let mut images = Vec::with_capacity(grouped.len());
    for (image_id, mut image_placements) in grouped {
        let image = graphics.image(image_id).ok_or_else(|| {
            format!("kitty placement references image {image_id} which is not stored")
        })?;
        let format = match image
            .format()
            .map_err(|error| format!("read image format failed: {error}"))?
        {
            ImageFormat::Rgb => ImagePixelFormat::Rgb,
            ImageFormat::Rgba => ImagePixelFormat::Rgba,
            ImageFormat::Gray => ImagePixelFormat::Gray,
            ImageFormat::GrayAlpha => ImagePixelFormat::GrayAlpha,
            // The core documents that PNG payloads are decoded to RGBA before storage.
            other => {
                return Err(format!(
                    "stored image {image_id} has unsupported format {other:?}"
                ));
            }
        };
        let pixels = image
            .data()
            .map_err(|error| format!("read image {image_id} pixels failed: {error}"))?
            .to_vec();
        image_placements.sort_by_key(|placement| placement.placement_id);
        images.push(ImageSnapshot {
            image_id,
            width: image
                .width()
                .map_err(|error| format!("read image width failed: {error}"))?,
            height: image
                .height()
                .map_err(|error| format!("read image height failed: {error}"))?,
            format,
            pixels,
            placements: image_placements,
        });
    }
    Ok(images)
}

/// Encode one copied source image as PNG.
pub(crate) fn encode_png(image: &ImageSnapshot) -> Result<Vec<u8>, String> {
    let (color, bytes_per_pixel) = match image.format {
        ImagePixelFormat::Rgb => (png::ColorType::Rgb, 3usize),
        ImagePixelFormat::Rgba => (png::ColorType::Rgba, 4usize),
        ImagePixelFormat::Gray => (png::ColorType::Grayscale, 1usize),
        ImagePixelFormat::GrayAlpha => (png::ColorType::GrayscaleAlpha, 2usize),
    };
    let expected = (image.width as usize)
        .saturating_mul(image.height as usize)
        .saturating_mul(bytes_per_pixel);
    if image.pixels.len() != expected {
        return Err(format!(
            "image {} pixel buffer is {} bytes, expected {expected}",
            image.image_id,
            image.pixels.len()
        ));
    }

    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, image.width, image.height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("encode image {} header failed: {error}", image.image_id))?;
        writer
            .write_image_data(&image.pixels)
            .map_err(|error| format!("encode image {} pixels failed: {error}", image.image_id))?;
        writer
            .finish()
            .map_err(|error| format!("finish image {} PNG failed: {error}", image.image_id))?;
    }
    Ok(encoded)
}

/// PNG decoder handed to the terminal core for Kitty PNG payloads.
///
/// The core's decode callback requires RGBA8, including for grayscale PNGs. The binding's
/// bundled decoder has no public constructor, so decoding uses the `png` crate directly.
#[derive(Default)]
pub(crate) struct PngDecoder {
    buffer: Vec<u8>,
}

impl DecodePng for PngDecoder {
    fn decode_png<'alloc>(
        &mut self,
        alloc: &'alloc Allocator<'_>,
        data: &[u8],
    ) -> Option<DecodedImage<'alloc>> {
        use png::{Decoder, Transformations};

        let mut decoder = Decoder::new(Cursor::new(data));
        // ALPHA expands palettes and adds alpha, but grayscale stays grayscale-alpha.
        decoder.set_transformations(Transformations::ALPHA | Transformations::STRIP_16);

        let mut frame = decoder.read_info().ok()?;
        let buffer_size = frame.output_buffer_size()?;
        self.buffer.resize(buffer_size, 0);
        let info = frame.next_frame(&mut self.buffer).ok()?;

        let decoded = &self.buffer[..info.buffer_size()];
        let bytes = match info.color_type {
            png::ColorType::Rgba => {
                let mut bytes = Bytes::new_with_alloc(alloc, decoded.len()).ok()?;
                bytes.copy_from_slice(decoded);
                bytes
            }
            png::ColorType::GrayscaleAlpha => {
                let mut bytes = Bytes::new_with_alloc(alloc, decoded.len() * 2).ok()?;
                for (gray, rgba) in decoded.chunks_exact(2).zip(bytes.chunks_exact_mut(4)) {
                    rgba.copy_from_slice(&[gray[0], gray[0], gray[0], gray[1]]);
                }
                bytes
            }
            _ => return None,
        };
        frame.finish().ok()?;

        Some(DecodedImage {
            width: info.width,
            height: info.height,
            data: bytes,
        })
    }
}

/// Register the decoder used by all Kitty PNG payloads on the current thread.
pub(crate) fn install_png_decoder() -> Result<(), String> {
    graphics::set_png_decoder(Some(Box::new(PngDecoder::default())))
        .map_err(|error| format!("install PNG decoder failed: {error}"))
}

/// A frozen source image returned by `read(images=True)`.
///
/// The object owns the source pixels captured at read time, so later terminal mutations, image
/// replacement, or session close do not change it.
#[pyclass(name = "TerminalImage", frozen, module = "terminal_use")]
pub(crate) struct PyTerminalImage {
    /// Kitty image identifier.
    #[pyo3(get)]
    pub(crate) image_id: u32,

    width: u32,

    height: u32,

    placements: Vec<PlacementSnapshot>,

    png: Vec<u8>,
}

impl PyTerminalImage {
    /// Build the Python object from a captured image and its encoded PNG.
    pub(crate) fn new(image: ImageSnapshot, png: Vec<u8>) -> Self {
        Self {
            image_id: image.image_id,
            width: image.width,
            height: image.height,
            placements: image.placements,
            png,
        }
    }
}

#[pymethods]
impl PyTerminalImage {
    /// Return the source image dimensions as (width, height) in pixels.
    #[getter]
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Return a copy of the captured placement metadata; editing it cannot mutate the snapshot.
    #[getter]
    fn placements(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        pythonize(py, &self.placements)
            .map(Bound::unbind)
            .map_err(|error| {
                PyRuntimeError::new_err(format!("convert image placements failed: {error}"))
            })
    }

    /// Return this captured source image as PNG bytes for display_image().
    fn _repr_png_<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.png)
    }

    fn __repr__(&self) -> String {
        format!(
            "TerminalImage(image_id={}, size={}x{}, placements={})",
            self.image_id,
            self.width,
            self.height,
            self.placements.len()
        )
    }
}
