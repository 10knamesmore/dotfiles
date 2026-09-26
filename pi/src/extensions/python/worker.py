"""Run persistent Python cells over dedicated control and event file descriptors."""

import code
from contextlib import chdir
from importlib import metadata
import io
import json
import linecache
import os
import platform
import signal
import sys
import types


class SessionInterpreter(code.InteractiveInterpreter):
    """Execute complete cells in a persistent main module and report failures."""

    def __init__(self) -> None:
        """Create the user namespace and its compiler state."""
        main = types.ModuleType("__main__")
        sys.modules["__main__"] = main
        super().__init__(main.__dict__)
        self.outcome = "completed"

    def runsource(self, source: str, filename: str = "<input>", symbol: str = "exec") -> bool:
        """Reject incomplete cells without retaining them for the next call."""
        if super().runsource(source, filename, symbol):
            try:
                raise SyntaxError("incomplete input", (filename, len(source.splitlines()) or 1, 1, ""))
            except SyntaxError:
                self.showsyntaxerror(filename)
        return False

    def showsyntaxerror(self, filename: str | None = None) -> None:
        """Mark a compilation failure while preserving the interpreter's traceback."""
        self.outcome = "python_error"
        super().showsyntaxerror(filename)

    def showtraceback(self) -> None:
        """Mark a runtime failure while preserving the interpreter's traceback."""
        self.outcome = "python_error"
        super().showtraceback()

    def runcode(self, compiled: types.CodeType) -> None:
        """Keep SystemExit and KeyboardInterrupt from terminating the session."""
        try:
            exec(compiled, self.locals)
        except KeyboardInterrupt:
            self.outcome = "interrupted"
            super().showtraceback()
        except BaseException:
            self.showtraceback()


def send_event(event: dict[str, object]) -> None:
    """Write a complete JSON event to the dedicated event descriptor."""
    frame = (json.dumps(event, ensure_ascii=False) + "\n").encode("utf-8")
    while frame:
        frame = frame[os.write(4, frame):]


executing = False


def handle_sigint(_signum: int, _frame: types.FrameType | None) -> None:
    """Interrupt the active cell without letting idle signals stop the reader."""
    if executing:
        raise KeyboardInterrupt


def execute_cell(interpreter: SessionInterpreter, request: dict[str, object], number: int) -> None:
    """Capture one cell's output, restore its working directory, and report completion."""
    global executing
    call_id = request["callId"]
    source = request["code"]
    output_path = request["outputPath"]
    cwd = request["cwd"]
    assert isinstance(source, str) and isinstance(output_path, str) and isinstance(cwd, str)

    filename = f"<python-{number}>"
    linecache.cache[filename] = (len(source), None, source.splitlines(keepends=True), filename)
    original_stdout, original_stderr = sys.stdout, sys.stderr
    original_stdout.flush()
    original_stderr.flush()
    saved_stdout, saved_stderr = os.dup(1), os.dup(2)
    output_fd = os.open(output_path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
    try:
        os.dup2(output_fd, 1)
        os.dup2(output_fd, 2)
        stdout = io.TextIOWrapper(os.fdopen(os.dup(1), "wb", buffering=0), encoding="utf-8", errors="backslashreplace", write_through=True)
        stderr = io.TextIOWrapper(os.fdopen(os.dup(2), "wb", buffering=0), encoding="utf-8", errors="backslashreplace", write_through=True)
        sys.stdout, sys.stderr = stdout, stderr
        interpreter.outcome = "completed"
        try:
            executing = True
            send_event({"type": "started", "callId": call_id})
            with chdir(cwd):
                interpreter.runsource(source, filename, symbol="exec")
        except (OSError, ValueError):
            interpreter.showtraceback()
        except KeyboardInterrupt:
            interpreter.outcome = "interrupted"
            interpreter.showtraceback()
            interpreter.outcome = "interrupted"
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
        os.dup2(saved_stdout, 1)
        os.dup2(saved_stderr, 2)
        os.close(saved_stdout)
        os.close(saved_stderr)
        os.close(output_fd)
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
    signal.signal(signal.SIGINT, handle_sigint)
    # An empty import path follows each call's cwd instead of pinning the startup directory.
    sys.path.insert(0, "")
    interpreter = SessionInterpreter()
    send_event({"type": "ready", "pid": os.getpid(), "environment": describe_environment()})
    with os.fdopen(3, "rb") as requests:
        for number, frame in enumerate(requests, start=1):
            request = json.loads(frame.decode("utf-8"))
            execute_cell(interpreter, request, number)


if __name__ == "__main__":
    if sys.argv[1:] == ["--describe-environment"]:
        print(json.dumps(describe_environment(), ensure_ascii=False))
    else:
        main()
