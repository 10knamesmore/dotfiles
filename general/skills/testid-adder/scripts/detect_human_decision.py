#!/usr/bin/env python3

import subprocess
import sys
import json
from pathlib import Path

def run(cmd: str) -> str:
    return subprocess.check_output(cmd, shell=True, text=True)

def require_file(path: str):
    if not Path(path).exists():
        print(f"ERROR: required file not found: {path}", file=sys.stderr)
        sys.exit(1)

def split_by_file(diff_text: str):
    """
    Very conservative diff splitter.
    Groups diff hunks by file path.
    """
    files = {}
    current_file = None
    buffer = []

    for line in diff_text.splitlines():
        if line.startswith("diff --git"):
            if current_file:
                files.setdefault(current_file, []).append("\n".join(buffer))
            parts = line.split()
            current_file = parts[-1].replace("b/", "")
            buffer = [line]
        else:
            buffer.append(line)

    if current_file:
        files.setdefault(current_file, []).append("\n".join(buffer))

    return files

def main():
    if len(sys.argv) != 2:
        print("Usage: detect_human_decisions.py <ai_proposed.patch>", file=sys.stderr)
        sys.exit(1)

    baseline_patch = sys.argv[1]
    require_file(baseline_patch)

    ai_all = Path(baseline_patch).read_text()
    accepted = run("git diff --cached")
    pending = run("git diff")

    ai_files = split_by_file(ai_all)
    accepted_files = split_by_file(accepted)
    pending_files = split_by_file(pending)

    result = {
        "accepted": [],
        "rejected": [],
        "pending": []
    }

    for file, hunks in ai_files.items():
        if file in accepted_files:
            result["accepted"].append({
                "file": file,
                "hunks": accepted_files[file]
            })
        elif file in pending_files:
            result["pending"].append({
                "file": file,
                "hunks": pending_files[file]
            })
        else:
            result["rejected"].append({
                "file": file,
                "hunks": hunks
            })

    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main()
