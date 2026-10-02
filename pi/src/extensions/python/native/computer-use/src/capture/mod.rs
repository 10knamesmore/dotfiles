//! Capture native pixel buffers with a lazily encoded image of the same frame.

mod buffer;

use std::io::{Read, Seek};
use std::os::fd::AsFd;

use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::hyprland::{self, Monitor};
use crate::wayland::{Desktop, Frame};
use crate::{logging, wait};
use buffer::PixelBuffer;

type Bounds = (f64, f64, f64, f64);
type Rect = (i64, i64, i64, i64);

/// One immutable captured region, usable as native pixels or an image; create with capture().
#[pyclass(module = "computer_use", frozen)]
pub(crate) struct Capture {
    /// Hyprland output name at capture time.
    #[pyo3(get)]
    monitor: String,

    /// Dimensions of the returned PNG, in pixels after cropping and downscaling.
    #[pyo3(get)]
    size: (u32, u32),

    /// Captured rectangle in desktop logical coordinates, including crop offsets.
    #[pyo3(get)]
    bounds: Bounds,

    /// Full-resolution region, retaining compositor channel packing and bit depth.
    #[pyo3(get)]
    buffer: PixelBuffer,
}

#[pymethods]
impl Capture {
    /// Encode PNG for display_image() on demand; never recaptures or modifies the buffer.
    fn _repr_png_<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let buffer = self.buffer.clone();
        let size = self.size;
        let png = logging::result("capture.image", wait::compute(py, move || buffer.png(size)))?;
        Ok(PyBytes::new(py, &png))
    }

    fn __repr__(&self) -> String {
        format!(
            "Capture(monitor={:?}, size={:?}, bounds={:?})",
            self.monitor, self.size, self.bounds
        )
    }
}

impl Capture {
    pub(crate) fn desktop_point(&self, x: f64, y: f64) -> PyResult<(f64, f64)> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.size.0 as f64
            || y >= self.size.1 as f64
        {
            return Err(wait::invalid(
                "coordinates must be inside the capture image pixels",
            ));
        }
        Ok((
            self.bounds.0 + x * self.bounds.2 / self.size.0 as f64,
            self.bounds.1 + y * self.bounds.3 / self.size.1 as f64,
        ))
    }
}

/// Capture native pixels from the focused or named monitor; max_size affects only the image.
#[pyfunction]
#[pyo3(signature = (monitor = None, *, rect = None, max_size = None))]
fn capture(
    py: Python<'_>,
    monitor: Option<String>,
    rect: Option<Rect>,
    max_size: Option<u32>,
) -> PyResult<Capture> {
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
            return Err(wait::runtime("compositor refused the capture"));
        }
        let format = desktop
            .state
            .frame
            .format
            .ok_or_else(|| wait::runtime("compositor did not offer a shared-memory capture"))?;
        let geometry = &desktop.state.frame;
        let length = geometry
            .stride
            .checked_mul(geometry.height)
            .filter(|length| *length <= i32::MAX as u32 && *length > 0)
            .ok_or_else(|| wait::runtime("invalid capture buffer size"))?;
        if geometry.width > i32::MAX as u32 || geometry.height > i32::MAX as u32 {
            return Err(wait::runtime("invalid capture dimensions"));
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
            return Err(wait::runtime("compositor could not copy the capture"));
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
            prepare(bytes, geometry, transform, info, rect, max_size)
        })
    })();
    logging::result("capture", result)
}

fn prepare(
    bytes: Vec<u8>,
    frame: Frame,
    transform: u32,
    monitor: Monitor,
    rect: Option<Rect>,
    max_size: Option<u32>,
) -> PyResult<Capture> {
    let buffer = PixelBuffer::from_frame(&bytes, &frame, transform)?;
    let (full_width, full_height) = buffer.size();
    let rect = rect.unwrap_or((0, 0, full_width as i64, full_height as i64));
    let buffer = buffer.crop(rect)?;
    let (x, y, width, height) = rect;
    let screen = monitor.bounds();
    let bounds = (
        screen.0 + x as f64 * screen.2 / full_width as f64,
        screen.1 + y as f64 * screen.3 / full_height as f64,
        width as f64 * screen.2 / full_width as f64,
        height as f64 * screen.3 / full_height as f64,
    );
    let mut size = buffer.size();
    if let Some(maximum) = max_size
        && size.0.max(size.1) > maximum
    {
        let scale = maximum as f64 / size.0.max(size.1) as f64;
        size = (
            ((size.0 as f64 * scale).round() as u32).max(1),
            ((size.1 as f64 * scale).round() as u32).max(1),
        );
    }
    Ok(Capture {
        monitor: monitor.name,
        size,
        bounds,
        buffer,
    })
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Capture>()?;
    module.add_class::<PixelBuffer>()?;
    module.add_function(wrap_pyfunction!(capture, module)?)?;
    Ok(())
}
