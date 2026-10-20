"""Publish guided code tours; Neovim owns navigation, highlights and comments."""

from typing import TYPE_CHECKING, NotRequired, TypedDict

from ._connection import bridge

if TYPE_CHECKING:
    from ._session import Session


class ShowcastHighlight(TypedDict):
    """Half-open range; each position is (1-based row, 0-based UTF-8 byte column)."""

    start: tuple[int, int]
    end: tuple[int, int]


class ShowcastStop(TypedDict):
    """A live buffer ID or absolute file path, cursor position and Markdown comment."""

    buffer: int | str
    position: tuple[int, int]
    title: str
    comment: str
    highlights: NotRequired[list[ShowcastHighlight]]


class Showcast:
    """One tour per connected editor, cleared on replacement or disconnect."""

    def __init__(self, session: "Session") -> None:
        self._session = session

    def start(self, *, title: str, stops: list[ShowcastStop]) -> None:
        """Tour real buffers in the current window, preserving unsaved contents.

        Positions use pynvim's (1-based row, 0-based UTF-8 byte column).
        Highlight ends are exclusive. n/N navigate, Tab toggles the comment,
        Enter focuses it for scrolling, and q restores the original buffer,
        cursor, scroll position and winbar without closing the window.
        """
        with self._session._operation("showcast"):
            self._session._connection.call(
                lambda nv: bridge(nv, ".showcast", "start", title, stops), 30
            )

    def stop(self) -> None:
        """Restore the original window view and remove tour highlights and mappings."""
        with self._session._operation("showcast"):
            self._session._connection.call(
                lambda nv: bridge(nv, ".showcast", "stop"), 30
            )

    def toggle_comment(self) -> None:
        """Toggle the current station's floating Markdown comment."""
        with self._session._operation("showcast"):
            self._session._connection.call(
                lambda nv: bridge(nv, ".showcast", "toggle_comment"), 30
            )
