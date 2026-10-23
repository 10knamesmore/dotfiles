"""Write the installed Python package inventory after dots sync prepares the environment."""

import json
import platform
import sys
from importlib import metadata
from pathlib import Path


def main() -> None:
    packages = sorted(
        (
            {"name": distribution.metadata["Name"], "version": distribution.version}
            for distribution in metadata.distributions()
        ),
        key=lambda package: package["name"].casefold(),
    )
    environment = {
        "executable": sys.executable,
        "version": platform.python_version(),
        "packages": packages,
    }
    manifest = (
        Path(__file__).resolve().parents[4] / ".gen" / "pi" / "python-environment.json"
    )
    manifest.parent.mkdir(parents=True, exist_ok=True)
    temporary = manifest.with_suffix(".json.tmp")
    temporary.write_text(
        json.dumps(environment, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    temporary.replace(manifest)
    print(
        f"Python {environment['version']}: wrote {len(packages)} installed packages to {manifest}"
    )


if __name__ == "__main__":
    main()
