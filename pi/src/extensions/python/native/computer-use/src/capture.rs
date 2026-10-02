//! Capture complete outputs before orienting, cropping and encoding their pixels.

use std::io::{Cursor, Read, Seek};
use std::os::fd::AsFd;

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage, imageops};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use wayland_client::protocol::wl_shm;

use crate::hyprland::{self, Monitor};
use crate::wayland::{Desktop, Frame};
use crate::{logging, wait};

type Bounds = (f64, f64, f64, f64);
type Rect = (i64, i64, i64, i64);

/// An immutable image and its captured desktop coordinates; create with screenshot().
#[pyclass(module = "computer_use", frozen)]
pub struct Screenshot {
    /// Hyprland output name at capture time.
    #[pyo3(get)]
    monitor: String,

    /// Dimensions of the returned PNG, in pixels after cropping and downscaling.
    #[pyo3(get)]
    size: (u32, u32),

    /// Captured rectangle in desktop logical coordinates, including crop offsets.
    #[pyo3(get)]
    bounds: Bounds,

    /// Encoded pixels remain private and are never included in diagnostics.
    png: Vec<u8>,
}

#[pymethods]
impl Screenshot {
    /// Return PNG bytes for display_image(); this does not recapture the screen.
    fn _repr_png_<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.png)
    }

    fn __repr__(&self) -> String {
        format!(
            "Screenshot(monitor={:?}, size={:?}, bounds={:?})",
            self.monitor, self.size, self.bounds
        )
    }
}

impl Screenshot {
    pub(crate) fn desktop_point(&self, x: f64, y: f64) -> PyResult<(f64, f64)> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.size.0 as f64
            || y >= self.size.1 as f64
        {
            return Err(wait::invalid(
                "coordinates must be inside the returned screenshot pixels",
            ));
        }
        Ok((
            self.bounds.0 + x * self.bounds.2 / self.size.0 as f64,
            self.bounds.1 + y * self.bounds.3 / self.size.1 as f64,
        ))
    }
}

/// Capture the focused or named monitor; rect uses full, orientation-corrected pixels.
#[pyfunction]
#[pyo3(signature = (monitor = None, *, rect = None, max_size = None))]
pub(crate) fn screenshot(
    py: Python<'_>,
    monitor: Option<String>,
    rect: Option<Rect>,
    max_size: Option<u32>,
) -> PyResult<Screenshot> {
    let result = (|| {
        if max_size == Some(0) {
            return Err(wait::invalid("max_size must be greater than zero"));
        }
        if let Some((x, y, w, h)) = rect
            && (x < 0 || y < 0 || w <= 0 || h <= 0)
        {
            return Err(wait::invalid(
                "rect must have non-negative x/y and positive width/height",
            ));
        }
        let info = hyprland::monitors(py)?
            .into_iter()
            .find(|info| {
                monitor
                    .as_ref()
                    .map_or(info.focused, |name| *name == info.name)
            })
            .ok_or_else(|| wait::invalid("requested monitor was not found"))?;
        let mut desktop = Desktop::connect(py)?;
        let output = desktop
            .state
            .outputs
            .values()
            .find(|output| output.name == info.name)
            .ok_or_else(|| wait::runtime("monitor is not advertised by Wayland"))?;
        let transform = output.transform;
        let manager = desktop
            .state
            .screencopy
            .as_ref()
            .ok_or_else(|| wait::runtime("zwlr_screencopy_manager_v1 v3 is required"))?;
        let frame = manager.capture_output(1, &output.proxy, &desktop.queue.handle(), ());
        desktop.wait_for(py, true, |state| {
            state.frame.buffer_done || state.frame.failed
        })?;
        if desktop.state.frame.failed {
            return Err(wait::runtime("compositor refused the screenshot"));
        }
        let format =
            desktop.state.frame.format.ok_or_else(|| {
                wait::runtime("compositor did not offer a shared-memory screenshot")
            })?;
        let geometry = &desktop.state.frame;
        let length = geometry
            .stride
            .checked_mul(geometry.height)
            .filter(|length| *length <= i32::MAX as u32 && *length > 0)
            .ok_or_else(|| wait::runtime("invalid screenshot buffer size"))?;
        if geometry.width > i32::MAX as u32 || geometry.height > i32::MAX as u32 {
            return Err(wait::runtime("invalid screenshot dimensions"));
        }
        let file = tempfile::tempfile().map_err(wait::runtime)?;
        file.set_len(length as u64).map_err(wait::runtime)?;
        let shm = desktop
            .state
            .shm
            .as_ref()
            .ok_or_else(|| wait::runtime("wl_shm is required"))?;
        let pool = shm.create_pool(file.as_fd(), length as i32, &desktop.queue.handle(), ());
        let buffer = pool.create_buffer(
            0,
            geometry.width as i32,
            geometry.height as i32,
            geometry.stride as i32,
            format,
            &desktop.queue.handle(),
            (),
        );
        frame.copy(&buffer);
        desktop.wait_for(py, true, |state| state.frame.ready || state.frame.failed)?;
        if desktop.state.frame.failed {
            return Err(wait::runtime("compositor could not copy the screenshot"));
        }
        frame.destroy();
        buffer.destroy();
        pool.destroy();
        desktop.sync(py, true)?;
        let geometry = std::mem::take(&mut desktop.state.frame);
        drop(desktop);
        wait::compute(py, move || {
            let mut file = file;
            file.rewind().map_err(wait::runtime)?;
            let mut bytes = vec![0; length as usize];
            file.read_exact(&mut bytes).map_err(wait::runtime)?;
            render(bytes, geometry, transform, info, rect, max_size)
        })
    })();
    logging::result("screenshot", result)
}

fn render(
    bytes: Vec<u8>,
    frame: Frame,
    transform: u32,
    monitor: Monitor,
    rect: Option<Rect>,
    max_size: Option<u32>,
) -> PyResult<Screenshot> {
    let format = frame
        .format
        .ok_or_else(|| wait::runtime("missing screenshot format"))?;
    use wl_shm::Format;
    let supported = matches!(
        format,
        Format::Argb8888
            | Format::Xrgb8888
            | Format::Abgr8888
            | Format::Xbgr8888
            | Format::Argb2101010
            | Format::Xrgb2101010
            | Format::Abgr2101010
            | Format::Xbgr2101010
    );
    if !supported {
        return Err(wait::runtime(format!(
            "unsupported screenshot pixel format: {format:?}"
        )));
    }
    if (frame.stride as u64) < frame.width as u64 * 4 {
        return Err(wait::runtime(
            "screenshot stride is smaller than one pixel row",
        ));
    }
    let mut pixels = RgbaImage::new(frame.width, frame.height);
    for (y, row) in bytes.chunks_exact(frame.stride as usize).enumerate() {
        for (x, pixel) in row[..frame.width as usize * 4].chunks_exact(4).enumerate() {
            let word = u32::from_ne_bytes(pixel.try_into().unwrap());
            let (r, g, b) = match format {
                Format::Argb8888 | Format::Xrgb8888 => {
                    ((word >> 16) as u8, (word >> 8) as u8, word as u8)
                }
                Format::Abgr8888 | Format::Xbgr8888 => {
                    (word as u8, (word >> 8) as u8, (word >> 16) as u8)
                }
                Format::Argb2101010 | Format::Xrgb2101010 => {
                    (ten_bit(word >> 20), ten_bit(word >> 10), ten_bit(word))
                }
                _ => (ten_bit(word), ten_bit(word >> 10), ten_bit(word >> 20)),
            };
            pixels.put_pixel(x as u32, y as u32, Rgba([r, g, b, 255]));
        }
    }
    if frame.y_invert {
        imageops::flip_vertical_in_place(&mut pixels);
    }
    // Wayland transform rotation is clockwise in image coordinates; reflection is
    // applied afterwards, as in grim's output-to-composite transformation.
    pixels = match transform & 3 {
        1 => imageops::rotate90(&pixels),
        2 => imageops::rotate180(&pixels),
        3 => imageops::rotate270(&pixels),
        _ => pixels,
    };
    if transform & 4 != 0 {
        imageops::flip_horizontal_in_place(&mut pixels);
    }
    let (full_width, full_height) = pixels.dimensions();
    let (x, y, width, height) = rect.unwrap_or((0, 0, full_width as i64, full_height as i64));
    if x.checked_add(width)
        .is_none_or(|right| right > full_width as i64)
        || y.checked_add(height)
            .is_none_or(|bottom| bottom > full_height as i64)
    {
        return Err(wait::invalid(
            "rect is outside the full orientation-corrected screenshot",
        ));
    }
    let screen = monitor.bounds();
    let bounds = (
        screen.0 + x as f64 * screen.2 / full_width as f64,
        screen.1 + y as f64 * screen.3 / full_height as f64,
        width as f64 * screen.2 / full_width as f64,
        height as f64 * screen.3 / full_height as f64,
    );
    let cropped =
        imageops::crop_imm(&pixels, x as u32, y as u32, width as u32, height as u32).to_image();
    let mut image = DynamicImage::ImageRgba8(cropped);
    if let Some(maximum) = max_size
        && image.width().max(image.height()) > maximum
    {
        image = image.resize(maximum, maximum, imageops::FilterType::Lanczos3);
    }
    let size = (image.width(), image.height());
    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, ImageFormat::Png)
        .map_err(wait::runtime)?;
    Ok(Screenshot {
        monitor: monitor.name,
        size,
        bounds,
        png: png.into_inner(),
    })
}

fn ten_bit(value: u32) -> u8 {
    (((value & 1023) * 255 + 511) / 1023) as u8
}
