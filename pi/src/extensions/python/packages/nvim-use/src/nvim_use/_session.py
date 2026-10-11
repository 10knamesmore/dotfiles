"""Own exclusive Neovim attachments while leaving editing to native pynvim."""

import atexit
import os
import threading
import time
from collections.abc import Callable, Generator
from concurrent.futures import Future
from contextlib import contextmanager
from typing import Any, Self, TypeVar

from pynvim import Nvim

from ._connection import Connection, bridge, seconds
from ._diagnostics import event
from ._discovery import resolve
from ._lsp import Lsp

T = TypeVar("T")
_identity: dict[str, Any] | None = None
_sessions: dict[str, "Session"] = {}


def _configure_worker(session_id: str, cwd: str) -> None:
    """Bind connections to the live Pi session before any user Python cells execute."""
    global _identity
    _identity = {
        "session_id": session_id,
        "cwd": cwd,
        "pid": os.getpid(),
        "model": None,
    }


def _set_model(model: dict[str, str] | None) -> None:
    """Publish Pi's selected model to current attachments and future handshakes."""
    if _identity is None:
        raise RuntimeError("nvim-use must be initialized by the Pi Python worker")
    _identity["model"] = model
    for session in list(_sessions.values()):
        if not session.closed:
            session._update_model(model)


class Session:
    """Keep an exclusive attachment alive across Python cells; create with connect().

    run() callbacks receive native pynvim.Nvim on a persistent RPC thread. Native
    handles can be retained, but their remote operations must stay in callbacks.
    disconnect() leaves Neovim, its buffers and language servers running.
    """

    def __init__(
        self, record: dict[str, Any], identity: dict[str, Any], timeout: float
    ) -> None:
        self.instance_id: str = record["instance_id"]
        self._notification_handler: Callable[[Nvim, str, list[Any]], None] | None = None
        self._responses: dict[str, Future[Any]] = {}
        self._responses_lock = threading.Lock()
        self._connection = Connection(record, identity, self._notification, timeout)
        self.lsp = Lsp(self)
        event(
            "connect",
            "completed",
            instance_id=self.instance_id,
            session_id=identity["session_id"],
        )

    @property
    def closed(self) -> bool:
        """Report whether the RPC connection has closed or started disconnecting."""
        return self._connection.closed

    def _notification(self, nv: Nvim, name: str, args: list[Any]) -> None:
        if name == "pi_nvim_lsp":
            token, response = args
            with self._responses_lock:
                future = self._responses.pop(token, None)
            if future is not None and not future.done():
                future.set_result(response)
        elif self._notification_handler is not None:
            self._notification_handler(nv, name, args)

    def _update_model(self, model: dict[str, str] | None) -> None:
        """Queue status metadata without blocking the Python worker on an editor prompt."""
        self._connection.check_thread()
        nv = self._connection.native
        assert nv is not None

        def update() -> None:
            if self.closed:
                return
            try:
                bridge(nv, "", "update_model", model)
                event("model", "updated", instance_id=self.instance_id, model=model)
            except Exception as error:  # noqa: BLE001 - Report background status failures without stopping RPC dispatch.
                event(
                    "model",
                    "failed",
                    instance_id=self.instance_id,
                    error_type=type(error).__name__,
                )

        try:
            nv.async_call(update)
        except RuntimeError:
            if not self.closed:
                raise

    @contextmanager
    def _operation(self, label: str) -> Generator[None]:
        self._connection.check_thread()
        started = time.monotonic()
        event(label, "started", instance_id=self.instance_id)
        try:
            self._connection.call(
                lambda nv: bridge(nv, "", "begin_operation", label), 5
            )
            try:
                yield
            finally:
                if not self.closed:
                    self._connection.call(lambda nv: bridge(nv, "", "end_operation"), 5)
        except BaseException as error:
            event(
                label,
                "failed",
                instance_id=self.instance_id,
                error_type=type(error).__name__,
                duration_ms=round((time.monotonic() - started) * 1000),
            )
            raise
        else:
            event(
                label,
                "completed",
                instance_id=self.instance_id,
                duration_ms=round((time.monotonic() - started) * 1000),
            )

    def run(self, action: Callable[[Nvim], T], *, timeout: float = 30) -> T:
        """Execute a synchronous callback with native pynvim and return its value.

        timeout is in seconds. Timeout or Python interruption closes this attachment
        to stop further queued callback work. LSP requests have separate cancellation
        and keep the connection alive. Call display_image() on the cell thread.
        """
        seconds(timeout)
        with self._operation("native"):
            return self._connection.call(action, timeout)

    def lua(self, code: str, *args: Any, timeout: float = 30) -> Any:
        """Execute Lua with native nvim_exec_lua arguments and return its RPC result."""
        seconds(timeout)
        with self._operation("lua"):
            return self._connection.call(lambda nv: nv.exec_lua(code, *args), timeout)

    def command(self, command: str, *, timeout: float = 30) -> None:
        """Execute an Ex command; mutations and saving follow that command's semantics."""
        seconds(timeout)
        with self._operation("command"):
            self._connection.call(lambda nv: nv.command(command), timeout)

    def on_notification(
        self, handler: Callable[[Nvim, str, list[Any]], None] | None
    ) -> None:
        """Set the handler for native notifications such as nvim_buf_lines_event.

        The handler runs on the RPC thread, receives native pynvim and may use its
        APIs. Register native subscriptions with run(); pass None to clear the handler.
        """
        self._connection.check_thread()
        self._notification_handler = handler

    def disconnect(self) -> None:
        """Release exclusive ownership and close this socket without exiting Neovim."""
        self._connection.check_thread()
        try:
            if not self.closed:
                self._connection.call(lambda nv: bridge(nv, "", "disconnect"), 2)
        finally:
            self._connection.stop()
            if _sessions.get(self.instance_id) is self:
                del _sessions[self.instance_id]
            event("disconnect", "completed", instance_id=self.instance_id)

    def __enter__(self) -> Self:
        if self.closed:
            raise ConnectionError("Neovim connection is closed")
        return self

    def __exit__(self, *_args: object) -> None:
        self.disconnect()

    def __repr__(self) -> str:
        return f"Session(instance_id={self.instance_id!r}, closed={self.closed})"


def connect(*, instance_id: str, timeout: float = 10) -> Session:
    """Acquire one discovered instance for this Pi session, reusing a live attachment.

    Another Pi session is rejected by Neovim's handshake. A restarted instance has
    a new ID and must be selected again. timeout is in seconds.
    """
    seconds(timeout)
    existing = _sessions.get(instance_id)
    if existing is not None and not existing.closed:
        existing._connection.check_thread()
        return existing
    if _identity is None:
        raise RuntimeError("nvim-use must be initialized by the Pi Python worker")
    try:
        session = Session(resolve(instance_id), _identity, timeout)
    except BaseException as error:
        event(
            "connect",
            "failed",
            instance_id=instance_id,
            error_type=type(error).__name__,
        )
        raise
    _sessions[instance_id] = session
    return session


def close_all() -> None:
    """Disconnect this worker's attachments when its Python environment exits."""
    for session in list(_sessions.values()):
        try:
            session.disconnect()
        except Exception as error:  # noqa: BLE001 - Finish closing the other attachments at interpreter exit.
            event(
                "disconnect",
                "failed",
                instance_id=session.instance_id,
                error_type=type(error).__name__,
            )


atexit.register(close_all)
