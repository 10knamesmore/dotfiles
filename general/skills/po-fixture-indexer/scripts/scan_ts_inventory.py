#!/usr/bin/env python3
"""Lightweight TypeScript inventory scanner for fixtures/ and pages/.

Outputs a JSON array of file entries with exports, classes, and methods.
This is heuristic (regex + brace matching) and intended to assist AI indexing.
"""

import argparse
import json
import os
import re
from pathlib import Path

EXPORT_CLASS_RE = re.compile(r"\bexport\s+class\s+([A-Za-z_][A-Za-z0-9_]*)")
EXPORT_DEFAULT_CLASS_RE = re.compile(r"\bexport\s+default\s+class\s+([A-Za-z_][A-Za-z0-9_]*)")
CLASS_RE = re.compile(r"\bclass\s+([A-Za-z_][A-Za-z0-9_]*)")
EXPORT_CONST_RE = re.compile(r"\bexport\s+const\s+([A-Za-z_][A-Za-z0-9_]*)")
EXPORT_FUNCTION_RE = re.compile(r"\bexport\s+function\s+([A-Za-z_][A-Za-z0-9_]*)")

METHOD_LINE_RE = re.compile(
    r"^\s*(?:public|private|protected|async|static|get|set|readonly|override|\s)*\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("
)

COMMENT_LINE_RE = re.compile(r"^\s*//")
COMMENT_BLOCK_START_RE = re.compile(r"^\s*/\*\*")
COMMENT_BLOCK_END_RE = re.compile(r"\*/\s*$")


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="utf-8", errors="replace")


def collect_leading_comment(lines, index):
    """Collect contiguous comment block immediately above a line index."""
    i = index - 1
    if i < 0:
        return None
    # Skip blank lines
    while i >= 0 and lines[i].strip() == "":
        i -= 1
    if i < 0:
        return None

    if COMMENT_LINE_RE.match(lines[i]):
        comment_lines = []
        while i >= 0 and COMMENT_LINE_RE.match(lines[i]):
            comment_lines.append(lines[i].strip())
            i -= 1
        comment_lines.reverse()
        return "\n".join(comment_lines)

    if COMMENT_BLOCK_END_RE.search(lines[i]) or COMMENT_BLOCK_START_RE.match(lines[i]):
        comment_lines = []
        while i >= 0:
            comment_lines.append(lines[i].rstrip())
            if COMMENT_BLOCK_START_RE.match(lines[i]):
                break
            i -= 1
        comment_lines.reverse()
        return "\n".join(comment_lines)

    return None


def scan_class_methods(lines, start_idx):
    """Scan for methods within a class body by brace matching."""
    methods = []
    brace_depth = 0
    in_class = False

    for i in range(start_idx, len(lines)):
        line = lines[i]
        # Enter class body on first '{'
        if not in_class:
            if "{" in line:
                in_class = True
                brace_depth += line.count("{") - line.count("}")
            continue

        brace_depth += line.count("{") - line.count("}")
        if brace_depth <= 0:
            break

        match = METHOD_LINE_RE.match(line)
        if match:
            name = match.group(1)
            if name == "constructor":
                continue
            comment = collect_leading_comment(lines, i)
            methods.append({"name": name, "comment": comment, "line": i + 1})

    return methods


def classify_kind(path: Path, fixtures_dir: str, pages_dir: str) -> str:
    parts = path.parts
    if fixtures_dir in parts:
        return "fixture"
    if pages_dir in parts:
        return "page"
    return "unknown"


def scan_file(path: Path, fixtures_dir: str, pages_dir: str):
    text = read_text(path)
    lines = text.splitlines()

    exports = []
    for i, line in enumerate(lines):
        for regex, kind in [
            (EXPORT_CLASS_RE, "export_class"),
            (EXPORT_DEFAULT_CLASS_RE, "export_default_class"),
            (EXPORT_CONST_RE, "export_const"),
            (EXPORT_FUNCTION_RE, "export_function"),
        ]:
            match = regex.search(line)
            if match:
                exports.append(
                    {
                        "name": match.group(1),
                        "kind": kind,
                        "comment": collect_leading_comment(lines, i),
                        "line": i + 1,
                    }
                )

    classes = []
    for i, line in enumerate(lines):
        match = CLASS_RE.search(line)
        if match:
            name = match.group(1)
            comment = collect_leading_comment(lines, i)
            methods = scan_class_methods(lines, i)
            classes.append(
                {
                    "name": name,
                    "comment": comment,
                    "line": i + 1,
                    "methods": methods,
                }
            )

    return {
        "path": str(path),
        "kind": classify_kind(path, fixtures_dir, pages_dir),
        "exports": exports,
        "classes": classes,
    }


def main():
    parser = argparse.ArgumentParser(description="Scan fixtures/ and pages/ TypeScript for inventory")
    parser.add_argument("--root", default=".", help="Repository root")
    parser.add_argument("--fixtures", default="fixtures", help="Fixtures directory name")
    parser.add_argument("--pages", default="pages", help="Pages directory name")
    parser.add_argument("--out", default="-", help="Output JSON file (default: stdout)")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    files = []
    for base in [args.fixtures, args.pages]:
        dir_path = root / base
        if not dir_path.exists():
            continue
        for path in dir_path.rglob("*.ts"):
            if path.name.endswith(".d.ts"):
                continue
            files.append(path)

    inventory = [scan_file(path, args.fixtures, args.pages) for path in sorted(files)]

    output = json.dumps(inventory, ensure_ascii=True, indent=2)
    if args.out == "-":
        print(output)
    else:
        out_path = Path(args.out)
        out_path.write_text(output, encoding="utf-8")


if __name__ == "__main__":
    main()
