# pi-repl-tools

Native image display and namespace summaries for Pi's Python worker on Linux and macOS. Distribution: `pi-repl-tools`; module: `pi_repl_tools`. Python 3.12 or later is required. Functions are exported by PyO3. The worker injects `display_image` and a session-bound `summarize`, so cells need no import.

## Image display

```python
from pathlib import Path
from pi_repl_tools import display_image

display_image(Path("chart.png"))
display_image(Path("photo.jpg").read_bytes())
```

`display_image(image, /) -> None` accepts a local `str`/`os.PathLike` path, encoded `bytes`, or an object whose `_repr_png_()` returns bytes, including Pillow images. Bytes always mean encoded image data. Tuple-returning rich representations and other IPython display protocols are not supported; no IPython runtime is required.

The format is detected from content, regardless of filename, and decoded for validation. PNG, JPEG, GIF and WebP retain their original bytes, including animation. Other decodable formats become PNG at the original width and height. Pixels are never resized. The enabled additional decoders cover BMP, DDS, OpenEXR, farbfeld, HDR, ICO, PNM, QOI and TIFF. AVIF and formats without recognizable content signatures are unsupported. Conversion can change color representation or discard metadata and additional pages.

Unsupported inputs or non-bytes `_repr_png_()` results raise `TypeError`. Invalid or unsupported encoded images raise `ValueError`; file access errors raise `OSError`. Representation and sink exceptions propagate unchanged.

The worker registers `_set_image_sink(callback, /) -> None`, where `callback(encoded_bytes, mime)` receives bytes plus `image/png`, `image/jpeg`, `image/gif` or `image/webp`. Passing `None` clears the sink. Displaying without a sink raises `RuntimeError`. Only the registering thread may display images or change an existing sink, keeping worker event writes serialized. The worker owns transport and call IDs; this package does not write to its event descriptor.

File reads, validation and PNG encoding release the GIL. Python signals are checked after native processing and before calling the sink, so an interrupted operation does not emit an image. Native decoding itself finishes before that signal check.

## Namespace summaries

The worker calls `_make_summarize(namespace, /)` after injecting its other helpers, then stores the returned callable as `summarize`. Calling `summarize()` prints `name`, `type` and `summary` columns for the current session namespace, including when called inside a function. Unchanged worker bindings are hidden; user names, including underscore-prefixed names, remain visible.

Summaries include scalar values, container lengths and bounded previews, module names, function parameters and defaults, NumPy array shape/dtype, and pandas DataFrame shape/columns. Optional data libraries are not imported. Unknown objects show their Python type only. Previews inspect exact built-in types, visit at most four entries per container and two nested container levels, and never invoke a custom object's `repr`. Each summary is limited to 240 characters. Function annotations are omitted without evaluation.

The binding uses Python's `functools.partial` and dictionaries, allowing Python GC to collect their reference cycle. Rust owns no global namespace reference. Output goes through Python `print`, preserving the worker's cell output capture.

## Diagnostics and build

Lifecycle, summary binding counts, image format, dimensions, encoded byte count and success/failure are logged to `$PI_REPL_TOOLS_LOG`, defaulting to the system temporary directory's `pi-repl-tools.log`. The log resets at approximately 1 MiB. Logs exclude object values, image contents, file paths and exception messages.

Build from this directory with `cargo check --locked` and `uv build --wheel --out-dir /tmp/pi-repl-tools-wheels`. Maturin uses PyO3 0.29.2 and `abi3-py312`, with a locked Cargo dependency graph. This package has no desktop or Wayland dependency and is built on both Linux and macOS.
