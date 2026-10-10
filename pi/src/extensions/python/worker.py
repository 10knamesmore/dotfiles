"""Run persistent Python cells over dedicated control and event file descriptors."""

import code
from collections.abc import Callable
from contextlib import chdir
from dataclasses import dataclass
from functools import partial
from importlib import metadata
import io
import json
import linecache
import os
import platform
import signal
import sys
import threading
import types
from typing import Literal, cast, override


WORKER_REQUEST_FD = 3
WORKER_EVENT_FD = 4


class SessionInterpreter(code.InteractiveInterpreter):
    """Execute complete cells in a persistent main module and report failures."""

    def __init__(self) -> None:
        """Create the user namespace and its compiler state."""
        main = types.ModuleType("__main__")
        sys.modules["__main__"] = main
        super().__init__(main.__dict__)
        self.outcome: Literal["completed", "python_error", "interrupted"] = "completed"

    @override
    def runsource(self, source: str, filename: str = "<input>", symbol: str = "exec") -> bool:
        """Reject incomplete cells without retaining them for the next call."""
        if super().runsource(source, filename, symbol):
            try:
                raise SyntaxError("incomplete input", (filename, len(source.splitlines()) or 1, 1, ""))
            except SyntaxError:
                self.showsyntaxerror(filename)
        return False

    @override
    def showsyntaxerror(self, filename: str | None = None, **kwargs: str) -> None:
        """Mark a compilation failure while preserving the interpreter's traceback."""
        self.outcome = "python_error"
        super().showsyntaxerror(filename, **kwargs)

    @override
    def showtraceback(self) -> None:
        """Mark a runtime failure while preserving the interpreter's traceback."""
        self.outcome = "python_error"
        super().showtraceback()

    @override
    def runcode(self, code: types.CodeType) -> None:
        """Keep SystemExit and KeyboardInterrupt from terminating the session."""
        try:
            exec(code, self.locals)
        except KeyboardInterrupt:
            self.outcome = "interrupted"
            super().showtraceback()
        except BaseException:
            self.showtraceback()


def send_event(event: dict[str, object]) -> None:
    """Write a complete JSON event to the dedicated event descriptor."""
    frame = (json.dumps(event, ensure_ascii=False) + "\n").encode("utf-8")
    while frame:
        frame = frame[os.write(WORKER_EVENT_FD, frame):]


def handle_process_ownership(kind: Literal["terminal", "browser"], event: dict[str, object]) -> None:
    """Forward an SDK process-group ownership change without terminal or browser content."""
    action = event.get("action")
    session_id = event.get("id")
    pid = event.get("pid")
    if action not in ("opened", "closed") or not isinstance(session_id, str) or not session_id:
        raise ValueError(f"malformed {kind} ownership event: {event!r}")
    if isinstance(pid, bool) or not isinstance(pid, int) or pid <= 0:
        raise ValueError(f"{kind} ownership event for {session_id!r} has no positive pid")
    send_event({"type": "process_ownership", "kind": kind, "action": action, "id": session_id, "pid": pid})


def install_process_ownership_hooks() -> None:
    """Register required SDK process groups with the controller before running user code."""
    try:
        from terminal_use import _set_lifecycle_hook as terminal_hook, _set_worker_control_fds
        from browser_use import _set_lifecycle_hook as browser_hook
    except ImportError as error:
        reason = f"a required process-owning Python SDK is not importable: {error}"
        try:
            send_event({"type": "startup_error", "reason": reason})
        except OSError:
            # The event pipe may already be gone; stderr still carries the reason.
            pass
        raise SystemExit(reason) from error
    _set_worker_control_fds((WORKER_REQUEST_FD, WORKER_EVENT_FD))
    terminal_hook(partial(handle_process_ownership, "terminal"))
    browser_hook(partial(handle_process_ownership, "browser"))


@dataclass
class CellImageOutput:
    """Keep image output tied to one active cell and its executing thread."""

    call_id: str
    directory: str
    thread_id: int
    sequence: int = 0


class CellRuntime:
    """Bind image output and Linux desktop cleanup to the worker's active cell."""

    def __init__(self, interpreter: SessionInterpreter) -> None:
        """Load required SDKs before user imports and expose native REPL helpers."""
        self.close_desktops: Callable[[], None] | None = None
        try:
            from pi_repl_tools import _make_summarize, _set_image_sink, display_image
            if sys.platform == "linux":
                from computer_use import _close_all
                self.close_desktops = _close_all
        except ImportError as error:
            reason = f"a required Python worker SDK is not importable: {error}"
            send_event({"type": "startup_error", "reason": reason})
            raise SystemExit(reason) from error
        self.active_cell: CellImageOutput | None = None
        self.pid = os.getpid()
        _set_image_sink(self.write_image)
        interpreter.locals["display_image"] = display_image
        interpreter.locals["summarize"] = _make_summarize(interpreter.locals)

    def begin_cell(self, call_id: str, output_path: str) -> None:
        """Start an image sequence in the controller's output directory for this cell."""
        self.active_cell = CellImageOutput(call_id, os.path.dirname(output_path), threading.get_ident())

    def write_image(self, data: bytes, mime_type: str) -> None:
        """Store encoded bytes before announcing their path; reject background output."""
        cell = self.active_cell
        if cell is None or cell.thread_id != threading.get_ident() or self.pid != os.getpid():
            raise RuntimeError("display_image() must run on the active Python cell's thread")
        extensions = {"image/png": "png", "image/jpeg": "jpg", "image/gif": "gif", "image/webp": "webp"}
        extension = extensions.get(mime_type)
        if extension is None:
            raise ValueError("display_image() returned an unsupported image format")
        cell.sequence += 1
        path = os.path.join(cell.directory, f"image-{cell.sequence}.{extension}")
        with open(path, "xb") as image:
            _ = image.write(data)
        send_event({"type": "image", "callId": cell.call_id, "path": path, "mimeType": mime_type})

    def finish_cell(self, failed: bool) -> str | None:
        """Forget image ownership and report cleanup failure without replacing the cell outcome."""
        self.active_cell = None
        if failed and self.close_desktops is not None:
            try:
                self.close_desktops()
            except BaseException as error:
                return type(error).__name__
        return None


executing = False


def handle_sigint(_signum: int, _frame: types.FrameType | None) -> None:
    """Interrupt the active cell without letting idle signals stop the reader."""
    if executing:
        raise KeyboardInterrupt


def execute_cell(
    interpreter: SessionInterpreter, request: dict[str, object], number: int, runtime: CellRuntime
) -> None:
    """Capture one cell's output, restore its working directory, and report completion."""
    global executing
    call_id = request["callId"]
    source = request["code"]
    output_path = request["outputPath"]
    cwd = request["cwd"]
    assert isinstance(call_id, str) and isinstance(source, str) and isinstance(output_path, str) and isinstance(cwd, str)

    filename = f"<python-{number}>"
    linecache.cache[filename] = (len(source), None, source.splitlines(keepends=True), filename)
    original_stdout, original_stderr = sys.stdout, sys.stderr
    _ = original_stdout.flush()
    _ = original_stderr.flush()
    saved_stdout, saved_stderr = os.dup(1), os.dup(2)
    output_fd = os.open(output_path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
    try:
        _ = os.dup2(output_fd, 1)
        _ = os.dup2(output_fd, 2)
        stdout = io.TextIOWrapper(os.fdopen(os.dup(1), "wb", buffering=0), encoding="utf-8", errors="backslashreplace", write_through=True)
        stderr = io.TextIOWrapper(os.fdopen(os.dup(2), "wb", buffering=0), encoding="utf-8", errors="backslashreplace", write_through=True)
        sys.stdout, sys.stderr = stdout, stderr
        interpreter.outcome = "completed"
        runtime.begin_cell(call_id, output_path)
        input_cleanup_error: str | None = None
        try:
            executing = True
            send_event({"type": "started", "callId": call_id})
            with chdir(cwd):
                _ = interpreter.runsource(source, filename, symbol="exec")
        except (OSError, ValueError):
            interpreter.showtraceback()
        except KeyboardInterrupt:
            interpreter.outcome = "interrupted"
            interpreter.showtraceback()
            interpreter.outcome = "interrupted"
        finally:
            try:
                input_cleanup_error = runtime.finish_cell(interpreter.outcome != "completed")
            finally:
                executing = False
            for stream in (stdout, stderr):
                try:
                    stream.flush()
                except (OSError, ValueError):
                    pass
            sys.stdout, sys.stderr = original_stdout, original_stderr
            stdout.close()
            stderr.close()
    finally:
        _ = os.dup2(saved_stdout, 1)
        _ = os.dup2(saved_stderr, 2)
        os.close(saved_stdout)
        os.close(saved_stderr)
        os.close(output_fd)
    if input_cleanup_error is not None:
        send_event({"type": "input_cleanup_failed", "callId": call_id, "errorType": input_cleanup_error})
    send_event({"type": "completed", "callId": call_id, "outcome": interpreter.outcome})


def describe_environment() -> dict[str, object]:
    """Report this interpreter and all installed distributions, including transitive dependencies."""
    packages = sorted(
        ({"name": distribution.metadata["Name"], "version": distribution.version} for distribution in metadata.distributions()),
        key=lambda package: package["name"].casefold(),
    )
    return {"executable": sys.executable, "version": platform.python_version(), "packages": packages}


def main() -> None:
    """Read one LF-delimited request at a time until the controller closes fd3."""
    os.setpgid(0, 0)
    _ = signal.signal(signal.SIGINT, handle_sigint)
    # Resolve native SDKs before user code can shadow them from the call cwd.
    install_process_ownership_hooks()
    interpreter = SessionInterpreter()
    runtime = CellRuntime(interpreter)
    # An empty import path follows each call's cwd instead of pinning the startup directory.
    sys.path.insert(0, "")
    send_event({"type": "ready", "pid": os.getpid(), "environment": describe_environment()})
    with os.fdopen(WORKER_REQUEST_FD, "rb") as requests:
        for number, frame in enumerate(requests, start=1):
            request = cast(dict[str, object], json.loads(frame.decode("utf-8")))
            execute_cell(interpreter, request, number, runtime)


if __name__ == "__main__":
    if sys.argv[1:] == ["--describe-environment"]:
        print(json.dumps(describe_environment(), ensure_ascii=False))
    else:
        main()
