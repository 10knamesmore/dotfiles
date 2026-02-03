#!/usr/bin/env python3
import csv
import json
import sys
from pathlib import Path


# 语义字段 → 多语言 header 映射
COLUMN_ALIASES = {
    "title": ["用例名称", "Test case name"],
    "preconditions": ["前置条件", "Precondition"],
    "steps": ["步骤描述", "Step description"],
    "expected": ["预期结果", "Expected result"],
    "module1": ["一级模块", "Level-1 module"],
    "module2": ["二级模块", "Level-2 module"],
    "module3": ["三级模块", "Level-3 module"],
}


def normalize(s):
    return s.strip().lower() if s else ""


def resolve_columns(headers):
    """把 CSV header 映射到标准字段名"""
    resolved = {}
    normalized_headers = {normalize(h): h for h in headers}

    for key, aliases in COLUMN_ALIASES.items():
        for alias in aliases:
            a = normalize(alias)
            if a in normalized_headers:
                resolved[key] = normalized_headers[a]
                break
        else:
            resolved[key] = None

    return resolved


def split_hash_lines(text: str):
    if not text:
        return []
    lines = []
    for raw in str(text).splitlines():
        raw = raw.strip()
        if raw.startswith("#"):
            raw = raw[1:].strip()
        if raw:
            lines.append(raw)
    return lines


def build_page(row, colmap):
    parts = [
        row.get(colmap["module1"], ""),
        row.get(colmap["module2"], ""),
        row.get(colmap["module3"], ""),
    ]
    parts = [str(p).strip() for p in parts if p]
    return "_".join(parts) if parts else "unknown"


def clean_row(row, colmap):
    title = str(row.get(colmap["title"], "")).strip()
    pre = str(row.get(colmap["preconditions"], "")).strip()
    steps = split_hash_lines(row.get(colmap["steps"], ""))
    expected = split_hash_lines(row.get(colmap["expected"], ""))

    return {
        "title": title,
        "page": build_page(row, colmap),
        "preconditions": [pre] if pre else [],
        "steps": steps,
        "expected": expected,
    }


def main():
    if len(sys.argv) < 2:
        print("Usage: clean_cases.py input.csv [output.json]")
        sys.exit(1)

    input_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    cases = []

    with input_path.open("r", encoding="utf-8-sig") as f:
        reader = csv.DictReader(f)
        headers = reader.fieldnames or []

        colmap = resolve_columns(headers)

        for row in reader:
            cases.append(clean_row(row, colmap))

    data = json.dumps(cases, ensure_ascii=False, indent=2)

    if output_path:
        output_path.write_text(data, encoding="utf-8")
    else:
        print(data)


if __name__ == "__main__":
    main()
