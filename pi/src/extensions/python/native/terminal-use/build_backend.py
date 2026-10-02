"""Build the native SDK with the Zig toolchain installed in build isolation.

libghostty-vt-sys invokes `zig`, while the ziglang wheel only installs a
`python-zig` entry point. Its package directory contains the real `zig` binary.
Maturin otherwise owns the complete wheel, editable and source build process.
"""

import os
from pathlib import Path

import ziglang
from maturin import (
    build_editable,
    build_sdist,
    build_wheel,
    get_requires_for_build_editable,
    get_requires_for_build_sdist,
    get_requires_for_build_wheel,
    prepare_metadata_for_build_editable,
    prepare_metadata_for_build_wheel,
)

os.environ["PATH"] = str(Path(ziglang.__file__).parent) + os.pathsep + os.environ["PATH"]
