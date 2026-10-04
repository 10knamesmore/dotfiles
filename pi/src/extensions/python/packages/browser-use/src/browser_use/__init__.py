"""Use Playwright through Pi-managed browser discovery, ownership, and synchronous calls."""

from ._discovery import BrowserInstance, discover
from ._process import _set_lifecycle_hook as _set_lifecycle_hook
from ._session import Session, close_all, connect, launch

__all__ = ["BrowserInstance", "Session", "close_all", "connect", "discover", "launch"]
