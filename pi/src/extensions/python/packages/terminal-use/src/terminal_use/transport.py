"""Run Kitty's maintained remote-control client against one explicit socket."""

import json
import subprocess
from pathlib import Path
from typing import Any

BRIDGE = Path.home() / ".local/share/terminal-use/bridge.py"
SOCKET_DIRECTORY = Path.home() / ".local/state/kitty"


class Transport:
    def __init__(self, address: str, timeout: float = 10.0):
        self.address = address
        self.timeout = timeout

    def command(self, *arguments: str, timeout: float | None = None) -> str:
        return self.command_bytes(*arguments, timeout=timeout).decode("utf-8")

    def command_bytes(self, *arguments: str, timeout: float | None = None) -> bytes:
        result = subprocess.run(
            ["kitten", "@", "--to", self.address, *arguments],
            capture_output=True,
            timeout=self.timeout if timeout is None else timeout,
            check=False,
        )
        if result.returncode:
            raise RuntimeError(result.stderr.decode("utf-8").strip())
        return result.stdout

    def bridge(
        self,
        action: str,
        window_id: int | None = None,
        *,
        timeout: float | None = None,
        **arguments: Any,
    ) -> Any:
        request = {"action": action, **arguments}
        match = []
        if window_id is not None:
            request["window_id"] = window_id
            match = ["--match", f"id:{window_id}"]
        output = self.command(
            "kitten",
            *match,
            str(BRIDGE),
            json.dumps(request, ensure_ascii=False),
            timeout=timeout,
        )
        return json.loads(output)
