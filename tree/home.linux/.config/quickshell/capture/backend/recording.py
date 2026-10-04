"""管理自己启动的 GSR、首帧确认和 MP4 收尾。

仅使用 -w portal；选择来源由独立 picker 逐次确认。停止只向持有的 PID 发信号，
不寻找或修改其他录屏进程。没有首帧的取消不保留文件，也不报告已保存。
"""

from __future__ import annotations

import asyncio
import shutil
import signal
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

from common import CaptureError, LOG, export_directory, export_filename


@dataclass
class _Recording:
    """保存一次录制的生命周期，picker_id 仅在原调用者 PID 匹配时赋值。

    started_at 来自 GSR 首帧侧文件，单位为 epoch 秒。selection_time 为用户确认
    时的 monotonic 秒，只用于首帧超时；cancelled 标记首帧确认前的主动停止。
    """

    options: dict
    path: Path | None = None
    process: asyncio.subprocess.Process | None = None
    task: asyncio.Task | None = None
    picker_id: str | None = None
    window_address: str | None = None
    started_at: float | None = None
    selection_time: float | None = None
    stop_requested: bool = False
    cancelled: bool = False
    failure: str | None = None
    wake: asyncio.Event = field(default_factory=asyncio.Event)


class Recording:
    """维持至多一份自有录制，只有时间戳侧文件确认首帧后才报告 recording。"""

    def __init__(self, emit: Callable[..., None], report_error: Callable[..., None], monitor) -> None:
        self._emit = emit
        self._report_error = report_error
        self._monitor = monitor
        self._active: _Recording | None = None
        self._cancel_picker: Callable | None = None

    def set_picker_canceller(self, cancel_picker: Callable) -> None:
        """接入 picker 的拒绝操作，用于停止尚在选择来源的录制。"""
        self._cancel_picker = cancel_picker

    def prepare(self, options: dict) -> None:
        """预留本次录制并启动任务，音频选项确定后由 GSR 请求 Portal picker。"""
        if self._active:
            raise CaptureError("recording_in_progress")
        if shutil.which("gpu-screen-recorder") is None:
            self._emit("recording", state="error")
            raise CaptureError("missing_dependency", "gpu-screen-recorder")
        if not self._monitor.available:
            self._emit("recording", state="error")
            raise CaptureError("portal_monitor_unavailable", "busctl")
        if any(type(options[key]) is not bool for key in ("systemAudio", "microphone", "cursor")):
            raise CaptureError("invalid_command")
        recording = _Recording(options)
        self._active = recording
        recording.task = asyncio.create_task(self._run(recording))

    def owns_picker(self, request_id: str) -> bool:
        """判断请求是否已通过 D-Bus 原调用者 PID 关联当前自有 GSR。"""
        return self._active is not None and self._active.picker_id == request_id

    def picker_opened(self, request_id: str, caller_pid: int | None) -> None:
        """关联属于 GSR 的请求；浏览器等其他请求不改变录制状态。"""
        recording = self._active
        if recording and recording.process and recording.process.returncode is None and caller_pid == recording.process.pid:
            recording.picker_id = request_id
            recording.wake.set()

    def picker_selected(self, request_id: str, address: str | None) -> None:
        """在本次 GSR 来源被明确选择后进入 starting，记录窗口关闭监听地址。"""
        if not self.owns_picker(request_id):
            return
        recording = self._active
        if recording.stop_requested:
            raise CaptureError("recording_cancelled")
        recording.window_address = address if address and int(address, 16) != 0 else None
        recording.selection_time = time.monotonic()
        recording.wake.set()
        self._emit("recording", state="starting")
        LOG.info("recording source selected window=%s", address is not None)

    def picker_cancelled(self, request_id: str) -> None:
        """只取消拥有此请求的录制，不把普通 Portal 拒绝当作录屏操作。"""
        if self.owns_picker(request_id):
            self._request_stop(self._active)

    def _request_stop(self, recording: _Recording) -> None:
        if not recording.stop_requested:
            recording.cancelled = recording.started_at is None
            recording.stop_requested = True
            recording.wake.set()
            LOG.info("recording stop requested before_first_frame=%s", recording.cancelled)

    async def stop(self) -> None:
        """停止自有 GSR 并等待保存；selecting/starting 中停止视为无成片的取消。"""
        recording = self._active
        if recording is None:
            return
        self._request_stop(recording)
        await recording.task

    async def window_closed(self, address: str) -> None:
        """在 Hyprland 关闭选定窗口时停止录制，不轮询或逐帧运行 hyprctl。"""
        recording = self._active
        if recording and recording.window_address and int(recording.window_address, 16) == int(address, 16):
            LOG.info("recorded window closed")
            self._request_stop(recording)

    async def _run(self, recording: _Recording) -> None:
        """启动并回收 GSR，只有正常退出且非空成片才报告最终路径。"""
        waiter = None
        diagnostics = None
        try:
            directory = await export_directory("VIDEOS", "Recordings")
            recording.path = directory / export_filename("Recording", "mp4")
            if recording.stop_requested:
                self._emit("recording", state="idle")
                return
            args = [
                "gpu-screen-recorder", "-w", "portal", "-restore-portal-session", "no",
                "-k", "h264", "-ac", "aac", "-c", "mp4", "-f", "60",
                "-cursor", "yes" if recording.options["cursor"] else "no",
                "-write-first-frame-ts", "yes", "-o", str(recording.path),
            ]
            audio = []
            if recording.options["systemAudio"]:
                audio.append("default_output")
            if recording.options["microphone"]:
                audio.append("default_input")
            if audio:
                args.extend(("-a", "|".join(audio)))
            # GSR 只向 server 持有的管道写诊断，UI 退出不能使它因 SIGPIPE 丢失成片。
            recording.process = await asyncio.create_subprocess_exec(
                *args, stdin=asyncio.subprocess.DEVNULL,
                stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.PIPE,
            )
            diagnostics = asyncio.create_task(self._read_diagnostics(recording.process.stderr))
            waiter = asyncio.create_task(recording.process.wait())
            self._emit("recording", state="selecting")
            LOG.info("recording process started pid=%s", recording.process.pid)
            await self._watch(recording, waiter)
            status = await waiter
            LOG.info("recording process exited status=%s", status)
            started_at = _first_frame_time(recording.path)
            if recording.started_at is None and started_at is not None and not recording.cancelled:
                if recording.selection_time is None:
                    raise CaptureError("picker_not_identified")
                recording.started_at = started_at
            if recording.failure:
                raise CaptureError(recording.failure)
            if recording.cancelled:
                self._discard(recording)
                self._emit("recording", state="idle")
                LOG.info("recording cancelled without saved video")
            elif status != 0:
                raise CaptureError("recorder_exited")
            elif recording.started_at is None:
                raise CaptureError("first_frame_missing")
            elif not recording.path.exists() or recording.path.stat().st_size == 0:
                raise CaptureError("empty_recording")
            else:
                self._emit("recording", state="idle", path=str(recording.path))
                LOG.info("recording saved bytes=%s", recording.path.stat().st_size)
        except Exception as error:
            if recording.process and recording.process.returncode is None:
                recording.process.kill()
                await recording.process.wait()
            self._discard(recording)
            self._emit("recording", state="error")
            if isinstance(error, FileNotFoundError) and recording.path is not None:
                error = CaptureError("missing_dependency", "gpu-screen-recorder")
            self._report_error("record_prepare" if recording.process is None else "recording", error)
        finally:
            if waiter:
                await waiter
            if diagnostics:
                await diagnostics
            if recording.path:
                recording.path.with_name(recording.path.name + ".ts").unlink(missing_ok=True)
            if self._cancel_picker:
                await self._cancel_picker(recording.picker_id)
            self._active = None

    async def _read_diagnostics(self, stream: asyncio.StreamReader) -> None:
        """持续消费 GSR stderr 到 EOF；UI 关闭日志管道后仍保持读取直到录制退出。"""
        while line := await stream.readline():
            LOG.info("gpu-screen-recorder: %s", line.decode(errors="replace").rstrip())

    async def _watch(self, recording: _Recording, waiter: asyncio.Task) -> None:
        """等待首帧时间戳或停止指令；启动失败和收尾超时只处理此子进程。"""
        launched_at = time.monotonic()
        stop_time = None
        while not waiter.done():
            recording.wake.clear()
            now = time.monotonic()
            if not recording.stop_requested and recording.started_at is None:
                started_at = _first_frame_time(recording.path)
                if started_at is not None:
                    if recording.selection_time is None:
                        recording.failure = "picker_not_identified"
                        self._request_stop(recording)
                    else:
                        recording.started_at = started_at
                        self._emit("recording", state="recording", startedAt=started_at)
                        LOG.info("recording first frame confirmed")
                elif recording.selection_time is not None and now - recording.selection_time > 15:
                    recording.failure = "first_frame_timeout"
                    self._request_stop(recording)
                elif recording.picker_id is None and now - launched_at > 30:
                    recording.failure = "picker_not_started"
                    self._request_stop(recording)
            if recording.stop_requested and stop_time is None:
                if self._cancel_picker:
                    await self._cancel_picker(recording.picker_id)
                if recording.process.returncode is None:
                    recording.process.send_signal(signal.SIGINT)
                stop_time = now
                if not recording.cancelled:
                    self._emit("recording", state="saving")
                LOG.info("recording SIGINT sent")
            if stop_time is not None and now - stop_time > (10 if recording.cancelled else 30):
                recording.failure = "recorder_stop_timeout"
                if recording.process.returncode is None:
                    recording.process.kill()
                break
            try:
                await asyncio.wait_for(recording.wake.wait(), 0.1)
            except TimeoutError:
                pass

    def _discard(self, recording: _Recording) -> None:
        if recording.path:
            recording.path.unlink(missing_ok=True)


def _first_frame_time(path: Path) -> float | None:
    """读取 GSR 的 .ts 表格，将 realtime_microsec 转成 epoch 秒。

    文件可能尚未写完，只有完整数字行才算首帧；不使用进程启动时间替代。
    格式见 gpu-screen-recorder(1) 的 -write-first-frame-ts。
    """
    sidecar = path.with_name(path.name + ".ts")
    if not sidecar.exists():
        return None
    content = sidecar.read_text()
    if not content.endswith("\n"):
        return None
    lines = content.splitlines()
    if len(lines) < 2:
        return None
    values = lines[1].split()
    if len(values) != 2 or not all(value.isdecimal() for value in values):
        return None
    return int(values[1]) / 1_000_000
