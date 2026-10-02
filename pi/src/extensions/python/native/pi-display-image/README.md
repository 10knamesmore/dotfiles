# pi-display-image

Native image display SDK for Pi's Python worker on Linux and macOS. Distribution: `pi-display-image-sdk`; module: `pi_display_image`. Python 3.12 or later is required. Both functions are exported directly by PyO3, with signatures in `pi_display_image.pyi`.

```python
from pathlib import Path
from pi_display_image import display_image

display_image(Path("chart.png"))
display_image(Path("photo.jpg").read_bytes())
```

`display_image(image, /) -> None` accepts a local `str`/`os.PathLike` path, encoded `bytes`, or an object whose `_repr_png_()` returns bytes, including Pillow images. Bytes always mean encoded image data. Tuple-returning rich representations and other IPython display protocols are not supported; no IPython runtime is required.

The format is detected from content, regardless of filename, and decoded for validation. PNG, JPEG, GIF and WebP retain their original bytes, including animation. Other decodable formats become PNG at the original width and height. Pixels are never resized. The enabled additional decoders cover BMP, DDS, OpenEXR, farbfeld, HDR, ICO, PNM, QOI and TIFF. AVIF and formats without recognizable content signatures are unsupported. Conversion can change color representation or discard metadata and additional pages.

Unsupported inputs or non-bytes `_repr_png_()` results raise `TypeError`. Invalid or unsupported encoded images raise `ValueError`; file access errors raise `OSError`. Representation and sink exceptions propagate unchanged.

The worker registers `_set_image_sink(callback, /) -> None`, where `callback(encoded_bytes, mime)` receives bytes plus `image/png`, `image/jpeg`, `image/gif` or `image/webp`. Passing `None` clears the sink. Displaying without a sink raises `RuntimeError`. Only the registering thread may display images or change an existing sink, keeping worker event writes serialized. The worker owns transport and call IDs; this package does not write to its event descriptor.

File reads, validation and PNG encoding release the GIL. Python signals are checked after native processing and before calling the sink, so an interrupted operation does not emit an image. Native decoding itself finishes before that signal check.

Lifecycle, image format, dimensions, encoded byte count and success/failure are logged to `$PI_DISPLAY_IMAGE_LOG`, defaulting to the system temporary directory's `pi-display-image-sdk.log`. The log resets at approximately 1 MiB. Logs exclude image contents, file paths and exception messages.

Build from this directory with `cargo check --locked` and `uv build --wheel --out-dir /tmp/pi-display-image-wheels`. Maturin uses PyO3 0.29.2 and `abi3-py312`, with a locked Cargo dependency graph. This package has no desktop or Wayland dependency and is built on both Linux and macOS.
