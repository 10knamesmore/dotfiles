"""Record browser lifecycle and operation outcomes without page or input data."""

import json
import os
from pathlib import Path
import tempfile
import threading
import time

_lock = threading.Lock()


def event(operation: str, outcome: str, **fields: str | int | float | None) -> None:
    """Append bounded diagnostic records; logging failures do not change browser actions."""
    path = Path(
        os.environ.get(
            "BROWSER_USE_LOG",
            Path(tempfile.gettempdir()) / f"browser-use-{os.getpid()}.log",
        )
    )
    record = {"time": time.time(), "operation": operation, "outcome": outcome, **fields}
    try:
        with _lock:
            path.parent.mkdir(parents=True, exist_ok=True)
            mode = "w" if path.exists() and path.stat().st_size >= 1_048_576 else "a"
            with path.open(mode) as output:
                output.write(json.dumps(record) + "\n")
    except OSError:
        pass
