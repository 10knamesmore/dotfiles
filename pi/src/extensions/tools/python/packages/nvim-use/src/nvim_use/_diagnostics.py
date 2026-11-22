"""Log connection and request outcomes without source text or LSP payloads."""

import json
import os
import tempfile
import threading
import time
from pathlib import Path

_lock = threading.Lock()


def event(operation: str, outcome: str, **fields: object) -> None:
    """Append operation metadata to NVIM_USE_LOG or a per-worker temporary log."""
    path = Path(
        os.environ.get(
            "NVIM_USE_LOG", Path(tempfile.gettempdir()) / f"nvim-use-{os.getpid()}.log"
        )
    )
    try:
        with _lock:
            path.parent.mkdir(parents=True, exist_ok=True)
            mode = "w" if path.exists() and path.stat().st_size >= 1_048_576 else "a"
            with path.open(mode) as output:
                output.write(
                    json.dumps(
                        {
                            "time": time.time(),
                            "operation": operation,
                            "outcome": outcome,
                            **fields,
                        }
                    )
                    + "\n"
                )
    except OSError:
        pass
