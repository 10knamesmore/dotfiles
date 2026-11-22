"""Keep native pynvim callbacks and notifications on one persistent RPC thread."""

import math
import threading
import time
from collections.abc import Callable
from concurrent.futures import Future
from typing import Any, TypeVar

from pynvim import Nvim
from pynvim.msgpack_rpc import socket_session
from pynvim.msgpack_rpc.session import Session as RpcSession

from ._diagnostics import event

T = TypeVar("T")


def seconds(value: float) -> float:
    """Validate a finite positive deadline in seconds."""
    if not math.isfinite(value) or value <= 0:
        raise ValueError("timeout must be finite and positive")
    return value


def bridge(nv: Nvim, module: str, method: str, *args: Any) -> Any:
    """Call a Lua bridge operation with this native client's channel identity."""
    return nv.exec_lua(
        f"return require('pi_bridge{module}').{method}(...)", nv.channel_id, *args
    )


class Connection:
    """Run pynvim on its own thread and wake every waiter when the socket closes."""

    def __init__(
        self,
        record: dict[str, Any],
        identity: dict[str, Any],
        notification: Callable[[Nvim, str, list[Any]], None],
        timeout: float,
    ) -> None:
        self._owner = threading.get_ident()
        self._lock = threading.Lock()
        self._pending: set[Future[Any]] = set()
        self._closing = threading.Event()
        self._ended = threading.Event()
        self.native: Nvim | None = None
        self._rpc: RpcSession | None = None
        self._ready: Future[dict[str, Any]] = self.future()
        self._thread = threading.Thread(
            target=self._serve,
            args=(record, identity, notification),
            name="nvim-use",
            daemon=True,
        )
        self._thread.start()
        try:
            self.record = self.wait(self._ready, timeout)
        except BaseException:
            self.stop()
            raise

    @property
    def closed(self) -> bool:
        return self._closing.is_set() or self._ended.is_set()

    def check_thread(self) -> None:
        if threading.get_ident() != self._owner:
            raise RuntimeError(
                "Call nvim-use on its creating Python thread; use native pynvim inside run callbacks"
            )

    def future(self) -> Future[Any]:
        """Register a waiter that must also fail if Neovim disconnects."""
        with self._lock:
            if self.closed:
                raise ConnectionError("Neovim connection is closed")
            future: Future[Any] = Future()
            self._pending.add(future)
        return future

    def forget(self, future: Future[Any]) -> None:
        with self._lock:
            self._pending.discard(future)

    def wait(self, future: Future[T], timeout: float) -> T:
        """Wait on the cell thread so Python interrupts and deadlines remain observable."""
        deadline = time.monotonic() + seconds(timeout)
        try:
            while not future.done():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("Neovim operation timed out")
                time.sleep(min(0.02, remaining))
            return future.result()
        finally:
            self.forget(future)

    def call(self, action: Callable[[Nvim], T], timeout: float) -> T:
        """Run a synchronous native callback; interruption closes this RPC connection."""
        self.check_thread()
        seconds(timeout)
        future = self.future()
        nv = self.native
        assert nv is not None

        def execute() -> None:
            if self.closed:
                return
            try:
                value = action(nv)
            except BaseException as error:  # noqa: BLE001 - Deliver callback exceptions, including interrupts, to the cell.
                if not future.done():
                    future.set_exception(error)
            else:
                if not future.done():
                    future.set_result(value)

        try:
            nv.async_call(execute)
            return self.wait(future, timeout)
        except (KeyboardInterrupt, TimeoutError):
            self.stop()
            raise
        finally:
            self.forget(future)

    def _serve(
        self,
        record: dict[str, Any],
        identity: dict[str, Any],
        notification: Callable[[Nvim, str, list[Any]], None],
    ) -> None:
        failure: BaseException = ConnectionError("Neovim connection closed")
        nv: Nvim | None = None
        try:
            self._rpc = socket_session(record["socket"])
            if self._closing.is_set():
                return
            nv = Nvim.from_session(self._rpc)
            self.native = nv
            if self._closing.is_set():
                return

            def setup() -> None:
                try:
                    info = bridge(nv, "", "connect", identity, record["instance_id"])
                    nv.api.set_client_info(
                        "pi-nvim",
                        {"major": 0, "minor": 1},
                        "remote",
                        {},
                        {key: identity[key] for key in ("session_id", "cwd", "pid")},
                    )
                    self._ready.set_result(info)
                except BaseException as error:  # noqa: BLE001 - Wake the cell waiting for this handshake.
                    self._ready.set_exception(error)
                    nv.stop_loop()

            def on_request(name: str, _args: list[Any]) -> Any:
                raise RuntimeError(f"Unsupported Neovim-to-Pi request: {name}")

            nv.run_loop(
                on_request,
                lambda name, args: notification(nv, name, args),
                setup_cb=setup,
                err_cb=lambda _message: event("rpc_callback", "failed"),
            )
        except BaseException as error:  # noqa: BLE001 - Wake all request waiters when the RPC thread exits.
            failure = error
            event("connection", "lost", error_type=type(error).__name__)
        finally:
            self._closing.set()
            try:
                if self._rpc is not None:
                    # pynvim closes its asyncio loop immediately after transport.close().
                    # Drain the transport's close callback first, which releases the OS
                    # socket even after a request timeout stopped the RPC loop.
                    rpc_loop: Any = self._rpc.loop
                    rpc_loop._transport.abort()
                    rpc_loop._loop.call_soon(rpc_loop._loop.stop)
                    rpc_loop._loop.run_forever()
                    self._rpc.close()
            except Exception as error:  # noqa: BLE001 - Cleanup failure must still wake disconnected waiters.
                event("socket_cleanup", "failed", error_type=type(error).__name__)
            with self._lock:
                for future in self._pending:
                    if not future.done():
                        future.set_exception(failure)
                self._pending.clear()
            self._ended.set()

    def stop(self) -> None:
        """Close the socket on its thread, including when a native RPC is waiting.

        An arbitrary Python callback must yield or finish before its thread can exit.
        Pi's worker deadline remains responsible for callbacks stuck in native code.
        """
        self.check_thread()
        self._closing.set()
        rpc = self._rpc
        if rpc is not None and not self._ended.is_set():
            try:
                rpc.loop.threadsafe_call(rpc.stop)
            except RuntimeError:
                pass  # The socket's event loop already finished closing.
        self._thread.join()
