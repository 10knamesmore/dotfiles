"""Keep Playwright's event loop running while synchronous Python cells wait or finish."""

import asyncio
from collections.abc import Awaitable, Callable
import concurrent.futures
import math
import threading
import time
from typing import TypeVar

from ._diagnostics import event

T = TypeVar("T")


def seconds(value: float) -> float:
    """Validate the positive, finite timeout used by the synchronous adapter."""
    if not math.isfinite(value) or value <= 0:
        raise ValueError("timeout must be finite and positive")
    return value


class BrowserLoop:
    """Own one session's event thread; callers wait on the thread that created it."""

    def __init__(self) -> None:
        self._owner = threading.get_ident()
        self._loop = asyncio.new_event_loop()
        self._thread = threading.Thread(
            target=self._serve, name="browser-use", daemon=True
        )
        self._thread.start()

    def check_thread(self) -> None:
        if threading.get_ident() != self._owner:
            raise RuntimeError(
                "Call browser-use from its creating Python thread, not inside a Playwright callback"
            )

    def _serve(self) -> None:
        asyncio.set_event_loop(self._loop)
        self._loop.set_exception_handler(
            lambda _loop, context: event(
                "background_task",
                "failed",
                error_type=type(context.get("exception")).__name__,
            )
        )
        try:
            self._loop.run_forever()
        finally:
            tasks = asyncio.all_tasks(self._loop)
            for task in tasks:
                task.cancel()
            if tasks:
                self._loop.run_until_complete(asyncio.wait(tasks, timeout=0.5))
            self._loop.close()

    def call(
        self, operation: str, action: Callable[[], Awaitable[T]], timeout: float
    ) -> T:
        """Wait synchronously for an async action; interrupting the caller cancels its task.

        Cancellation cannot undo commands already delivered to Chrome. Wait for the
        task's cancellation cleanup before returning, so it cannot overlap the next cell.
        Pi's outer deadline terminates the worker if user code refuses cancellation.
        """
        self.check_thread()
        timeout = seconds(timeout)
        result: concurrent.futures.Future[T] = concurrent.futures.Future()
        finished = threading.Event()
        tasks: list[asyncio.Task[None]] = []
        started = time.monotonic()
        event(operation, "started")

        async def execute() -> None:
            try:
                async with asyncio.timeout(timeout):
                    value = await action()
            except BaseException as error:
                # Deliver SystemExit/KeyboardInterrupt to the cell interpreter rather
                # than letting asyncio terminate this persistent event thread.
                result.set_exception(error)
            else:
                result.set_result(value)

        def complete(task: asyncio.Task[None]) -> None:
            try:
                if task.cancelled():
                    result.cancel()
                elif (error := task.exception()) is not None:
                    result.set_exception(error)
            finally:
                finished.set()

        def submit() -> None:
            task = self._loop.create_task(execute())
            tasks.append(task)
            task.add_done_callback(complete)

        def cancel() -> None:
            for task in tasks:
                task.cancel()

        self._loop.call_soon_threadsafe(submit)
        try:
            while not finished.wait(0.025):
                pass
            value = result.result()
        except BaseException as error:
            if not finished.is_set():
                # submit and cancel are queued in order on the same event loop.
                self._loop.call_soon_threadsafe(cancel)
                finished.wait()
            event(
                operation,
                "failed",
                error_type=type(error).__name__,
                duration_ms=round((time.monotonic() - started) * 1000),
            )
            raise
        event(
            operation,
            "completed",
            duration_ms=round((time.monotonic() - started) * 1000),
        )
        return value

    def stop(self) -> None:
        """Stop dispatching events after Playwright has disconnected."""
        self.check_thread()
        self._loop.call_soon_threadsafe(self._loop.stop)
        self._thread.join()
