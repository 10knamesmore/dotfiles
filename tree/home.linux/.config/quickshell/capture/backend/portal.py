"""桥接 XDPH 的独立 picker，并识别选择请求是否来自本进程的录制。

XDPH 不向 custom picker 传调用者身份。通过 busctl 观察后端 SelectSources 的
未完成调用，再从 session 路径查询原调用者 PID；不能把“正在选录屏范围”当成授权。
所有请求仍须由 UI 逐次确认，识别调用者仅用于录制状态和取消处理。
"""

from __future__ import annotations

import asyncio
import json
from dataclasses import dataclass, field
from typing import Callable
from uuid import uuid4

from common import CaptureError, LOG, hyprland_json, run_command

_PORTAL_SERVICE = "org.freedesktop.impl.portal.desktop.hyprland"
_PORTAL_INTERFACE = "org.freedesktop.impl.portal.ScreenCast"


@dataclass
class _PortalCall:
    """保留尚未返回的 SelectSources 调用及原应用 PID；查询结束后唤醒 picker。"""

    session_handle: str
    caller_pid: int | None = None
    resolved: asyncio.Event = field(default_factory=asyncio.Event)


class PortalMonitor:
    """识别当前 XDPH 同步 picker 的调用者，不按 appId 或窗口列表猜测来源。

    XDPH 在 SelectSources 中同步等待 picker，未完成调用按接收顺序处理。
    只在 D-Bus 查询到的原调用者 PID 等于自己启动的 GSR 时关联录制。

    Attributes:
        available: 观察进程存活且已确认订阅；为 False 时禁止启动自有录制。
    """

    def __init__(self, report_error: Callable[..., None]) -> None:
        self._report_error = report_error
        self._process: asyncio.subprocess.Process | None = None
        self._task: asyncio.Task | None = None
        self._calls: dict[tuple[str, int], _PortalCall] = {}
        self._seen = asyncio.Event()
        self._changed = asyncio.Event()
        self.available = False
        self._closing = False

    async def start(self) -> None:
        """在接收 picker 前建立观察通道，用只读属性请求确认订阅已经生效。"""
        try:
            self._process = await asyncio.create_subprocess_exec(
                "busctl", "--user", "--json=short", "--no-pager", "monitor", _PORTAL_SERVICE,
                stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
            )
            self._task = asyncio.create_task(self._read())
            async with asyncio.timeout(3):
                while not self._seen.is_set():
                    await run_command(
                        "busctl", "--user", "--timeout=1", "get-property", _PORTAL_SERVICE,
                        "/org/freedesktop/portal/desktop", _PORTAL_INTERFACE, "version",
                    )
                    try:
                        await asyncio.wait_for(self._seen.wait(), 0.1)
                    except TimeoutError:
                        pass
            self.available = True
            LOG.info("portal request monitor ready")
        except Exception as error:
            LOG.error("portal monitor could not start: %s", error)
            self._report_error("picker", CaptureError("portal_monitor_unavailable", "busctl"))
            await self.close()

    async def _read(self) -> None:
        try:
            while line := await self._process.stdout.readline():
                message = json.loads(line)
                if message.get("interface") == "org.freedesktop.DBus.Properties" and message.get("member") == "Get":
                    self._seen.set()
                if message.get("type") == "method_call" and message.get("interface") == _PORTAL_INTERFACE and message.get("member") == "SelectSources":
                    call = _PortalCall(message["payload"]["data"][1])
                    key = (message["sender"], message["cookie"])
                    self._calls[key] = call
                    self._changed.set()
                    try:
                        # session/<unique_bus_name>/<token> 指向原调用应用；
                        # 转发方法的 sender 则是 xdg-desktop-portal，不能用于识别 GSR。
                        sender = ":" + call.session_handle.split("/")[-2].replace("_", ".")
                        response = await run_command(
                            "busctl", "--user", "--json=short", "--timeout=2", "call",
                            "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus",
                            "GetConnectionUnixProcessID", "s", sender,
                        )
                        call.caller_pid = json.loads(response)["data"][0]
                    except Exception as error:
                        LOG.warning("portal caller lookup failed: %s", error)
                    finally:
                        call.resolved.set()
                elif message.get("type") in ("method_return", "error"):
                    self._calls.pop((message.get("destination"), message.get("reply_cookie")), None)
                    self._changed.set()
            status = await self._process.wait()
            diagnostic = (await self._process.stderr.read()).decode(errors="replace").strip()
            if not self._closing:
                LOG.error("portal monitor exited status=%s: %s", status, diagnostic)
                self._report_error("picker", CaptureError("portal_monitor_failed"))
        except asyncio.CancelledError:
            raise
        except Exception as error:
            if not self._closing:
                self._report_error("picker", error)
        finally:
            self.available = False
            self._changed.set()

    async def caller_pid(self) -> int | None:
        """返回当前同步 picker 的原调用者；无法识别时不关联录制。"""
        if not self.available:
            return None
        try:
            async with asyncio.timeout(2):
                while self.available:
                    if self._calls:
                        call = next(iter(self._calls.values()))
                        await call.resolved.wait()
                        if call in self._calls.values():
                            return call.caller_pid
                    self._changed.clear()
                    await self._changed.wait()
        except TimeoutError:
            LOG.warning("picker caller was not identified")
        return None

    async def close(self) -> None:
        """终止本次 busctl 观察进程，不改 Portal 或其他客户端。"""
        self._closing = True
        self.available = False
        if self._process and self._process.returncode is None:
            self._process.terminate()
        if self._task:
            await asyncio.gather(self._task, return_exceptions=True)
        if self._process:
            await self._process.wait()


@dataclass
class _Picker:
    """保留一次待确认请求，result 送达 socket 或被取消后才结束活动状态。"""

    request_id: str
    windows: list[dict]
    result: asyncio.Future
    caller_pid: int | None = None


class PickerBridge:
    """维护一个待确认的 Portal 请求；socket 断开和 UI 取消均拒绝该请求。"""

    def __init__(self, emit: Callable[..., None], report_error: Callable[..., None], monitor: PortalMonitor, recording) -> None:
        self._emit = emit
        self._report_error = report_error
        self._monitor = monitor
        self._recording = recording
        self._active: _Picker | None = None
        self._connections: set[asyncio.Task] = set()
        self._closing = False

    async def connect(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        """接收 picker 的窗口 token 列表，等待明确确认，不读取全局待选目标。"""
        task = asyncio.current_task()
        self._connections.add(task)
        picker = None
        disconnected = None
        try:
            async with asyncio.timeout(2):
                line = await reader.readline()
            request = json.loads(line)
            if request["type"] != "picker" or not isinstance(request["windows"], list):
                raise CaptureError("invalid_request")
            if self._active or self._closing:
                writer.write(b'{"cancelled":true}\n')
                await writer.drain()
                LOG.info("additional picker rejected")
                return
            windows = request["windows"]
            if any(not all(isinstance(window[key], str) for key in ("id", "address", "title", "appId")) for window in windows):
                raise CaptureError("invalid_request")
            picker = _Picker(uuid4().hex, windows, asyncio.get_running_loop().create_future())
            self._active = picker
            disconnected = asyncio.create_task(reader.read())
            picker.caller_pid = await self._monitor.caller_pid()
            self._recording.picker_opened(picker.request_id, picker.caller_pid)
            self._emit("picker_request", requestId=picker.request_id, windows=windows)
            LOG.info("picker requested windows=%s owned_recording=%s", len(windows), self._recording.owns_picker(picker.request_id))
            completed, _ = await asyncio.wait((picker.result, disconnected), return_when=asyncio.FIRST_COMPLETED)
            if disconnected in completed:
                await self._finish(picker, None)
                return
            response = picker.result.result() if self._active is picker else {"cancelled": True}
            writer.write(json.dumps(response).encode() + b"\n")
            await writer.drain()
            await self._finish(picker, None if response["cancelled"] else response, delivered=True)
        except (BrokenPipeError, ConnectionResetError):
            if picker and self._active is picker:
                await self._finish(picker, None)
        except asyncio.CancelledError:
            if picker and self._active is picker:
                await self._finish(picker, None)
            raise
        except Exception as error:
            self._report_error("picker", error, picker.request_id if picker else None)
            if picker and self._active is picker:
                await self._finish(picker, None)
        finally:
            if disconnected:
                disconnected.cancel()
                await asyncio.gather(disconnected, return_exceptions=True)
            writer.close()
            await writer.wait_closed()
            self._connections.discard(task)

    def _request(self, request_id: str) -> _Picker:
        if not self._active or self._active.request_id != request_id:
            raise CaptureError("unknown_picker")
        return self._active

    async def select(self, request_id: str, selection: dict) -> None:
        """验证本次请求的 token 或显示器内逻辑矩形，返回不带 restore flag 的选择。"""
        picker = self._request(request_id)
        if picker.result.done():
            raise CaptureError("selection_in_progress")
        kind = selection["type"]
        address = None
        if kind == "window":
            window = next((window for window in picker.windows if window["id"] == selection["id"]), None)
            if window is None:
                raise CaptureError("invalid_selection")
            text = f"[SELECTION]/window:{window['id']}"
            address = window["address"]
        elif kind in ("output", "region"):
            monitors = await hyprland_json("monitors")
            monitor = next((item for item in monitors if item["name"] == selection["output"] and not item.get("disabled", False)), None)
            if monitor is None:
                raise CaptureError("invalid_selection")
            if kind == "output":
                text = f"[SELECTION]/screen:{monitor['name']}"
            else:
                x, y, width, height = (selection[key] for key in ("x", "y", "width", "height"))
                monitor_width, monitor_height = monitor["width"], monitor["height"]
                if monitor["transform"] % 2:
                    monitor_width, monitor_height = monitor_height, monitor_width
                if any(type(value) is not int for value in (x, y, width, height)) or min(x, y) < 0 or min(width, height) <= 0:
                    raise CaptureError("invalid_selection")
                if x + width > monitor_width / monitor["scale"] or y + height > monitor_height / monitor["scale"]:
                    raise CaptureError("invalid_selection")
                text = f"[SELECTION]/region:{monitor['name']}@{x},{y},{width},{height}"
        else:
            raise CaptureError("invalid_selection")
        # hyprctl 验证期间，picker 可能已经断开或被停止录制的命令取消。
        if self._active is not picker:
            raise CaptureError("unknown_picker")
        self._recording.picker_selected(request_id, address)
        await self._finish(picker, {"cancelled": False, "selection": text})

    async def cancel(self, request_id: str) -> None:
        """拒绝指定请求，只取消与此请求关联的自有 GSR。"""
        await self._finish(self._request(request_id), None)

    async def _finish(self, picker: _Picker, result: dict | None, *, delivered: bool = False) -> None:
        if self._active is not picker:
            return
        if not picker.result.done():
            picker.result.set_result(result or {"cancelled": True})
        # 确认尚未送进 socket 时仍算活动 picker；断开必须取消，而非报告授权成功。
        if result is not None and not delivered:
            return
        self._active = None
        cancelled = result is None
        self._emit("picker_closed", requestId=picker.request_id, cancelled=cancelled)
        LOG.info("picker closed cancelled=%s", cancelled)
        if cancelled:
            self._recording.picker_cancelled(picker.request_id)

    async def cancel_recording_picker(self, request_id: str | None) -> None:
        """录制停止时只拒绝对应 picker，保留浏览器等其他待确认请求。"""
        if self._active and self._active.request_id == request_id:
            await self._finish(self._active, None)

    async def close(self) -> None:
        """拒绝未确认请求并等待所有 picker 连接关闭。"""
        self._closing = True
        if self._active:
            await self._finish(self._active, None)
        if self._connections:
            await asyncio.gather(*self._connections, return_exceptions=True)
