#!/usr/bin/env python3
"""Render Markdown into styled HTML using a fixed template with syntax highlighting."""

from __future__ import annotations

import argparse
import datetime as dt
import html
import re
from pathlib import Path
from typing import Dict, List, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render markdown into template-based HTML")
    parser.add_argument("input", help="Input markdown file path")
    parser.add_argument("--output", "-o", help="Output html file path")
    parser.add_argument("--template", "-t", help="Template html file path")
    parser.add_argument("--title", help="Force document title")
    parser.add_argument("--toc-min-level", type=int, default=2, help="TOC minimum heading level")
    parser.add_argument("--toc-max-level", type=int, default=3, help="TOC maximum heading level")
    return parser.parse_args()


def load_render_libs():
    try:
        import markdown as md  # type: ignore
    except Exception as exc:
        raise SystemExit(
            "Missing dependency: markdown. Install with: pip install markdown pygments"
        ) from exc

    try:
        from pygments.formatters import HtmlFormatter  # type: ignore
    except Exception as exc:
        raise SystemExit(
            "Missing dependency: pygments. Install with: pip install markdown pygments"
        ) from exc

    return md, HtmlFormatter


def slugify(text: str) -> str:
    s = text.strip().lower()
    s = re.sub(r"[`*_~]", "", s)
    s = re.sub(r"[^\w\-\s\u4e00-\u9fff]", "", s)
    s = re.sub(r"[\s_]+", "-", s)
    s = re.sub(r"-+", "-", s).strip("-")
    return s or "section"


def extract_headings(md_text: str) -> List[Tuple[int, str, str]]:
    headings: List[Tuple[int, str, str]] = []
    used: Dict[str, int] = {}
    for line in md_text.splitlines():
        m = re.match(r"^(#{1,6})\s+(.+?)\s*$", line)
        if not m:
            continue
        level = len(m.group(1))
        title = m.group(2).strip()
        base = slugify(title)
        used[base] = used.get(base, 0) + 1
        anchor = base if used[base] == 1 else f"{base}-{used[base]}"
        headings.append((level, title, anchor))
    return headings


def apply_heading_ids(html_text: str, headings: List[Tuple[int, str, str]]) -> str:
    idx = 0

    def repl(match: re.Match[str]) -> str:
        nonlocal idx
        level = int(match.group(1))
        body = match.group(2)
        if idx >= len(headings):
            return match.group(0)
        expected_level, _, anchor = headings[idx]
        if level != expected_level:
            return match.group(0)
        idx += 1
        return f'<h{level} id="{anchor}">{body}</h{level}>'

    return re.sub(r"<h([1-6])>(.*?)</h\1>", repl, html_text, flags=re.S)


def render_toc(headings: List[Tuple[int, str, str]], min_level: int, max_level: int) -> str:
    rows: List[str] = []
    for level, title, anchor in headings:
        if level < min_level or level > max_level:
            continue
        cls = f"lv{level}"
        rows.append(f'<li class="{cls}"><a href="#{anchor}">{html.escape(title)}</a></li>')
    if not rows:
        return '<div class="empty-toc">No headings in selected range.</div>'
    return "<ul>\n" + "\n".join(rows) + "\n</ul>"


def markdown_to_html(md_text: str) -> Tuple[str, str]:
    md, HtmlFormatter = load_render_libs()

    content_html = md.markdown(
        md_text,
        extensions=["fenced_code", "tables", "sane_lists", "codehilite"],
        extension_configs={
            "codehilite": {
                "guess_lang": False,
                "linenums": False,
                "css_class": "codehilite",
                "use_pygments": True,
            }
        },
    )

    highlight_css = HtmlFormatter(style="github-dark", cssclass="codehilite").get_style_defs(".codehilite")
    return content_html, highlight_css


def resolve_template_path(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).expanduser().resolve()
    return (Path(__file__).resolve().parent.parent / "assets" / "doc-template.html").resolve()


def main() -> None:
    args = parse_args()

    input_path = Path(args.input).expanduser().resolve()
    if not input_path.exists():
        raise SystemExit(f"Input markdown not found: {input_path}")

    template_path = resolve_template_path(args.template)
    if not template_path.exists():
        raise SystemExit(f"Template not found: {template_path}")

    output_path = Path(args.output).expanduser().resolve() if args.output else input_path.with_suffix(".html")

    md_text = input_path.read_text(encoding="utf-8")
    headings = extract_headings(md_text)

    title = args.title
    if not title:
        for level, text, _ in headings:
            if level == 1:
                title = text
                break
    if not title:
        title = input_path.stem

    content_html, highlight_css = markdown_to_html(md_text)
    content_html = apply_heading_ids(content_html, headings)
    toc_html = render_toc(headings, args.toc_min_level, args.toc_max_level)

    template = template_path.read_text(encoding="utf-8")
    rendered = (
        template.replace("{{TITLE}}", html.escape(title))
        .replace("{{TOC}}", toc_html)
        .replace("{{CONTENT}}", content_html)
        .replace("{{HIGHLIGHT_CSS}}", highlight_css)
        .replace("{{UPDATED_AT}}", dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC"))
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered, encoding="utf-8")

    print(f"Markdown: {input_path}")
    print(f"Template: {template_path}")
    print(f"HTML: {output_path}")


if __name__ == "__main__":
    main()
