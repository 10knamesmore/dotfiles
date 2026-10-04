"""提供捕获后端的会话路径、结构化错误和外部命令执行。

命令输出不进入界面；stdout 由 server 独占为 NDJSON，诊断只写 stderr。
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import tempfile
from pathlib import Path
from datetime import datetime
from uuid import uuid4

LOG = logging.getLogger("dots.capture")


class CaptureError(Exception):
    """携带可供界面翻译的错误，不把内部诊断当成展示文案。

    Attributes:
        code: 与操作无关的失败种类。
        dependency: 缺失的可执行文件名称；其他错误为 None。
    """

    def __init__(self, code: str, dependency: str | None = None) -> None:
        super().__init__(code)
        self.code = code
        self.dependency = dependency


def session_directory() -> Path:
    """定位当前 Hyprland 的运行时目录，不跨桌面会话共享捕获数据。"""
    return Path(os.environ["XDG_RUNTIME_DIR"]) / "hypr" / os.environ["HYPRLAND_INSTANCE_SIGNATURE"]


def socket_path() -> Path:
    """返回 XDPH picker 与当前 Quickshell 后端共同使用的私有 socket。"""
    return session_directory() / "dots-capture.sock"


async def run_command(*args: str, data: bytes | None = None) -> bytes:
    """执行捕获工具并等候完成；取消时终止并回收本次子进程。

    Args:
        args: 可执行文件及其独立参数，不经过 shell。
        data: 通过 stdin 传入的数据；None 表示无输入。

    Raises:
        CaptureError: 工具缺失或命令以非零状态退出。
    """
    try:
        process = await asyncio.create_subprocess_exec(
            *args,
            stdin=asyncio.subprocess.PIPE if data is not None else asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
    except FileNotFoundError as error:
        raise CaptureError("missing_dependency", args[0]) from error
    try:
        output, diagnostic = await process.communicate(data)
    except asyncio.CancelledError:
        if process.returncode is None:
            process.kill()
        await process.communicate()
        raise
    if process.returncode != 0:
        LOG.error("%s exited status=%s: %s", args[0], process.returncode, diagnostic.decode(errors="replace").strip())
        raise CaptureError("command_failed")
    if diagnostic:
        LOG.debug("%s: %s", args[0], diagnostic.decode(errors="replace").strip())
    return output


async def copy_to_clipboard(mime_type: str, data: bytes) -> None:
    """发布 Wayland 剪贴板内容，等 wl-copy 父进程成功退出后返回。

    剪贴板由 wl-copy 的后台子进程继续持有，不属于 server 的退出清理范围。
    stdout 不使用 PIPE，stderr 写匿名文件；子进程继承它们也不会阻止命令完成。
    取消仅回收尚未退出的父进程，不终止已经接管 selection 的后台服务。

    Args:
        mime_type: 内容的 MIME 类型。
        data: 要提供给剪贴板读取者的完整内容。

    Raises:
        CaptureError: wl-copy 缺失或父进程以非零状态退出。
    """
    with tempfile.TemporaryFile() as diagnostic_file:
        try:
            process = await asyncio.create_subprocess_exec(
                "wl-copy", "--type", mime_type,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=diagnostic_file,
            )
        except FileNotFoundError as error:
            raise CaptureError("missing_dependency", "wl-copy") from error
        try:
            await process.communicate(data)
        except asyncio.CancelledError:
            process.stdin.close()
            if process.returncode is None:
                process.kill()
            await process.wait()
            raise
        diagnostic_file.seek(0)
        diagnostic = diagnostic_file.read().decode(errors="replace").strip()
        if process.returncode != 0:
            LOG.error("wl-copy exited status=%s: %s", process.returncode, diagnostic)
            raise CaptureError("command_failed")
        if diagnostic:
            LOG.debug("wl-copy: %s", diagnostic)


async def hyprland_json(query: str) -> object:
    """读取 compositor 状态，不记录包含窗口标题的原始响应。"""
    return json.loads(await run_command("hyprctl", "-j", query))


async def export_directory(kind: str, folder: str) -> Path:
    """查询 XDG 用户目录并创建本次导出的子目录。

    Args:
        kind: xdg-user-dir 的 PICTURES 或 VIDEOS。
        folder: Screenshots 或 Recordings；原始截图不使用此函数。
    """
    directory = Path((await run_command("xdg-user-dir", kind)).decode().strip())
    if not directory.is_absolute():
        raise CaptureError("invalid_save_directory")
    destination = directory / folder
    destination.mkdir(parents=True, exist_ok=True)
    return destination


def export_filename(prefix: str, extension: str) -> str:
    """生成包含本地时间的文件名，避免同秒导出覆盖已有成品。"""
    return f"{prefix}_{datetime.now():%Y-%m-%d_%H-%M-%S}_{uuid4().hex[:8]}.{extension}"
