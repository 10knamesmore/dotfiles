//! Validate image inputs and deliver encoded images to the Python worker's sink.

mod diagnostics;

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::Instant;

use image::ImageFormat;
use pyo3::exceptions::{PyAttributeError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule, PyString};

static IMAGE_SINK: Mutex<Option<ImageSink>> = Mutex::new(None);

/// Keep the callback and its owner together so event writes stay on one thread.
struct ImageSink {
    /// Worker callback receiving encoded bytes and their MIME type.
    callback: Py<PyAny>,

    /// Thread allowed to invoke or replace this callback.
    thread: ThreadId,
}

/// Own input data while file reads and decoding run without the GIL.
enum ImageSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

/// Carry validated bytes and metadata without retaining decoded pixels.
struct PreparedImage {
    /// Original supported encoding or a PNG produced from the decoded raster.
    bytes: Vec<u8>,

    /// MIME type inferred from the bytes delivered to the sink.
    mime: &'static str,

    /// Original decoded width and height in pixels; conversion never resizes.
    dimensions: (u32, u32),
}

/// Register the native Python API; the worker installs its own image transport.
#[pymodule]
fn pi_display_image(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(display_image, module)?)?;
    module.add_function(wrap_pyfunction!(_set_image_sink, module)?)?;
    Ok(())
}

/// Install or clear the worker callback on the thread that will display images.
///
/// The callback receives encoded bytes and the inferred MIME type. Passing None
/// releases it. An existing sink can only be changed by its registering thread.
/// Callback exceptions propagate from display_image unchanged.
#[pyfunction]
#[pyo3(signature = (callback, /))]
fn _set_image_sink(callback: Option<Bound<'_, PyAny>>) -> PyResult<()> {
    let result = (|| {
        if callback.as_ref().is_some_and(|value| !value.is_callable()) {
            return Err(PyTypeError::new_err("image sink must be callable or None"));
        }
        let mut sink = IMAGE_SINK
            .lock()
            .map_err(|_| PyRuntimeError::new_err("image sink mutex poisoned"))?;
        if let Some(sink) = sink.as_ref() {
            ensure_sink_thread(sink)?;
        }
        let replacement = callback.map(|callback| ImageSink {
            callback: callback.unbind(),
            thread: std::thread::current().id(),
        });
        let previous = std::mem::replace(&mut *sink, replacement);
        let installed = sink.is_some();
        // Python finalizers may call this module again; release the mutex first.
        drop(sink);
        drop(previous);
        diagnostics::event(if installed {
            "sink_registered"
        } else {
            "sink_cleared"
        });
        Ok(())
    })();
    if result.is_err() {
        diagnostics::event("sink_registration_failed");
    }
    result
}

/// Display an image through the worker's registered sink without resizing.
///
/// Accept a local str/PathLike path, encoded bytes, or an object whose
/// _repr_png_() returns bytes (including Pillow images). Detect the format from
/// content and decode it for validation. Preserve PNG, JPEG, GIF and WebP bytes;
/// encode other supported raster formats as PNG at the original dimensions.
///
/// Raise RuntimeError if no sink is installed or this is not its owning thread,
/// TypeError for unsupported input, ValueError for invalid/unsupported images,
/// and OSError for file access failures. Representation and sink exceptions
/// propagate. File reads and image processing release the GIL; pending Python
/// signals are checked before delivering any image.
#[pyfunction]
#[pyo3(signature = (image, /))]
fn display_image(py: Python<'_>, image: &Bound<'_, PyAny>) -> PyResult<()> {
    let started = Instant::now();
    diagnostics::event("display_started");
    let result = display(py, image);
    match &result {
        Ok(()) => diagnostics::event(&format!(
            "display_completed elapsed_ms={}",
            started.elapsed().as_millis()
        )),
        Err(_) => diagnostics::event(&format!(
            "display_failed elapsed_ms={}",
            started.elapsed().as_millis()
        )),
    }
    result
}

fn display(py: Python<'_>, image: &Bound<'_, PyAny>) -> PyResult<()> {
    image_sink(py)?;
    py.check_signals()?;
    let source = image_source(image)?;
    let prepared = py.detach(|| prepare_image(source));
    py.check_signals()?;
    let prepared = prepared?;
    diagnostics::event(&format!(
        "image_validated mime={} width={} height={} bytes={}",
        prepared.mime,
        prepared.dimensions.0,
        prepared.dimensions.1,
        prepared.bytes.len()
    ));
    // _repr_png_ may run arbitrary Python, including clearing the sink.
    let callback = image_sink(py)?;
    callback.call1(py, (PyBytes::new(py, &prepared.bytes), prepared.mime))?;
    Ok(())
}

fn image_sink(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let sink = IMAGE_SINK
        .lock()
        .map_err(|_| PyRuntimeError::new_err("image sink mutex poisoned"))?;
    let sink = sink.as_ref().ok_or_else(|| {
        PyRuntimeError::new_err(
            "display_image requires an image sink registered by the Python worker",
        )
    })?;
    ensure_sink_thread(sink)?;
    Ok(sink.callback.clone_ref(py))
}

fn ensure_sink_thread(sink: &ImageSink) -> PyResult<()> {
    if sink.thread != std::thread::current().id() {
        return Err(PyRuntimeError::new_err(
            "image sink operations must run on the thread that registered the sink",
        ));
    }
    Ok(())
}

fn image_source(value: &Bound<'_, PyAny>) -> PyResult<ImageSource> {
    if let Ok(bytes) = value.cast::<PyBytes>() {
        return Ok(ImageSource::Bytes(bytes.as_bytes().to_vec()));
    }
    if value.is_instance_of::<PyString>() || value.hasattr("__fspath__")? {
        let path = value
            .py()
            .import("os")?
            .call_method1("fsdecode", (value,))?;
        return Ok(ImageSource::Path(path.extract::<PathBuf>()?));
    }
    let representation = match value.getattr("_repr_png_") {
        Ok(method) => method.call0()?,
        Err(error) if error.is_instance_of::<PyAttributeError>(value.py()) => {
            return Err(PyTypeError::new_err(
                "image must be a local str/PathLike path, encoded bytes, or an object with _repr_png_() -> bytes",
            ));
        }
        Err(error) => return Err(error),
    };
    let bytes = representation
        .cast::<PyBytes>()
        .map_err(|_| PyTypeError::new_err("_repr_png_() must return bytes"))?;
    Ok(ImageSource::Bytes(bytes.as_bytes().to_vec()))
}

fn prepare_image(source: ImageSource) -> PyResult<PreparedImage> {
    let bytes = match source {
        ImageSource::Path(path) => std::fs::read(path)?,
        ImageSource::Bytes(bytes) => bytes,
    };
    let format = image::guess_format(&bytes).map_err(image_error)?;
    let decoded = image::load_from_memory_with_format(&bytes, format).map_err(image_error)?;
    let dimensions = (decoded.width(), decoded.height());
    let mime = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Gif => "image/gif",
        ImageFormat::WebP => "image/webp",
        _ => {
            let mut encoded = Cursor::new(Vec::new());
            decoded
                .write_to(&mut encoded, ImageFormat::Png)
                .map_err(image_error)?;
            return Ok(PreparedImage {
                bytes: encoded.into_inner(),
                mime: "image/png",
                dimensions,
            });
        }
    };
    Ok(PreparedImage {
        bytes,
        mime,
        dimensions,
    })
}

fn image_error(error: image::ImageError) -> PyErr {
    PyValueError::new_err(format!("invalid or unsupported image: {error}"))
}
