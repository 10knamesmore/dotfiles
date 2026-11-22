"""Discover, launch and connect to independent Kitty processes."""

import json
import os
import subprocess
import time
import uuid
from pathlib import Path
from typing import Any, Sequence

from .session import Session, duration
from .transport import BRIDGE, SOCKET_DIRECTORY, Transport


class KittyInstance:
    def __init__(self, address: str, *, timeout: float = 10.0):
        self.socket = address
        self._transport = Transport(address, timeout)
        identity = self._transport.bridge("hello")
        self.id: str = identity["instance_id"]
        self.pid: int = identity["pid"]
        self.event_directory = Path.home() / ".local/state/terminal-use" / self.id

    def __repr__(self) -> str:
        return f"KittyInstance(pid={self.pid}, socket={self.socket!r})"

    def list_windows(self) -> list[dict[str, Any]]:
        result = []
        for os_window in json.loads(self._transport.command("ls")):
            for tab in os_window["tabs"]:
                for window in tab["windows"]:
                    del window["env"]
                    result.append(
                        {
                            **window,
                            "os_window_id": os_window["id"],
                            "tab_id": tab["id"],
                            "tab_title": tab["title"],
                            "rows": window["lines"],
                            "cols": window["columns"],
                        }
                    )
        return result

    def attach(self, window_id: int) -> Session:
        session = Session(self, window_id)
        session.inspect()
        return session

    def _launch(
        self,
        kind: str,
        argv: Sequence[str] | None,
        *,
        cwd: str | None,
        env: dict[str, str] | None,
        title: str | None,
        cols: int | None,
        rows: int | None,
    ) -> Session:
        if (cols is None) != (rows is None):
            raise ValueError("Specify both cols and rows")
        if cols is not None and rows is not None and (cols <= 0 or rows <= 0):
            raise ValueError("cols and rows must be positive")
        arguments = ["launch", "--type", kind, "--watcher", str(BRIDGE)]
        if cwd is not None:
            arguments.extend(("--cwd", str(Path(cwd).expanduser().resolve())))
        if title is not None:
            arguments.extend(("--title", title))
        for key, value in (env or {}).items():
            arguments.extend(("--env", f"{key}={value}"))
        if argv is not None:
            if not argv or isinstance(argv, str):
                raise ValueError(
                    "argv must be a nonempty sequence of program arguments"
                )
            arguments.extend(("--", *argv))
        window_id = self._transport.bridge("launch", arguments=arguments)
        session = Session(self, window_id)
        if cols is not None and rows is not None:
            session.resize(cols=cols, rows=rows)
        return session

    def open_window(
        self,
        argv: Sequence[str] | None = None,
        *,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
        title: str | None = None,
        cols: int | None = None,
        rows: int | None = None,
    ) -> Session:
        return self._launch(
            "os-window", argv, cwd=cwd, env=env, title=title, cols=cols, rows=rows
        )

    def open_tab(
        self,
        argv: Sequence[str] | None = None,
        *,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
        title: str | None = None,
        cols: int | None = None,
        rows: int | None = None,
    ) -> Session:
        return self._launch(
            "tab", argv, cwd=cwd, env=env, title=title, cols=cols, rows=rows
        )

    def open_split(
        self,
        argv: Sequence[str] | None = None,
        *,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
        title: str | None = None,
        cols: int | None = None,
        rows: int | None = None,
    ) -> Session:
        return self._launch(
            "window", argv, cwd=cwd, env=env, title=title, cols=cols, rows=rows
        )


def connect(*, socket: str | None = None, timeout: float = 10.0) -> KittyInstance:
    """Connect explicitly, or use the KITTY_LISTEN_ON inherited from the parent terminal."""
    return KittyInstance(
        os.environ["KITTY_LISTEN_ON"] if socket is None else socket, timeout=timeout
    )


def list_instances() -> list[dict[str, Any]]:
    instances = []
    for path in sorted(SOCKET_DIRECTORY.glob("control-*.sock")):
        address = f"unix:{path}"
        instance = connect(socket=address)
        instances.append(
            {
                "socket": address,
                "id": instance.id,
                "pid": instance.pid,
                "windows": instance.list_windows(),
            }
        )
    return instances


def launch(
    argv: Sequence[str] | None = None,
    *,
    cwd: str | None = None,
    cols: int = 120,
    rows: int = 40,
    hidden: bool = False,
    timeout: float = 20.0,
) -> Session:
    """Start an independent Kitty and return its initial terminal, running argv or the default shell."""
    duration(timeout, "timeout")
    if timeout == 0 or cols <= 0 or rows <= 0:
        raise ValueError("timeout, cols and rows must be positive")
    if argv is not None and (not argv or isinstance(argv, str)):
        raise ValueError("argv must be a nonempty sequence of program arguments")
    SOCKET_DIRECTORY.mkdir(parents=True, exist_ok=True, mode=0o700)
    path = SOCKET_DIRECTORY / f"control-{uuid.uuid4().hex}.sock"
    arguments = [
        "--listen-on",
        f"unix:{path}",
        "--override",
        "allow_remote_control=socket-only",
        "--override",
        f"watcher={BRIDGE}",
        "--override",
        "remember_window_size=no",
        "--override",
        f"initial_window_width={cols}c",
        "--override",
        f"initial_window_height={rows}c",
        "--session",
        "none",
    ]
    if hidden:
        arguments.append("--start-as=hidden")
    if cwd is not None:
        arguments.extend(("--directory", str(Path(cwd).expanduser().resolve())))
    if argv is not None:
        arguments.extend(("--", *argv))
    log_directory = Path.home() / ".local/state/terminal-use"
    log_directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    log_path = log_directory / f"{path.stem}.log"
    with log_path.open("ab") as log:
        subprocess.Popen(
            ["kitty", *arguments],
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=log,
            start_new_session=True,
        )
    deadline = time.monotonic() + timeout
    while not path.exists():
        if time.monotonic() >= deadline:
            raise TimeoutError(f"Kitty did not create {path}; startup log: {log_path}")
        time.sleep(0.05)
    instance = connect(
        socket=f"unix:{path}", timeout=max(0.001, deadline - time.monotonic())
    )
    (initial_window,) = instance.list_windows()
    return instance.attach(window_id=initial_window["id"])
