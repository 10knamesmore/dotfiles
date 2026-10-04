"""Own only SDK-launched Chrome groups and report them to Pi's worker supervisor."""

import asyncio
from collections.abc import Callable
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import threading
from uuid import uuid4

from ._diagnostics import event
from ._discovery import endpoint_from_file, installed_executable

_hook: Callable[[dict[str, object]], None] | None = None
_hook_thread: int | None = None


def _set_lifecycle_hook(
    callback: Callable[[dict[str, object]], None] | None = None,
) -> None:
    """Register the worker's process-ownership sink before any browser is launched."""
    global _hook, _hook_thread
    _hook = callback
    _hook_thread = threading.get_ident() if callback is not None else None


def check_thread() -> None:
    if _hook_thread is not None and threading.get_ident() != _hook_thread:
        raise RuntimeError(
            "Browser lifecycle operations must run on the Python worker thread"
        )


def _notify(action: str, identity: str, pid: int) -> None:
    check_thread()
    if _hook is not None:
        _hook({"action": action, "id": identity, "pid": pid})
    event("process_ownership", action, id=identity, pid=pid)


class OwnedBrowser:
    """Supervise a separate Chrome process group, leaving explicit profiles on disk."""

    def __init__(
        self,
        executable: str | Path | None,
        directory: str | Path | None,
        headless: bool,
    ) -> None:
        check_thread()
        if directory is None:
            self._profile = tempfile.TemporaryDirectory(prefix="browser-use-profile-")
            self.directory = Path(self._profile.name)
        else:
            self._profile = None
            self.directory = Path(directory).absolute()
        self.id = uuid4().hex
        self._closed = False
        try:
            self.directory.mkdir(parents=True, exist_ok=True)
            path = self.directory / "DevToolsActivePort"
            self._previous_endpoint = path.read_text() if path.exists() else None
            arguments = [
                str(executable) if executable is not None else installed_executable(),
                "--remote-debugging-port=0",
                "--remote-debugging-address=127.0.0.1",
                "--no-first-run",
                "--no-default-browser-check",
                f"--user-data-dir={self.directory}",
            ]
            if headless:
                arguments.append("--headless=new")
            arguments.append("about:blank")
            self.process = subprocess.Popen(
                arguments,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
        except BaseException:
            if self._profile is not None:
                self._profile.cleanup()
            raise
        try:
            # Register before waiting for CDP, so Pi can clean up interrupted startup.
            _notify("opened", self.id, self.process.pid)
        except BaseException:
            self.close()
            raise

    async def endpoint(self) -> str:
        """Wait for this launch's endpoint; a reused profile's previous file is not readiness."""
        path = self.directory / "DevToolsActivePort"
        while self.process.poll() is None:
            try:
                contents = path.read_text()
            except FileNotFoundError:
                contents = None
            if contents is not None and contents != self._previous_endpoint:
                return endpoint_from_file(path)
            await asyncio.sleep(0.025)
        raise RuntimeError(
            f"Chrome exited before publishing its CDP endpoint (code {self.process.returncode})"
        )

    def close(self) -> None:
        """Terminate and reap the owned group, then release ownership and temporary storage."""
        check_thread()
        if self._closed:
            return
        if self.process.poll() is None:
            try:
                os.killpg(self.process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            self.process.wait()
        _notify("closed", self.id, self.process.pid)
        self._closed = True
        if self._profile is not None:
            self._profile.cleanup()
