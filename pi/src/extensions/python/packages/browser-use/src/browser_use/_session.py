"""Connect Pi-managed browser lifetimes to unwrapped Playwright async objects."""

import atexit
import asyncio
from collections.abc import Awaitable, Callable
from pathlib import Path
import tempfile
from typing import Literal, TypeVar

from playwright.async_api import (
    Browser,
    BrowserContext,
    Error,
    Playwright,
    async_playwright,
)

from ._diagnostics import event
from ._discovery import resolve
from ._process import OwnedBrowser, check_thread
from ._runtime import BrowserLoop, seconds

T = TypeVar("T")
_sessions: set["Session"] = set()


class Session:
    """Keep a browser connection alive across Python cells; create with connect or launch.

    Attributes:
        pid: Browser main PID for local discovery/launch, otherwise None.
        mode: Whether close disconnects user Chrome or also terminates owned Chrome.
        closed: Whether this adapter has been closed; not a live browser health check.
    """

    def __init__(
        self,
        source: str | OwnedBrowser,
        pid: int | None,
        timeout: float,
        *,
        configure_browser: bool = False,
    ) -> None:
        owned = source if isinstance(source, OwnedBrowser) else None
        self.pid = pid
        self.mode: Literal["attached", "launched"] = (
            "launched" if owned is not None else "attached"
        )
        self.closed = False
        self._owned = owned
        self._source = source
        self._artifacts = tempfile.TemporaryDirectory(prefix="browser-use-artifacts-")
        self._loop = BrowserLoop()
        self._manager = async_playwright()
        self._playwright: Playwright | None = None
        self._browser: Browser | None = None
        self._context: BrowserContext | None = None
        self._configure_browser = owned is not None or configure_browser
        try:
            self._loop.call(
                "launch" if owned is not None else "connect", self._connect, timeout
            )
        except BaseException:
            try:
                self.close()
            except Exception:
                # close() records cleanup failures; preserve the original startup error.
                pass
            raise
        _sessions.add(self)
        event("session", "opened", mode=self.mode, pid=pid)

    async def _connect(self) -> None:
        endpoint = (
            await self._source.endpoint()
            if isinstance(self._source, OwnedBrowser)
            else self._source
        )
        self._playwright = await self._manager.start()
        self._browser = await self._playwright.chromium.connect_over_cdp(
            endpoint,
            timeout=0,
            no_defaults=not self._configure_browser,
            is_local=self.pid is not None,
            artifacts_dir=self._artifacts.name,
        )
        self._context = self._browser.contexts[0]
        self._context.set_default_timeout(10_000)
        self._context.set_default_navigation_timeout(30_000)

    def run(
        self, action: Callable[[BrowserContext], Awaitable[T]], *, timeout: float = 30
    ) -> T:
        """Run an async callback on this browser's loop and synchronously return its result.

        The callback receives the native Playwright default BrowserContext. All Page,
        Locator, Download and event-handler operations belong inside this callback or
        callbacks it registers. Handles may be saved across cells, but stay on this loop.
        Return screenshot bytes and call display_image on the Python cell's main thread.

        Args:
            action: Async function, or a function returning an awaitable.
            timeout: Deadline for the entire callback in seconds. Native Playwright
                method timeouts remain milliseconds. Neither extends Pi's outer limit.
        """
        if self.closed:
            raise RuntimeError("Browser session is closed")

        async def execute() -> T:
            if self._context is None:
                raise RuntimeError("Browser session is not connected")
            return await action(self._context)

        return self._loop.call("run", execute, timeout)

    async def _disconnect(self) -> None:
        try:
            if self._browser is not None and self._browser.is_connected():
                try:
                    if self._context is not None:
                        await self._context.unroute_all(behavior="ignoreErrors")
                        for page in self._context.pages:
                            if not page.is_closed():
                                await page.unroute_all(behavior="ignoreErrors")
                    if self._owned is not None or self._configure_browser:
                        cdp = await self._browser.new_browser_cdp_session()
                        if self._owned is not None:
                            # CDP attachments disconnect on Browser.close(); explicitly
                            # request Chrome exit only when this adapter owns its process.
                            await cdp.send("Browser.close")
                            deadline = asyncio.get_running_loop().time() + 1
                            while (
                                self._owned.process.poll() is None
                                and asyncio.get_running_loop().time() < deadline
                            ):
                                await asyncio.sleep(0.025)
                        else:
                            await cdp.send(
                                "Browser.setDownloadBehavior", {"behavior": "default"}
                            )
                            await cdp.detach()
                except Error as error:
                    event(
                        "browser_cleanup",
                        "disconnected_or_failed",
                        error_type=type(error).__name__,
                    )
                await self._browser.close()
        finally:
            # A manager exists even when connection startup is cancelled partway through.
            await self._manager.__aexit__()

    def close(self) -> None:
        """Disconnect, stop event handling, and terminate only SDK-launched Chrome.

        Existing user tabs remain open. Removes this client's request routes and resets
        downloads when explicitly enabled. Idempotent; callable only on the owning
        Python thread, never from a Playwright callback.
        """
        self._loop.check_thread()
        check_thread()
        if self.closed:
            return
        self.closed = True
        try:
            try:
                self._loop.call("disconnect", self._disconnect, 5)
            except Exception as error:
                event("disconnect", "cleanup_failed", error_type=type(error).__name__)
                raise
        finally:
            try:
                self._loop.stop()
            finally:
                try:
                    if self._owned is not None:
                        self._owned.close()
                finally:
                    self._artifacts.cleanup()
                    _sessions.discard(self)
                    event("session", "closed", mode=self.mode, pid=self.pid)

    def __enter__(self) -> "Session":
        if self.closed:
            raise RuntimeError("Browser session is closed")
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()

    def __repr__(self) -> str:
        return f"Session(mode={self.mode!r}, pid={self.pid}, closed={self.closed})"


def connect(
    *,
    pid: int | None = None,
    endpoint: str | None = None,
    configure_browser: bool = False,
    timeout: float = 30,
) -> Session:
    """Attach by window/browser PID or CDP address; closing preserves the user's Chrome.

    Supply exactly one of pid or endpoint. Remote debugging must already be enabled;
    Chrome's connection authorization is left to the user. Timeout is in seconds.
    configure_browser=True opts into Playwright's default page overrides and download
    handling. The default leaves those settings alone; native expect_download requires
    opting in when connecting, not enabling a second CDP client's download events.
    """
    check_thread()
    seconds(timeout)
    if pid is not None:
        if endpoint is not None:
            raise ValueError("Supply exactly one of pid or endpoint")
        instance, address = resolve(pid)
        return Session(
            address, instance.pid, timeout, configure_browser=configure_browser
        )
    if endpoint is None:
        raise ValueError("Supply exactly one of pid or endpoint")
    return Session(endpoint, None, timeout, configure_browser=configure_browser)


def launch(
    *,
    executable_path: str | Path | None = None,
    user_data_dir: str | Path | None = None,
    headless: bool = False,
    timeout: float = 30,
) -> Session:
    """Launch installed Chrome in a separately owned group; never download a browser.

    An omitted profile is temporary. An explicit profile survives close and must not
    be the user's daily Chrome profile. Timeout is in seconds, including CDP readiness.
    """
    check_thread()
    seconds(timeout)
    owned = OwnedBrowser(executable_path, user_data_dir, headless)
    try:
        return Session(owned, owned.process.pid, timeout)
    except BaseException:
        owned.close()
        raise


def close_all() -> None:
    """Close this worker's browser sessions, leaving attached user browsers alive."""
    check_thread()
    failure: Exception | None = None
    for session in list(_sessions):
        try:
            session.close()
        except Exception as error:
            failure = error
    if failure is not None:
        raise failure


atexit.register(close_all)
