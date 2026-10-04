"""Read the live workspace on a separate thread; never evaluate browser-supplied code."""

from itertools import islice
import codecs
import json
import os
import sys
import threading
import time
import types

import psutil
import terminal_use

PAGE_SIZE = 50
RAW_PREVIEW_BYTES = 65_536


def array_module(value: object):
    """Recognize exact ndarrays only when user code has already imported NumPy."""
    module = sys.modules.get("numpy")
    if type(module) is types.ModuleType and type(value) is module.__dict__.get("ndarray"):
        return module
    return None


def type_name(value: object) -> str:
    """Read the type's built-in name descriptor without invoking a custom metaclass getter."""
    return type.__dict__["__name__"].__get__(type(value))


def summarize(value: object) -> dict:
    """Describe known data types without calling user-defined repr, properties, or iterators."""
    kind = type(value)
    result = {"type": type_name(value), "summary": "", "expandable": False}
    if value is None or kind in (bool, float, complex):
        result["summary"] = repr(value)
    elif kind is int:
        result["summary"] = str(value) if value.bit_length() < 4096 else f"整数 · {value.bit_length()} bits"
    elif kind in (str, bytes):
        result.update(size=len(value), summary=repr(value[:160]) + ("…" if len(value) > 160 else ""))
    elif kind in (list, tuple, dict, set, frozenset):
        result.update(size=len(value), summary=f"{len(value)} 项", expandable=kind in (list, tuple, dict))
    elif kind is terminal_use.Session:
        info = value.inspect()
        result.update(terminalId=info["id"], summary=f"{info['id']} · {'已关闭' if info['closed'] else info['status']['kind']}")
    elif kind is types.ModuleType:
        name = value.__dict__.get("__name__")
        result["summary"] = name if type(name) is str else ""
    elif array_module(value) is not None:
        result.update(size=int(value.size), summary=f"shape={value.shape} · {value.dtype.name}",
                      expandable=value.dtype.kind in "biufcUS" and not value.dtype.hasobject)
    return result


def category(value: object) -> str:
    if type(value) is types.ModuleType:
        return "module"
    if type(value) in (types.FunctionType, types.BuiltinFunctionType):
        return "function"
    return "data"


def namespace_items(namespace: dict) -> list[tuple[str, object]]:
    """Copy name bindings, not object contents, so cell assignments cannot resize this traversal."""
    return [(name, value) for name, value in namespace.copy().items()
            if type(name) is str and not name.startswith("__") and name != "display_image"]


def variables(namespace: dict, request: dict) -> dict:
    """Filter current bindings before describing only the requested page."""
    search = request.get("search", "").casefold()
    items = [(name, value) for name, value in namespace_items(namespace)
             if (request.get("definitions", False) or category(value) == "data")
             and (not search or search in name.casefold() or search in type_name(value).casefold())]
    items.sort(key=lambda item: item[0])
    offset = request.get("offset", 0)
    return {"view": "variables", "total": len(items), "offset": offset, "pageSize": PAGE_SIZE,
            "variables": [{"name": name, "category": category(value), **summarize(value)}
                          for name, value in items[offset:offset + PAGE_SIZE]]}


def child_at(value: object, selector: dict) -> object:
    """Follow a current container position; dictionary positions are not durable object identities."""
    index = selector["index"]
    if selector["kind"] == "entry" and type(value) is dict:
        entry = next(islice(value.items(), index, index + 1), None)
        if entry is not None:
            return entry[1]
    if selector["kind"] == "index":
        if type(value) in (list, tuple):
            return value[index]
        if array_module(value) is not None and summarize(value)["expandable"]:
            return value.flat[index].item()
    raise KeyError("value path is no longer present")


def inspect_value(namespace: dict, request: dict) -> dict:
    """Resolve a live path and read one page, flattening only the requested ndarray slice."""
    name, path = request["name"], request["path"]
    value = namespace[name]
    for selector in path:
        value = child_at(value, selector)
    description = summarize(value)
    offset = request.get("offset", 0)
    children = []
    if type(value) is dict:
        for index, (key, child) in enumerate(islice(value.items(), offset, offset + PAGE_SIZE), offset):
            children.append({"label": summarize(key)["summary"], "selector": {"kind": "entry", "index": index},
                             **summarize(child)})
    elif type(value) in (list, tuple):
        for index, child in enumerate(value[offset:offset + PAGE_SIZE], offset):
            children.append({"label": f"[{index}]", "selector": {"kind": "index", "index": index}, **summarize(child)})
    elif array_module(value) is not None and description["expandable"]:
        for index, child in enumerate(value.flat[offset:offset + PAGE_SIZE], offset):
            children.append({"label": f"flat[{index}]", "selector": {"kind": "index", "index": index},
                             **summarize(child.item())})
    return {"view": "value", "name": name, "path": path, "value": description, "children": children,
            "offset": offset, "total": description.get("size", 0) if description["expandable"] else 0,
            "pageSize": PAGE_SIZE}


def terminals(namespace: dict) -> dict:
    """List open SDK handles, including exited processes, with current variable aliases."""
    bindings: dict[str, list[str]] = {}
    for name, value in namespace_items(namespace):
        if type(value) is terminal_use.Session:
            bindings.setdefault(value.id, []).append(name)
    result = []
    for session in terminal_use.list():
        info = session.inspect()
        info["hasError"] = info.pop("error") is not None
        info["variables"] = sorted(bindings.get(info["id"], []))
        info["command"], info["cwd"] = None, None
        if info["status"]["kind"] == "running":
            try:
                process = psutil.Process(info["pid"])
                with process.oneshot():
                    info["command"], info["cwd"] = process.cmdline(), process.cwd()
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
        result.append(info)
    return {"view": "terminals", "terminals": result}


def inspect_terminal(request: dict) -> dict:
    """Read retained output without consuming it or resizing the PTY to the browser viewport."""
    session = next((item for item in terminal_use.list() if item.id == request["id"]), None)
    if session is None:
        raise KeyError("terminal was closed")
    result = {"view": "terminal", "id": session.id, "mode": request["mode"]}
    if request["mode"] == "screen":
        info = session.inspect()
        screen = session.read(rect=(0, 0, min(info["cols"], 240), min(info["rows"], 100)), cells=True)
        result["screen"] = {key: screen[key] for key in (
            "lines", "cells", "full_size", "rect", "cursor", "alternate_screen", "generation")}
        result["screen"]["hasError"] = screen["error"] is not None
    else:
        since = request.get("since")
        output = session.read_raw(max_bytes=1_048_576 if since is None else RAW_PREVIEW_BYTES, since=since)
        data = output["data"][-RAW_PREVIEW_BYTES:]
        # Leave an incomplete UTF-8 character in the ring for the next incremental read.
        decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")
        text = decoder.decode(data, final=output["reader_done"] and not output["truncated"])
        pending_bytes = len(decoder.getstate()[0])
        result["raw"] = {"text": text, "start": output["end"] - len(data), "end": output["end"] - pending_bytes,
                         "lostBytes": output["lost_bytes"], "droppedBytes": output["dropped_bytes"],
                         "truncated": output["truncated"] or len(data) < output["bytes"]}
    return result


def start_observer(namespace: dict, request_fd: int, response_fd: int) -> None:
    """Keep inspection off the execution queue. The GIL can still delay this thread."""
    def serve() -> None:
        with os.fdopen(request_fd, "rb") as requests, os.fdopen(response_fd, "wb", buffering=0) as responses:
            for frame in requests:
                request = json.loads(frame)
                query = request["query"]
                try:
                    view = query["view"]
                    if view == "variables":
                        data = variables(namespace, query)
                    elif view == "value":
                        data = inspect_value(namespace, query)
                    elif view == "terminals":
                        data = terminals(namespace)
                    else:
                        data = inspect_terminal(query)
                    reply = {"requestId": request["requestId"], "status": "ok", "data": data,
                             "sampledAt": int(time.time() * 1000)}
                except (KeyError, IndexError):
                    reply = {"requestId": request["requestId"], "status": "not_found"}
                except Exception as error:
                    reply = {"requestId": request["requestId"], "status": "inspection_failed",
                             "errorType": type_name(error)}
                encoded = (json.dumps(reply, ensure_ascii=True, allow_nan=False) + "\n").encode("utf-8")
                try:
                    while encoded:
                        written = responses.write(encoded)
                        encoded = encoded[written:]
                except BrokenPipeError:
                    return

    threading.Thread(target=serve, name="python-observer", daemon=True).start()
