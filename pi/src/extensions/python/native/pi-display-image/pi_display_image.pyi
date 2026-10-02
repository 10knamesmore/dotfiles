"""Display validated images through the Python worker's image sink."""

from collections.abc import Callable
from os import PathLike
from typing import Protocol

class _PngRepresentation(Protocol):
    """Provide encoded image bytes using the PNG representation convention."""

    def _repr_png_(self) -> bytes | None: ...

def display_image(
    image: str | PathLike[str] | PathLike[bytes] | bytes | _PngRepresentation,
    /,
) -> None:
    """Display an image at its original dimensions on the sink's owning thread.

    Args:
        image: Local path, encoded image bytes, or an object with _repr_png_().

    Raises:
        RuntimeError: No sink is registered, or another thread owns it.
        TypeError: Input is unsupported or _repr_png_() does not return bytes.
        ValueError: Encoded data cannot be decoded as a supported image.
        OSError: The local file cannot be read.

    Representation and sink exceptions propagate unchanged.
    """
    ...

def _set_image_sink(callback: Callable[[bytes, str], None] | None, /) -> None:
    """Register the worker callback on its owning thread; None releases it.

    The callback receives encoded bytes and inferred MIME. An existing sink can
    only be replaced or cleared by its owner. The worker owns transport and call IDs.
    """
    ...
