"""Build the native SDK with the Zig toolchain installed in build isolation.

libghostty-vt-sys invokes `zig`, while the ziglang wheel only installs a
`python-zig` entry point. Its package directory contains the real `zig` binary.
Maturin otherwise owns the complete wheel, editable and source build process.
"""

import atexit
import os
import shutil
import tempfile
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

zig_dir = Path(ziglang.__file__).parent
zig_binary = zig_dir / "zig"

# zig 0.15.2 发现代理环境变量后，会在 CONNECT 隧道里发明文 HTTP 而不是做 TLS
# 握手（ziglang/zig#19878），经代理拉取 ghostty 依赖只会收到 400。用 shim 把这
# 些变量从 zig 的进程里抹掉：只有 zig 直连，uv/cargo/git 照常走代理。
shim_dir = Path(tempfile.mkdtemp(prefix="terminal-use-zig-"))
atexit.register(shutil.rmtree, shim_dir, ignore_errors=True)
zig_shim = shim_dir / "zig"
zig_shim.write_text(
    "#!/bin/sh\n"
    "unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY\n"
    f'exec "{zig_binary}" "$@"\n'
)
zig_shim.chmod(0o755)

os.environ["PATH"] = (
    str(shim_dir) + os.pathsep + str(zig_dir) + os.pathsep + os.environ["PATH"]
)
