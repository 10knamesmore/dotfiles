"""Find Neovim instances from their private, event-updated runtime records."""

import json
import os
import stat
import sys
from pathlib import Path
from typing import Any

import psutil


def runtime_directory() -> Path:
    """Return the shared registry directory: Linux runtime memory or macOS user temp."""
    variable = "TMPDIR" if sys.platform == "darwin" else "XDG_RUNTIME_DIR"
    value = os.environ.get(variable)
    if not value:
        raise RuntimeError(f"{variable} must be set to discover Neovim instances")
    return Path(value) / "pi-nvim"


def discover() -> list[dict[str, Any]]:
    """List live current-user instances without acquiring their Pi connection.

    cwd, current_file, workspace_roots and connection are registration summaries.
    The connection handshake verifies instance identity against the running editor.
    Records left by dead processes are ignored; only Neovim writes its records.
    """
    instances = []
    for path in runtime_directory().glob("*.json"):
        try:
            record = json.loads(path.read_text())
            process = psutil.Process(record["pid"])
            if process.uids().real != os.getuid():
                continue
            if process.create_time() > record["started_at"] + 1:
                continue
            if not stat.S_ISSOCK(Path(record["socket"]).stat().st_mode):
                continue
            instances.append(record)
        except (FileNotFoundError, ProcessLookupError, psutil.NoSuchProcess):
            continue
    return sorted(instances, key=lambda item: (item["cwd"], item["pid"]))


def resolve(instance_id: str) -> dict[str, Any]:
    """Resolve an exact discovery ID so restarted editors require a fresh selection."""
    for instance in discover():
        if instance["instance_id"] == instance_id:
            return instance
    raise LookupError(f"Neovim instance {instance_id!r} is no longer available")
