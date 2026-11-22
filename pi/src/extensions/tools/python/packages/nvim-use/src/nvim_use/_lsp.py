"""Send native LSP requests through Neovim clients and adapt editor coordinates."""

from typing import TYPE_CHECKING, Any
from uuid import uuid4

from ._connection import bridge, seconds

if TYPE_CHECKING:
    from ._session import Session


class LspError(RuntimeError):
    """Expose the server's original ResponseError, including code and optional data."""

    def __init__(self, response: dict[str, Any]) -> None:
        self.response = response
        super().__init__(f"LSP {response['code']}: {response['message']}")


class Lsp:
    """Reuse an attached editor's language-server clients without requiring buffers.

    Raw params/results keep the client's native LSP position encoding. Editor
    positions use pynvim cursor coordinates: 1-based row, 0-based UTF-8 byte column.
    All methods run on the session's creating Python thread.
    """

    def __init__(self, session: "Session") -> None:
        self._session = session

    def clients(self) -> list[dict[str, Any]]:
        """List active clients, workspace folders, encodings, capabilities and buffers."""
        with self._session._operation("lsp.clients"):
            return self._session._connection.call(
                lambda nv: bridge(nv, ".lsp", "clients"), 5
            )

    def position_params(
        self,
        *,
        method: str,
        buffer: int,
        editor_position: tuple[int, int],
        client_id: int | None = None,
    ) -> dict[str, Any]:
        """Build native textDocument/position params for a buffer and editor coordinates.

        Returns {client_id, params}. Without client_id the buffer must have exactly
        one client supporting method. A buffer handle of 0 selects the current buffer.
        The editor's focus and cursor are left unchanged.
        """
        with self._session._operation("lsp.position"):
            return self._session._connection.call(
                lambda nv: bridge(
                    nv,
                    ".lsp",
                    "position_params",
                    method,
                    buffer,
                    list(editor_position),
                    client_id,
                ),
                5,
            )

    def request(
        self,
        *,
        method: str,
        client_id: int | None = None,
        params: Any = None,
        buffer: int | None = None,
        editor_position: tuple[int, int] | None = None,
        timeout: float = 30,
    ) -> Any:
        """Return a native LSP result, or raise LspError with the server's ResponseError.

        Supply client_id + params for a raw request, including URIs for unopened
        files. Alternatively supply buffer + editor_position; params may then add
        method-specific fields such as context or newName. client_id is optional
        only when one attached client supports the method. Params never load files.

        timeout is in seconds. Timeout or Python interruption cancels this request
        and keeps the editor connection alive; late replies are discarded.
        """
        seconds(timeout)
        if buffer is not None or editor_position is not None:
            if buffer is None or editor_position is None:
                raise ValueError("Supply both buffer and editor_position")
            if params is not None and not isinstance(params, dict):
                raise ValueError(
                    "Extra params for editor coordinates must be a dictionary"
                )
            if params and ("textDocument" in params or "position" in params):
                raise ValueError(
                    "Use raw params or editor coordinates for textDocument/position"
                )
            adapted = self.position_params(
                method=method,
                buffer=buffer,
                editor_position=editor_position,
                client_id=client_id,
            )
            client_id = adapted["client_id"]
            params = {**adapted["params"], **(params or {})}
        if client_id is None:
            raise ValueError(
                "client_id is required for raw LSP requests; select it with lsp.clients()"
            )
        return self._wait_request(
            "start", method, (client_id, method, params, buffer), timeout
        )

    def _wait_request(
        self, entry: str, label: str, arguments: tuple[Any, ...], timeout: float
    ) -> Any:
        session = self._session
        connection = session._connection
        with session._operation(label):
            token = uuid4().hex
            future = connection.future()
            with session._responses_lock:
                session._responses[token] = future
            try:
                connection.call(
                    lambda nv: bridge(nv, ".lsp", entry, token, *arguments), timeout
                )
                response = connection.wait(future, timeout)
                if response["error"] is not None:
                    raise LspError(response["error"])
                return response["result"]
            finally:
                with session._responses_lock:
                    pending = session._responses.pop(token, None)
                connection.forget(future)
                if pending is not None and not session.closed:
                    connection.call(lambda nv: bridge(nv, ".lsp", "cancel", token), 2)

    def notify(self, *, client_id: int, method: str, params: Any) -> bool:
        """Send a native LSP notification; document synchronization belongs to Neovim.

        Use open_document() for didOpen and native buffer operations for changes,
        saving and unloading, so Neovim retains document version ownership.
        """
        with self._session._operation(method):
            return self._session._connection.call(
                lambda nv: bridge(nv, ".lsp", "notify", client_id, method, params), 5
            )

    def open_document(self, *, client_id: int, uri: str) -> int:
        """Load a document in a hidden buffer and attach the chosen existing client.

        Returns the native buffer number. An existing buffer retains unsaved text.
        Windows and focus stay unchanged; unload with native buffer APIs when done.
        """
        with self._session._operation("lsp.open_document"):
            return self._session._connection.call(
                lambda nv: bridge(nv, ".lsp", "open_document", client_id, uri), 30
            )

    def execute_command(
        self,
        *,
        client_id: int,
        command: dict[str, Any],
        buffer: int | None = None,
        timeout: float = 30,
    ) -> Any:
        """Execute an LSP Command through Neovim, including registered client commands.

        Pass a Command object (command, optional title and arguments), for example
        a resolved code action's command. Server commands return their native result.
        """
        seconds(timeout)
        return self._wait_request(
            "execute_command",
            "lsp.execute_command",
            (client_id, command, buffer),
            timeout,
        )
