#!/usr/bin/env python3
"""让 Quickshell 跟踪启动器，而非负责录制收尾的 server。

server 继承 stdin/stdout/stderr，启动器只等待其退出，不转发协议、不自动重启。
Quickshell 销毁 Process 时即使 SIGKILL 启动器，关闭 stdin 管道后 server 仍可
收到 EOF，停止自有录制并等待保存和临时文件清理。
"""

import subprocess
import sys
from pathlib import Path


def main() -> int:
    """启动相邻的 server.py，继承标准流并返回其退出码。"""
    server = subprocess.Popen([sys.executable, str(Path(__file__).with_name("server.py"))])
    return server.wait()


if __name__ == "__main__":
    raise SystemExit(main())
