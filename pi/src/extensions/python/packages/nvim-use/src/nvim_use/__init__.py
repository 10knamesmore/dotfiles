"""Attach Pi to existing Neovim instances with native pynvim and LSP access."""

from ._discovery import discover
from ._lsp import LspError
from ._session import Session, connect
from ._session import _configure_worker as _configure_worker
from ._session import _set_model as _set_model

__all__ = ["LspError", "Session", "connect", "discover"]
