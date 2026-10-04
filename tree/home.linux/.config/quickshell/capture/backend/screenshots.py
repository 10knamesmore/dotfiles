"""采集原生像素截图，将界面合成的 BMP 裁剪并导出为 PNG。

每次 snapshot 独占运行时目录；release 可取消未完成采集，退出清理全部目录。
只有显式 save 写入 Pictures，复制和原始截图均不产生永久图片。
"""

from __future__ import annotations

import asyncio
import io
import re
import shutil
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from PIL import Image

from common import CaptureError, LOG, copy_to_clipboard, export_directory, export_filename, hyprland_json, run_command


@dataclass
class _Snapshot:
    directory: Path
    task: asyncio.Task | None = None


class Screenshots:
    """管理当前后端持有的截图和导出操作。

    Attributes:
        root: 当前进程的私有运行时目录，退出时整目录删除。
    """

    def __init__(self, root: Path, emit: Callable[..., None], report_error: Callable[..., None]) -> None:
        self.root = root
        self._emit = emit
        self._report_error = report_error
        self._snapshots: dict[str, _Snapshot] = {}

    def snapshot(self, request_id: str) -> None:
        """启动独立采集任务，使 stdin 可同时处理 release 或其他命令。

        同一 requestId 在 release 前不可再次采集；目录不使用调用方字符串命名。
        """
        if request_id in self._snapshots:
            raise CaptureError("request_in_use")
        snapshot = _Snapshot(Path(tempfile.mkdtemp(prefix="snapshot-", dir=self.root)))
        self._snapshots[request_id] = snapshot
        snapshot.task = asyncio.create_task(self._capture(request_id, snapshot))

    async def _capture(self, request_id: str, snapshot: _Snapshot) -> None:
        """读取桌面逻辑位置，以实际 PNG 尺寸保留各显示器的 scale 和变换。"""
        started = time.monotonic()
        LOG.info("snapshot started")
        try:
            monitors, clients, pointer, focused = await asyncio.gather(
                hyprland_json("monitors"), hyprland_json("clients"),
                hyprland_json("cursorpos"), hyprland_json("activewindow"),
            )
            # 同时采集各显示器；失败时也先等其余工具退出，再删除请求目录。
            screens = await asyncio.gather(*(
                self._capture_output(monitor, snapshot.directory / f"display-{index}.png")
                for index, monitor in enumerate(monitors) if not monitor.get("disabled", False)
            ), return_exceptions=True)
            for screen in screens:
                if isinstance(screen, Exception):
                    raise screen
            if not screens:
                raise CaptureError("no_outputs")
            names = {monitor["id"]: monitor["name"] for monitor in monitors}
            windows = [{
                "address": client["address"], "title": client["title"], "appId": client["class"],
                "x": client["at"][0], "y": client["at"][1],
                "width": client["size"][0], "height": client["size"][1],
                "monitor": names.get(client["monitor"], ""), "workspaceId": client["workspace"]["id"],
                "focused": client["address"] == focused.get("address"),
            } for client in clients if client["mapped"] and not client["hidden"]]
            self._emit("snapshot", requestId=request_id, screens=screens, windows=windows, pointer=pointer)
            LOG.info("snapshot completed displays=%s windows=%s elapsed_ms=%.0f", len(screens), len(windows), (time.monotonic() - started) * 1000)
        except asyncio.CancelledError:
            raise
        except Exception as error:
            self._report_error("snapshot", error, request_id)
            shutil.rmtree(snapshot.directory)
            self._snapshots.pop(request_id, None)

    async def _capture_output(self, monitor: dict, image_path: Path) -> dict:
        """采集单个显示器的原生像素；运行时 PNG 不压缩，避免阻塞浮层出现。"""
        scale = monitor["scale"]
        await run_command("grim", "-o", monitor["name"], "-s", str(scale), "-l", "0", str(image_path))
        with Image.open(image_path) as image:
            pixel_width, pixel_height = image.size
        return {
            "name": monitor["name"], "x": monitor["x"], "y": monitor["y"],
            "width": pixel_width / scale, "height": pixel_height / scale,
            "pixelWidth": pixel_width, "pixelHeight": pixel_height, "scale": scale,
            "image": str(image_path), "activeWorkspace": monitor["activeWorkspace"]["id"],
        }

    async def release(self, request_id: str) -> None:
        """取消尚未完成的采集，回收工具进程后删除原始图和界面合成图。"""
        snapshot = self._snapshots.pop(request_id, None)
        if snapshot is None:
            return
        if snapshot.task and not snapshot.task.done():
            snapshot.task.cancel()
            await asyncio.gather(snapshot.task, return_exceptions=True)
        shutil.rmtree(snapshot.directory, ignore_errors=True)
        LOG.info("snapshot released")

    async def export(self, command: dict) -> None:
        """读取本次 snapshot 目录内的合成 BMP，按物理像素裁剪为 PNG 后复制或保存。

        imagePath 必须是绝对路径，解析符号链接后仍须位于该 snapshot 目录内。
        合成图随 release 清理；导出成功不自动 release。
        """
        request_id = command["requestId"]
        if request_id not in self._snapshots:
            raise CaptureError("unknown_request")
        action = command["action"]
        if action not in ("copy", "save"):
            raise CaptureError("invalid_action")
        image_path = Path(command["imagePath"])
        snapshot_directory = self._snapshots[request_id].directory.resolve()
        if not image_path.is_absolute() or image_path.resolve().parent != snapshot_directory:
            raise CaptureError("invalid_image")
        LOG.info("screenshot export started action=%s", action)
        # BMP 中间图避免 UI 线程压缩整屏后又在这里解码；只压缩最终裁剪结果。
        png = await asyncio.to_thread(_crop_image_to_png, image_path, command["crop"])
        if action == "copy":
            await copy_to_clipboard("image/png", png)
            self._emit("exported", requestId=request_id, action=action)
        else:
            directory = await export_directory("PICTURES", "Screenshots")
            path = directory / export_filename("Screenshot", "png")
            await asyncio.to_thread(_write_png, path, png)
            self._emit("exported", requestId=request_id, action=action, path=str(path))
        LOG.info("screenshot export completed action=%s", action)

    async def copy_color(self, value: str) -> None:
        """复制十六进制或 RGB 色值，不记录颜色内容。"""
        if not re.fullmatch(r"#[0-9a-fA-F]{6}|rgb\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*\)", value):
            raise CaptureError("invalid_color")
        if value.startswith("rgb") and any(int(part) > 255 for part in re.findall(r"\d+", value)):
            raise CaptureError("invalid_color")
        await copy_to_clipboard("text/plain;charset=utf-8", value.encode())
        self._emit("copied_color", value=value)
        LOG.info("color copied")

    async def close(self) -> None:
        """回收全部采集任务，删除进程持有的运行时图片。"""
        await asyncio.gather(*(self.release(key) for key in list(self._snapshots)))


def _crop_image_to_png(image_path: Path, crop: dict) -> bytes:
    """读取合成 BMP，只裁剪图内的整数物理像素矩形并无损编码为 PNG。"""
    try:
        with Image.open(image_path) as image:
            if image.format != "BMP":
                raise CaptureError("invalid_image")
            x, y, width, height = (crop[key] for key in ("x", "y", "width", "height"))
            if any(type(value) is not int for value in (x, y, width, height)):
                raise CaptureError("invalid_crop")
            if x < 0 or y < 0 or width <= 0 or height <= 0 or x + width > image.width or y + height > image.height:
                raise CaptureError("invalid_crop")
            output = io.BytesIO()
            image.crop((x, y, x + width, y + height)).save(output, format="PNG", compress_level=1)
            return output.getvalue()
    except (ValueError, OSError) as error:
        raise CaptureError("invalid_image") from error


def _write_png(path: Path, data: bytes) -> None:
    with path.open("xb") as output:
        try:
            output.write(data)
        except OSError:
            path.unlink(missing_ok=True)
            raise
