#!/usr/bin/env python3
"""运行 Quickshell 的截图、Portal picker 和录制后端。

stdin/stdout 使用 NDJSON；诊断仅写 stderr。进程拥有当前 Hyprland 私有 socket、
临时原始截图和自己启动的 GSR。stdin EOF、SIGINT 或 SIGTERM 均等待录制收尾后退出。
"""

from __future__ import annotations

import asyncio
import fcntl
import json
import logging
import os
import shutil
import signal
import sys
import tempfile
from pathlib import Path

from common import CaptureError, LOG, session_directory, socket_path
from portal import PickerBridge, PortalMonitor
from recording import Recording
from screenshots import Screenshots


def _discard_disconnected_output(stream) -> None:
    """把已断开读端的输出转到 /dev/null，避免退出 flush 再次触发 BrokenPipe。"""
    with open(os.devnull, "w") as sink:
        os.dup2(sink.fileno(), stream.fileno())


class _CaptureLogHandler(logging.StreamHandler):
    """在 UI 关闭 stderr 后停止向该管道写日志，不中断录制保存或诊断读取。"""

    def handleError(self, record: logging.LogRecord) -> None:
        """只忽略已断开日志管道的 BrokenPipe；其他日志错误沿用标准处理。"""
        if isinstance(sys.exception(), BrokenPipeError):
            _discard_disconnected_output(self.stream)
        else:
            super().handleError(record)


class CaptureServer:
    """执行 UI 命令并维护单个捕获会话，不负责界面文案或自动授权。"""

    def __init__(self) -> None:
        self._shutdown = asyncio.Event()
        self._jobs: set[asyncio.Task] = set()
        self._screenshots: Screenshots | None = None
        self._monitor = PortalMonitor(self.report_error)
        self._recording = Recording(self.emit, self.report_error, self._monitor)
        self._picker = PickerBridge(self.emit, self.report_error, self._monitor, self._recording)
        self._recording.set_picker_canceller(self._picker.cancel_recording_picker)

    def emit(self, event_type: str, **fields) -> None:
        """输出一行结构化事件；父进程关闭 stdout 时转入正常收尾。"""
        try:
            print(json.dumps({"type": event_type, **fields}, ensure_ascii=False, separators=(",", ":")), flush=True)
        except BrokenPipeError:
            _discard_disconnected_output(sys.stdout)
            self._shutdown.set()

    def report_error(self, operation: str, error: Exception, request_id: str | None = None) -> None:
        """记录内部诊断，向界面仅发 code、operation 和可选关联字段。"""
        if isinstance(error, CaptureError):
            code = error.code
        elif isinstance(error, (KeyError, ValueError, TypeError)):
            code = "invalid_command"
        elif isinstance(error, OSError):
            code = "io_failed"
        else:
            code = "operation_failed"
        LOG.error("operation=%s code=%s: %s", operation, code, error, exc_info=not isinstance(error, CaptureError))
        fields = {"operation": operation, "code": code}
        if request_id is not None:
            fields["requestId"] = request_id
        if isinstance(error, CaptureError) and error.dependency:
            fields["dependency"] = error.dependency
        self.emit("error", **fields)

    def _dispatch(self, command: dict) -> None:
        operation = command["type"]
        if not isinstance(operation, str):
            raise CaptureError("invalid_command")
        request_id = command.get("requestId")
        if operation in ("snapshot", "release", "export", "picker_select", "picker_cancel"):
            if not isinstance(request_id, str) or not request_id:
                raise CaptureError("invalid_command")
        if operation == "snapshot":
            self._screenshots.snapshot(request_id)
        elif operation == "record_prepare":
            self._recording.prepare(command)
        elif operation in ("release", "export", "copy_color", "picker_select", "picker_cancel", "stop_recording"):
            task = asyncio.create_task(self._execute(command))
            self._jobs.add(task)
            task.add_done_callback(self._jobs.discard)
        else:
            raise CaptureError("unknown_command")

    async def _execute(self, command: dict) -> None:
        operation = command["type"]
        try:
            if operation == "release":
                await self._screenshots.release(command["requestId"])
            elif operation == "export":
                await self._screenshots.export(command)
            elif operation == "copy_color":
                await self._screenshots.copy_color(command["value"])
            elif operation == "picker_select":
                await self._picker.select(command["requestId"], command["selection"])
            elif operation == "picker_cancel":
                await self._picker.cancel(command["requestId"])
            elif operation == "stop_recording":
                await self._recording.stop()
        except Exception as error:
            self.report_error(operation, error, command.get("requestId"))

    async def _read_input(self) -> None:
        reader = asyncio.StreamReader(limit=sys.maxsize)
        protocol = asyncio.StreamReaderProtocol(reader)
        transport, _ = await asyncio.get_running_loop().connect_read_pipe(lambda: protocol, sys.stdin.buffer)
        try:
            while line := await reader.readline():
                command = None
                try:
                    command = json.loads(line)
                    if not isinstance(command, dict):
                        raise CaptureError("invalid_command")
                    self._dispatch(command)
                except Exception as error:
                    operation = command.get("type", "command") if isinstance(command, dict) else "command"
                    request_id = command.get("requestId") if isinstance(command, dict) else None
                    self.report_error(operation if isinstance(operation, str) else "command", error, request_id if isinstance(request_id, str) else None)
            LOG.info("stdin closed; capture shutdown requested")
        finally:
            transport.close()
            self._shutdown.set()

    async def _watch_windows(self) -> None:
        writer = None
        try:
            reader, writer = await asyncio.open_unix_connection(session_directory() / ".socket2.sock")
            while line := await reader.readline():
                text = line.decode().strip()
                if text.startswith("closewindow>>"):
                    await self._recording.window_closed(text.split(">>", 1)[1])
            LOG.info("Hyprland event socket closed; capture shutdown requested")
            self._shutdown.set()
        except asyncio.CancelledError:
            raise
        except Exception as error:
            self.report_error("recording", error)
        finally:
            if writer:
                writer.close()
                await writer.wait_closed()

    async def run(self) -> None:
        """持有实例锁直到停止保存和截图清理完成，防止重载覆盖旧 socket。"""
        loop = asyncio.get_running_loop()
        for signum in (signal.SIGTERM, signal.SIGINT):
            loop.add_signal_handler(signum, self._shutdown.set)
        path = socket_path()
        root = None
        socket_server = None
        input_task = None
        windows_task = None
        with path.with_name(path.name + ".lock").open("a") as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise CaptureError("server_already_running") from error
            # 持锁后才可删除旧 socket。旧 Quickshell 后端会一直持锁到 GSR 保存完毕，
            # 避免重载时新后端抢走旧进程仍在使用的 socket。
            path.unlink(missing_ok=True)
            try:
                root = Path(tempfile.mkdtemp(prefix="dots-capture-", dir=session_directory()))
                self._screenshots = Screenshots(root, self.emit, self.report_error)
                await self._monitor.start()
                socket_server = await asyncio.start_unix_server(self._picker.connect, path=path, limit=sys.maxsize)
                path.chmod(0o600)
                input_task = asyncio.create_task(self._read_input())
                windows_task = asyncio.create_task(self._watch_windows())
                self.emit("ready", dependencies={
                    "grim": shutil.which("grim") is not None,
                    "wlCopy": shutil.which("wl-copy") is not None,
                    "gpuScreenRecorder": shutil.which("gpu-screen-recorder") is not None,
                })
                LOG.info("capture server ready")
                await self._shutdown.wait()
            finally:
                LOG.info("capture server stopping")
                if socket_server:
                    socket_server.close()
                if input_task:
                    input_task.cancel()
                    await asyncio.gather(input_task, return_exceptions=True)
                # wait_closed 也等待客户端断开；必须先拒绝活动 picker 并关闭连接。
                await self._picker.close()
                if socket_server:
                    await socket_server.wait_closed()
                await self._recording.stop()
                if self._jobs:
                    await asyncio.gather(*self._jobs, return_exceptions=True)
                if self._screenshots:
                    await self._screenshots.close()
                if windows_task:
                    windows_task.cancel()
                    await asyncio.gather(windows_task, return_exceptions=True)
                await self._monitor.close()
                path.unlink(missing_ok=True)
                if root:
                    shutil.rmtree(root, ignore_errors=True)
                LOG.info("capture server stopped")


def main() -> int:
    """启动服务；只对本进程新建文件设置私有权限。"""
    os.umask(0o077)
    logging.basicConfig(
        level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s %(message)s",
        handlers=[_CaptureLogHandler(sys.stderr)],
    )
    server = CaptureServer()
    try:
        asyncio.run(server.run())
        return 0
    except Exception as error:
        server.report_error("startup", error)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
