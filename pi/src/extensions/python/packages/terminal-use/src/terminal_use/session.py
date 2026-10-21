"""Operations on one terminal window owned by a Kitty instance."""

import base64
import json
import math
import re
import subprocess
import time
from typing import TYPE_CHECKING, Any, Literal, Sequence

from .models import RectInput, rectangle

if TYPE_CHECKING:
    from .instance import KittyInstance


KEY_ALIASES = {
    "return": "enter",
    "esc": "escape",
    "prior": "page_up",
    "pageup": "page_up",
    "next": "page_down",
    "pagedown": "page_down",
    " ": "space",
    "`": "grave_accent",
    "grave": "grave_accent",
    "-": "minus",
    "=": "equal",
    "[": "left_bracket",
    "bracketleft": "left_bracket",
    "]": "right_bracket",
    "bracketright": "right_bracket",
    "\\": "backslash",
    ";": "semicolon",
    "'": "apostrophe",
    ".": "period",
    ",": "comma",
    "/": "slash",
    "control": "ctrl",
    "cmd": "super",
}
MODIFIERS = {"ctrl", "shift", "alt", "super"}


def duration(value: float, name: str) -> None:
    if not math.isfinite(value) or value < 0:
        raise ValueError(f"{name} must be finite and nonnegative")


def key_name(keys: Sequence[str]) -> str:
    names = [KEY_ALIASES.get(key.lower(), key.lower()) for key in keys]
    if sum(name not in MODIFIERS for name in names) != 1:
        raise ValueError(
            "Pass one base key and separate modifiers, for example send_key('ctrl', 'c')"
        )
    return "+".join(names)


def encode_event(event: dict[str, Any]) -> dict[str, Any]:
    kind = event["kind"]
    if kind == "key":
        return {"kind": kind, "key": key_name(event["keys"])}
    if kind in ("text", "paste"):
        if not isinstance(event["text"], str):
            raise TypeError("text must be a string")
        return {"kind": kind, "text": event["text"]}
    if kind == "raw":
        return {"kind": kind, "data": base64.b64encode(event["data"]).decode("ascii")}
    if kind != "mouse":
        raise ValueError(f"Unknown input kind: {kind}")
    result = dict(event)
    if result["action"] not in ("press", "release", "move", "scroll"):
        raise ValueError("Unknown mouse action")
    for coordinate in ("col", "row"):
        if not isinstance(result[coordinate], int) or result[coordinate] < 0:
            raise ValueError(f"{coordinate} must be a nonnegative integer")
    result["modifiers"] = [modifier.lower() for modifier in result.get("modifiers", [])]
    if any(modifier not in MODIFIERS for modifier in result["modifiers"]):
        raise ValueError("Unknown mouse modifier")
    if result["action"] == "scroll":
        result["steps"] = result.get("steps", 1)
        if (
            result["direction"] not in ("up", "down", "left", "right")
            or result["steps"] <= 0
        ):
            raise ValueError("Scroll requires a direction and positive steps")
    elif result["action"] != "move" or result.get("button") is not None:
        if result["button"] not in ("left", "middle", "right"):
            raise ValueError("Mouse button must be left, middle or right")
    return result


def project_closed_screen(
    screen: dict[str, Any], rect: dict[str, int] | None, trim: bool
) -> dict[str, Any]:
    requested = rect or {
        "x": 0,
        "y": 0,
        "width": screen["full_size"]["cols"],
        "height": screen["full_size"]["rows"],
    }
    x = min(requested["x"], screen["full_size"]["cols"])
    y = min(requested["y"], screen["full_size"]["rows"])
    width = min(requested["width"], screen["full_size"]["cols"] - x)
    height = min(requested["height"], screen["full_size"]["rows"] - y)
    screen["cells"] = [row[x : x + width] for row in screen["cells"][y : y + height]]
    lines = []
    for row in screen["cells"]:
        text = "".join(
            cell["text"]
            for index, cell in enumerate(row)
            if not cell["wide_continuation"]
            and not (cell["wide"] and index == len(row) - 1)
        )
        lines.append(text.rstrip(" ") if trim else text)
    screen["lines"], screen["text"] = lines, "\n".join(lines)
    screen["rect"] = {
        "requested": requested,
        "effective": {"x": x, "y": y, "width": width, "height": height},
    }
    cursor = screen["cursor"]
    inside = x <= cursor["x"] < x + width and y <= cursor["y"] < y + height
    cursor.update(
        inside_rect=inside,
        relative_x=cursor["x"] - x if inside else None,
        relative_y=cursor["y"] - y if inside else None,
    )
    return screen


class Session:
    """A handle to a Kitty window. Dropping or detaching the handle leaves it running."""

    def __init__(self, instance: "KittyInstance", window_id: int):
        self.instance = instance
        self.window_id = window_id
        self._detached = False

    def __repr__(self) -> str:
        return f"Session(socket={self.instance.socket!r}, window_id={self.window_id})"

    @property
    def id(self) -> str:
        return f"{self.instance.id}:{self.window_id}"

    @property
    def pid(self) -> int:
        return self.inspect()["pid"]

    @property
    def closed(self) -> bool:
        return self._closed_event() is not None

    def _closed_event(self) -> dict[str, Any] | None:
        self._require_attached()
        path = self.instance.event_directory / f"{self.window_id}.json"
        return json.loads(path.read_text()) if path.exists() else None

    def _require_attached(self) -> None:
        if self._detached:
            raise RuntimeError("This handle is detached; attach the window again")

    def _request(self, action: str, **arguments: Any) -> Any:
        self._require_attached()
        return self.instance._transport.bridge(action, self.window_id, **arguments)

    def inspect(self) -> dict[str, Any]:
        event = self._closed_event()
        if event is not None:
            return {
                **event["info"],
                "status": event["status"],
                "closed": True,
                "session_id": self.id,
            }
        info = self._request("inspect")
        return {**info, "closed": False, "session_id": self.id}

    def read(
        self,
        *,
        rect: RectInput | None = None,
        wait_for: dict[str, str] | None = None,
        timeout: float = 0.0,
        trim_trailing_spaces: bool = True,
        cells: bool = False,
    ) -> dict[str, Any]:
        duration(timeout, "timeout")
        region = rectangle(rect)
        matcher = None
        if wait_for is not None:
            if set(wait_for) not in ({"contains"}, {"regex"}):
                raise ValueError(
                    "wait_for must contain exactly one of contains or regex"
                )
            pattern = next(iter(wait_for.values()))
            if not pattern or timeout <= 0:
                raise ValueError(
                    "Waiting requires a nonempty pattern and positive timeout"
                )
            matcher = re.compile(
                re.escape(pattern) if "contains" in wait_for else pattern
            )
        elif timeout:
            raise ValueError("timeout requires wait_for")
        deadline = time.monotonic() + timeout
        screen = None
        status = {"kind": "running"}
        while True:
            if (
                screen is not None
                and matcher is not None
                and time.monotonic() >= deadline
            ):
                matched, timed_out = False, True
                break
            event = self._closed_event()
            if event is None:
                try:
                    screen = self._request(
                        "read",
                        rect=region,
                        cells=cells,
                        trim_trailing_spaces=trim_trailing_spaces,
                        timeout=min(
                            self.instance._transport.timeout,
                            max(0.001, deadline - time.monotonic()),
                        )
                        if matcher
                        else None,
                    )
                except subprocess.TimeoutExpired:
                    if matcher is None or screen is None or time.monotonic() < deadline:
                        raise
                    matched, timed_out = False, True
                    break
                status = screen["status"]
            else:
                screen = project_closed_screen(
                    event["screen"], region, trim_trailing_spaces
                )
                status = event["status"]
            matched = matcher is None or matcher.search(screen["text"]) is not None
            timed_out = not matched and time.monotonic() >= deadline
            if matched or timed_out or event is not None:
                break
            time.sleep(min(0.05, max(0.0, deadline - time.monotonic())))
        screen.update(
            session_id=self.id,
            status=status,
            wait={
                "matched": matched,
                "timed_out": timed_out,
                "pattern_supplied": matcher is not None,
            },
        )
        if not cells:
            screen["cells"] = None
        return screen

    def screenshot(self) -> bytes:
        """Return a PNG of the visible terminal, without changing focus or layout."""
        self._require_attached()
        return self.instance._transport.command_bytes(
            "screenshot", "--match", f"id:{self.window_id}"
        )

    def history(self, *, ansi: bool = False) -> str:
        self._require_attached()
        arguments = ["get-text", "--match", f"id:{self.window_id}", "--extent", "all"]
        if ansi:
            arguments.append("--ansi")
        return self.instance._transport.command(*arguments)

    def command_output(self, *, ansi: bool = False) -> str:
        self._require_attached()
        arguments = [
            "get-text",
            "--match",
            f"id:{self.window_id}",
            "--extent",
            "last_cmd_output",
        ]
        if ansi:
            arguments.append("--ansi")
        return self.instance._transport.command(*arguments)

    def input(
        self, events: Sequence[dict[str, Any]], *, delay: float = 0.0
    ) -> list[bool]:
        """Send validated events; delay is milliseconds between events, never a GUI-thread sleep."""
        duration(delay, "delay")
        encoded = [encode_event(event) for event in events]
        if not delay:
            return self._request("input", events=encoded)
        sent = []
        for index, event in enumerate(encoded):
            if index:
                time.sleep(delay / 1000)
            sent.extend(self._request("input", events=[event]))
        return sent

    def send_text(self, text: str) -> None:
        self.input([{"kind": "text", "text": text}])

    def send_key(self, *keys: str) -> bool:
        return self.input([{"kind": "key", "keys": keys}])[0]

    def paste(self, text: str) -> None:
        self.input([{"kind": "paste", "text": text}])

    def write(self, data: bytes) -> None:
        self.input([{"kind": "raw", "data": data}])

    def mouse_down(
        self, *, col: int, row: int, button: str = "left", modifiers: Sequence[str] = ()
    ) -> bool:
        return self.input(
            [
                {
                    "kind": "mouse",
                    "action": "press",
                    "col": col,
                    "row": row,
                    "button": button,
                    "modifiers": modifiers,
                }
            ]
        )[0]

    def mouse_up(
        self, *, col: int, row: int, button: str = "left", modifiers: Sequence[str] = ()
    ) -> bool:
        return self.input(
            [
                {
                    "kind": "mouse",
                    "action": "release",
                    "col": col,
                    "row": row,
                    "button": button,
                    "modifiers": modifiers,
                }
            ]
        )[0]

    def mouse_move(
        self,
        *,
        col: int,
        row: int,
        button: str | None = None,
        modifiers: Sequence[str] = (),
    ) -> bool:
        return self.input(
            [
                {
                    "kind": "mouse",
                    "action": "move",
                    "col": col,
                    "row": row,
                    "button": button,
                    "modifiers": modifiers,
                }
            ]
        )[0]

    def click(
        self, *, col: int, row: int, button: str = "left", modifiers: Sequence[str] = ()
    ) -> list[bool]:
        return self.input(
            [
                {
                    "kind": "mouse",
                    "action": action,
                    "col": col,
                    "row": row,
                    "button": button,
                    "modifiers": modifiers,
                }
                for action in ("press", "release")
            ]
        )

    def scroll(
        self,
        *,
        col: int,
        row: int,
        direction: str,
        steps: int = 1,
        modifiers: Sequence[str] = (),
    ) -> bool:
        return self.input(
            [
                {
                    "kind": "mouse",
                    "action": "scroll",
                    "col": col,
                    "row": row,
                    "direction": direction,
                    "steps": steps,
                    "modifiers": modifiers,
                }
            ]
        )[0]

    def focus(self) -> None:
        self._require_attached()
        self.instance._transport.command(
            "focus-window", "--match", f"id:{self.window_id}"
        )

    def resize(
        self, *, cols: int, rows: int, allow_font_resize: bool = False
    ) -> dict[str, Any]:
        """Request an approximate grid. Font changes affect the entire OS window."""
        if cols <= 0 or rows <= 0:
            raise ValueError("cols and rows must be positive")
        info = self.inspect()
        siblings = [
            window
            for window in self.instance.list_windows()
            if window["tab_id"] == info["tab_id"]
        ]
        if len(siblings) == 1:
            self.instance._transport.command(
                "resize-os-window",
                "--match",
                f"id:{self.window_id}",
                "--unit",
                "cells",
                "--width",
                str(cols),
                "--height",
                str(rows),
            )
        else:
            for axis, delta in (
                ("horizontal", cols - info["cols"]),
                ("vertical", rows - info["rows"]),
            ):
                self.instance._transport.command(
                    "resize-window",
                    "--match",
                    f"id:{self.window_id}",
                    "--axis",
                    axis,
                    "--increment",
                    str(delta),
                )
        time.sleep(0.1)
        info = self.inspect()
        if allow_font_resize:
            ratio = min(info["cols"] / cols, info["rows"] / rows)
            if ratio < 0.95:
                self._request("font_size", size=info["font_size"] * ratio)
                time.sleep(0.1)
                info = self.inspect()
        return {
            "requested": {"cols": cols, "rows": rows},
            "actual": {"cols": info["cols"], "rows": info["rows"]},
        }

    def fit_grid(
        self, *, min_cols: int, min_rows: int, min_font_size: float = 8.0
    ) -> dict[str, Any]:
        """Fit content by reducing the shared OS-window font size, at most three times."""
        if (
            min_cols <= 0
            or min_rows <= 0
            or not math.isfinite(min_font_size)
            or min_font_size <= 0
        ):
            raise ValueError("Grid dimensions and minimum font size must be positive")
        info = self.inspect()
        for _ in range(3):
            ratio = min(info["cols"] / min_cols, info["rows"] / min_rows)
            if ratio >= 1 or info["font_size"] <= min_font_size:
                break
            self._request(
                "font_size", size=max(min_font_size, info["font_size"] * ratio * 0.98)
            )
            time.sleep(0.1)
            info = self.inspect()
        return {
            "actual": {"cols": info["cols"], "rows": info["rows"]},
            "font_size": info["font_size"],
            "fits": info["cols"] >= min_cols and info["rows"] >= min_rows,
        }

    def signal(
        self,
        signal_name: str,
        *,
        target: Literal["foreground", "process", "process_group"] = "foreground",
    ) -> None:
        if target not in ("foreground", "process", "process_group"):
            raise ValueError("Unknown signal target")
        self._request("signal", signal=signal_name.upper(), target=target)

    def wait(self, *, timeout: float = 5.0) -> dict[str, Any]:
        """Wait for window closure; an exit code is returned only when Kitty reports it."""
        duration(timeout, "timeout")
        deadline = time.monotonic() + timeout
        while True:
            event = self._closed_event()
            if event is not None:
                return {"closed": True, "timed_out": False, "status": event["status"]}
            if time.monotonic() >= deadline:
                return {
                    "closed": event is not None,
                    "timed_out": True,
                    "status": event["status"]
                    if event is not None
                    else self.inspect()["status"],
                }
            time.sleep(min(0.05, max(0, deadline - time.monotonic())))

    def wait_command(self, *, after: int, timeout: float = 5.0) -> dict[str, Any]:
        """Wait for a shell-integrated command newer than the recorded sequence."""
        duration(timeout, "timeout")
        deadline = time.monotonic() + timeout
        while True:
            info = self.inspect()
            command = info["command"]
            if (
                command is not None
                and command["sequence"] > after
                and not command["running"]
            ):
                return {"completed": True, "timed_out": False, "command": command}
            if info["closed"] or time.monotonic() >= deadline:
                return {
                    "completed": False,
                    "timed_out": not info["closed"],
                    "command": command,
                }
            time.sleep(min(0.05, max(0, deadline - time.monotonic())))

    def close(self, *, timeout: float = 5.0) -> dict[str, Any]:
        if not self.closed:
            self.instance._transport.command(
                "close-window", "--match", f"id:{self.window_id}"
            )
        return self.wait(timeout=timeout)

    def detach(self) -> None:
        self._detached = True
