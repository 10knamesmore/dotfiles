"""Kitty-side screen access, input, and window lifecycle recording.

Loaded by Kitty as both a no-UI kitten and a global watcher. Only Python's
standard library and Kitty's own APIs are used in this process.
"""

import base64
import json
import os
import signal
import time
import uuid
from pathlib import Path

from kittens.tui.handler import result_handler
from kitty.fast_data_types import (
    DRAG,
    GLFW_MOD_ALT,
    GLFW_MOD_CONTROL,
    GLFW_MOD_SHIFT,
    GLFW_MOD_SUPER,
    MOVE,
    PRESS,
    RELEASE,
    cell_size_for_window,
    os_window_font_size,
    send_mouse_event,
)


def instance_id(boss):
    if not hasattr(boss, "_terminal_use_instance_id"):
        boss._terminal_use_instance_id = uuid.uuid4().hex
    return boss._terminal_use_instance_id


def event_path(boss, window_id):
    return (
        Path.home()
        / ".local/state/terminal-use"
        / instance_id(boss)
        / f"{window_id}.json"
    )


def save_event(boss, window, event):
    path = event_path(boss, window.id)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(event, ensure_ascii=False))
    temporary.replace(path)


def on_load(boss, data):
    instance_id(boss)


def on_cmd_startstop(boss, window, data):
    sequence = getattr(window, "_terminal_use_command_sequence", 0)
    if data["is_start"]:
        sequence += 1
        window._terminal_use_command_sequence = sequence
    window._terminal_use_command = {
        "sequence": sequence,
        "running": data["is_start"],
        "command": data["cmdline"],
        "exit_code": None if data["is_start"] else data["exit_status"],
    }


def on_close(boss, window, data):
    status = {"kind": "closed"}
    if window.child_died:
        code = window.child_exit_code
        status = {
            "kind": "exited",
            "exit_code": code if code >= 0 else None,
            "signal": -code if code < 0 else None,
        }
    save_event(
        boss,
        window,
        {
            "info": window_info(window),
            "status": status,
            "screen": snapshot(window, {"cells": True}),
            "closed_at": time.time(),
        },
    )


def watch(window):
    if not getattr(window, "_terminal_use_watched", False):
        window._terminal_use_watched = True
        for name, callback in (
            ("on_close", on_close),
            ("on_cmd_startstop", on_cmd_startstop),
        ):
            callbacks = getattr(window.watchers, name)
            if not any(
                getattr(item, "_terminal_use_callback", None) == name
                for item in callbacks
            ):
                callbacks.append(callback)


def window_info(window):
    result = window.as_dict()
    del result["env"]
    width, height = cell_size_for_window(window.os_window_id)
    result.update(
        os_window_id=window.os_window_id,
        tab_id=window.tab_id,
        rows=window.screen.lines,
        cols=window.screen.columns,
        cell_size={"width": width, "height": height},
        font_size=os_window_font_size(window.os_window_id),
        command=getattr(window, "_terminal_use_command", None),
        status={"kind": "running"},
    )
    return result


def color(value):
    kind = value & 255
    if kind == 0:
        return {"kind": "default"}
    if kind == 1:
        return {"kind": "indexed", "index": (value >> 8) & 255}
    return {
        "kind": "rgb",
        "red": (value >> 24) & 255,
        "green": (value >> 16) & 255,
        "blue": (value >> 8) & 255,
    }


def snapshot(window, options):
    screen = window.screen
    requested = options.get("rect") or {
        "x": 0,
        "y": 0,
        "width": screen.columns,
        "height": screen.lines,
    }
    x = min(requested["x"], screen.columns)
    y = min(requested["y"], screen.lines)
    width = max(0, min(requested["width"], screen.columns - x))
    height = max(0, min(requested["height"], screen.lines - y))
    lines, cells = [], []
    for row in range(y, y + height):
        line = screen.line(row)
        text, row_cells = [], []
        for col in range(x, x + width):
            value = line[col]
            # Kitty stores untouched cells as NUL; they occupy one blank column.
            char_width = 1 if value == "\0" else line.width(col)
            value = " " if value == "\0" else value
            if char_width and col + char_width <= x + width:
                text.append(value)
            if options.get("cells"):
                style = line.cursor_from(col)
                row_cells.append(
                    {
                        "text": value,
                        "wide": char_width > 1,
                        "wide_continuation": char_width == 0,
                        "foreground": color(style.fg),
                        "background": color(style.bg),
                        "bold": bool(style.bold),
                        "dim": bool(style.dim),
                        "italic": bool(style.italic),
                        "underline": bool(style.decoration),
                        "inverse": bool(style.reverse),
                    }
                )
        rendered = "".join(text)
        lines.append(
            rendered.rstrip(" ")
            if options.get("trim_trailing_spaces", True)
            else rendered
        )
        cells.append(row_cells)
    cursor = screen.cursor
    inside = x <= cursor.x < x + width and y <= cursor.y < y + height
    cell_width, cell_height = cell_size_for_window(window.os_window_id)
    return {
        "window_id": window.id,
        "full_size": {"cols": screen.columns, "rows": screen.lines},
        "cell_size": {"width": cell_width, "height": cell_height},
        "rect": {
            "requested": requested,
            "effective": {"x": x, "y": y, "width": width, "height": height},
        },
        "lines": lines,
        "text": "\n".join(lines),
        "cursor": {
            "x": cursor.x,
            "y": cursor.y,
            "visible": screen.cursor_visible,
            "inside_rect": inside,
            "relative_x": cursor.x - x if inside else None,
            "relative_y": cursor.y - y if inside else None,
        },
        "alternate_screen": screen.is_using_alternate_linebuf(),
        "application_cursor": screen.cursor_key_mode,
        "bracketed_paste": screen.in_bracketed_paste_mode,
        "cells": cells if options.get("cells") else None,
        "status": {"kind": "running"},
    }


def send_input(window, events):
    from kitty.options.utils import parse_shortcut
    from kitty.utils import sanitize_for_bracketed_paste

    buttons = {"left": 1, "middle": 2, "right": 3}
    modifiers = {
        "shift": GLFW_MOD_SHIFT,
        "ctrl": GLFW_MOD_CONTROL,
        "alt": GLFW_MOD_ALT,
        "super": GLFW_MOD_SUPER,
    }
    for event in events:
        if event["kind"] == "key":
            parse_shortcut(event["key"])
        elif event["kind"] == "mouse":
            if not (
                0 <= event["col"] < window.screen.columns
                and 0 <= event["row"] < window.screen.lines
            ):
                raise ValueError("Mouse coordinates are outside the terminal grid")
    sent = []
    for event in events:
        kind = event["kind"]
        if kind == "key":
            sent.append(not window.send_key(event["key"]))
        elif kind in ("text", "paste", "raw"):
            data = (
                base64.b64decode(event["data"])
                if kind == "raw"
                else event["text"].encode()
            )
            if kind == "paste" and window.screen.in_bracketed_paste_mode:
                data = b"\x1b[200~" + sanitize_for_bracketed_paste(data) + b"\x1b[201~"
            sent.append(window.write_to_child(data) is None)
        else:
            mods = 0
            for modifier in event.get("modifiers", []):
                mods |= modifiers[modifier]
            action = event["action"]
            count = 1
            if action == "scroll":
                button = {"up": 4, "down": 5, "left": 6, "right": 7}[event["direction"]]
                action_code = PRESS
                count = event["steps"]
            else:
                button = buttons[event["button"]] if event.get("button") else 0
                action_code = {
                    "press": PRESS,
                    "release": RELEASE,
                    "move": DRAG if button else MOVE,
                }[action]
            emitted = False
            for _ in range(count):
                emitted = (
                    send_mouse_event(
                        window.screen,
                        event["col"],
                        event["row"],
                        button,
                        action_code,
                        mods,
                    )
                    or emitted
                )
            sent.append(emitted)
    return sent


def main(args):
    pass


@result_handler(no_ui=True)
def handle_result(args, answer, target_window_id, boss):
    request = json.loads(args[1])
    action = request["action"]
    if action == "hello":
        return json.dumps({"instance_id": instance_id(boss), "pid": os.getpid()})
    if action == "launch":
        window_id = int(
            boss.call_remote_control(boss.active_window, tuple(request["arguments"]))
        )
        watch(boss.window_id_map[window_id])
        return json.dumps(window_id)
    window = boss.window_id_map[request["window_id"]]
    watch(window)
    if action == "inspect":
        result = window_info(window)
    elif action == "read":
        result = snapshot(window, request)
    elif action == "input":
        result = send_input(window, request["events"])
    elif action == "font_size":
        boss._change_font_size({window.os_window_id: request["size"]})
        result = None
    elif action == "signal":
        signum = getattr(signal, "SIG" + request["signal"].removeprefix("SIG"))
        target = request["target"]
        if target == "foreground":
            window.signal_child(signum)
        elif target == "process":
            os.kill(window.child.pid, signum)
        else:
            os.killpg(os.getpgid(window.child.pid), signum)
        result = None
    else:
        raise ValueError(f"Unknown terminal-use operation: {action}")
    return json.dumps(result, ensure_ascii=False)


on_close._terminal_use_callback = "on_close"
on_cmd_startstop._terminal_use_callback = "on_cmd_startstop"
