#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["markdown", "pyyaml"]
# ///
"""将 Markdown 渲染为基于固定模板的 HTML 页面。

提供从 Markdown 源文件到完整 HTML 的一站式转换，包含目录生成、
标题锚点注入与代码块元信息保留。
"""

from __future__ import annotations

import argparse
import datetime as dt
import html
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

BEIJING_TZ = dt.timezone(dt.timedelta(hours=8), name="CST")


def split_frontmatter(md_text: str) -> tuple[Any | None, str]:
    """拆分文档开头的 YAML frontmatter 与正文。

    仅识别位于文件起始位置的 ``---`` 包裹块。

    Args:
        md_text: 原始 Markdown 文本。

    Returns:
        ``(frontmatter_data, markdown_body)``。若不存在 frontmatter，
        ``frontmatter_data`` 为 ``None``，正文为原文。
    """
    if not md_text.startswith("---"):
        return None, md_text

    lines = md_text.splitlines(keepends=True)
    if not lines or lines[0].strip() != "---":
        return None, md_text

    closing_idx: int | None = None
    for idx in range(1, len(lines)):
        if lines[idx].strip() == "---":
            closing_idx = idx
            break

    if closing_idx is None:
        return None, md_text

    raw_frontmatter = "".join(lines[1:closing_idx])
    body = "".join(lines[closing_idx + 1 :])

    try:
        import yaml
    except Exception as exc:
        raise SystemExit(
            "Missing dependency: pyyaml. Install with: pip install pyyaml"
        ) from exc

    parsed = yaml.safe_load(raw_frontmatter)
    return parsed, body


def parse_args() -> argparse.Namespace:
    """解析命令行参数。

    Returns:
        解析结果命名空间，包含 input、output、template、title、
        toc_min_level、toc_max_level、open 字段。
    """
    parser = argparse.ArgumentParser(
        description="Render markdown into template-based HTML"
    )
    _ = parser.add_argument("input", help="Input markdown file path")
    _ = parser.add_argument("--output", "-o", help="Output html file path")
    _ = parser.add_argument("--template", "-t", help="Template html file path")
    _ = parser.add_argument("--title", help="Force document title")
    _ = parser.add_argument(
        "--toc-min-level", type=int, default=2, help="TOC minimum heading level"
    )
    _ = parser.add_argument(
        "--toc-max-level", type=int, default=3, help="TOC maximum heading level"
    )
    _ = parser.add_argument(
        "--open",
        "-O",
        action="store_true",
        help="Open output HTML in browser after rendering",
    )
    _ = parser.add_argument(
        "--keep-md",
        action="store_true",
        help="Keep the source Markdown file after rendering (default: delete it)",
    )
    return parser.parse_args()


def slugify(text: str) -> str:
    """将标题文本转换为 URL 安全的锚点 slug。

    保留中文字符、字母、数字与连字符，空格和下划线统一转为连字符，
    连续连字符合并，首尾连字符去除。

    Args:
        text: 原始标题字符串，可含 Markdown 行内标记。

    Returns:
        小写、无特殊字符的 slug；若结果为空则返回 ``"section"``。
    """
    s = text.strip().lower()
    s = re.sub(r"[`*_~]", "", s)
    s = re.sub(r"[^\w\-\s\u4e00-\u9fff]", "", s)
    s = re.sub(r"[\s_]+", "-", s)
    s = re.sub(r"-+", "-", s).strip("-")
    return s or "section"


@dataclass(frozen=True)
class HeadingSection:
    """Markdown 标题及其对应章节范围。"""

    level: int
    title: str
    anchor: str
    start_line: int
    end_line: int


def extract_headings(md_text: str) -> list[tuple[int, str, str]]:
    """从 Markdown 文本中提取所有标题及其锚点。

    重复标题自动追加 ``-2``、``-3`` 等后缀以保证锚点唯一。
    会跳过围栏代码块中的内容，避免把代码注释误识别为标题。

    Args:
        md_text: Markdown 源文本。

    Returns:
        按出现顺序排列的列表，每项为 ``(级别, 标题文本, 锚点 slug)`` 三元组。
    """
    return [
        (section.level, section.title, section.anchor)
        for section in extract_heading_sections(md_text)
    ]


def extract_heading_sections(md_text: str) -> list[HeadingSection]:
    """从 Markdown 文本中提取标题及其章节范围。"""
    sections: list[HeadingSection] = []
    used: dict[str, int] = {}
    in_fence = False
    lines = md_text.splitlines()
    for idx, line in enumerate(lines):
        if re.match(r"^```", line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = re.match(r"^(#{1,6})\s+(.+?)\s*$", line)
        if not m:
            continue
        level = len(m.group(1))
        title = m.group(2).strip()
        base = slugify(title)
        used[base] = used.get(base, 0) + 1
        anchor = base if used[base] == 1 else f"{base}-{used[base]}"
        sections.append(
            HeadingSection(
                level=level,
                title=title,
                anchor=anchor,
                start_line=idx,
                end_line=len(lines) - 1,
            )
        )

    resolved: list[HeadingSection] = []
    for idx, section in enumerate(sections):
        end_line = len(lines) - 1
        for next_section in sections[idx + 1 :]:
            if next_section.level <= section.level:
                end_line = next_section.start_line - 1
                break
        resolved.append(
            HeadingSection(
                level=section.level,
                title=section.title,
                anchor=section.anchor,
                start_line=section.start_line,
                end_line=end_line,
            )
        )
    return resolved


def build_section_md_map(
    md_text: str, sections: list[HeadingSection]
) -> dict[str, str]:
    """构建标题 id 到原始 Markdown 章节文本的映射。"""
    lines = md_text.splitlines()
    result: dict[str, str] = {}
    for section in sections:
        result[section.anchor] = "\n".join(
            lines[section.start_line : section.end_line + 1]
        ).strip("\n")
    return result


def apply_heading_ids(html_text: str, headings: list[tuple[int, str, str]]) -> str:
    """将锚点 id 注入 HTML 中的标题元素。

    按出现顺序将 ``extract_headings`` 生成的锚点写入对应 ``<hN>`` 标签，
    级别不匹配时跳过，超出范围时原样保留。

    Args:
        html_text: 由 Markdown 渲染器生成的 HTML 字符串。
        headings: ``extract_headings`` 返回的标题列表。

    Returns:
        所有标题均带有 ``id`` 属性的 HTML 字符串。
    """
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


def render_toc(
    headings: list[tuple[int, str, str]], min_level: int, max_level: int
) -> str:
    """生成文章目录的 HTML 片段。

    仅包含级别在 ``[min_level, max_level]`` 范围内的标题，
    以 ``lv{N}`` class 区分缩进层级。

    Args:
        headings: ``extract_headings`` 返回的标题列表。
        min_level: 目录包含的最小标题级别（含）。
        max_level: 目录包含的最大标题级别（含）。

    Returns:
        ``<ul>`` 列表 HTML；若无符合条件的标题则返回提示占位 div。
    """
    rows: list[str] = []
    for level, title, anchor in headings:
        if level < min_level or level > max_level:
            continue
        cls = f"lv{level}"
        rows.append(
            f'<li class="{cls}"><a href="#{anchor}">{html.escape(title)}</a></li>'
        )
    if not rows:
        return '<div class="empty-toc">No headings in selected range.</div>'
    return "<ul>\n" + "\n".join(rows) + "\n</ul>"


@dataclass(frozen=True)
class CodeFenceMeta:
    """围栏代码块的元信息。"""

    lang: str
    title: str
    highlight_lines: str


def parse_fence_meta(info: str) -> CodeFenceMeta:
    """解析围栏代码块首行的语言与附加元信息。"""
    raw = info.strip()
    if not raw:
        return CodeFenceMeta(lang="", title="", highlight_lines="")

    parts = raw.split(maxsplit=1)
    lang = parts[0]
    rest = parts[1] if len(parts) > 1 else ""

    title_match = re.search(r'title\s*=\s*"([^"]+)"', rest)
    if not title_match:
        title_match = re.search(r"title\s*=\s*'([^']+)'", rest)

    line_match = re.search(r"\{([^}]+)\}", rest)

    return CodeFenceMeta(
        lang=lang,
        title=title_match.group(1).strip() if title_match else "",
        highlight_lines=line_match.group(1).strip() if line_match else "",
    )


def render_code_block(code: str, meta: CodeFenceMeta, index: int) -> str:
    """将围栏代码块渲染为供前端 Shiki 接管的基础 DOM。"""
    code_id = f"code-{index}"
    attrs = [f'class="code-block"', f'id="{code_id}"', f'data-code-id="{code_id}"']
    if meta.lang:
        attrs.append(f'data-lang="{html.escape(meta.lang, quote=True)}"')
    if meta.title:
        attrs.append(f'data-title="{html.escape(meta.title, quote=True)}"')
    if meta.highlight_lines:
        attrs.append(
            f'data-highlight-lines="{html.escape(meta.highlight_lines, quote=True)}"'
        )

    lang_badge = html.escape(meta.lang or "text")
    title_html = (
        f'<span class="code-title">{html.escape(meta.title)}</span>'
        if meta.title
        else ""
    )
    raw_code = html.escape(code.rstrip("\n"))

    return (
        f"<div {' '.join(attrs)}>\n"
        '  <div class="code-toolbar">\n'
        '    <div class="code-toolbar-meta">\n'
        f'      <span class="code-lang-chip">{lang_badge}</span>\n'
        f"      {title_html}\n"
        "    </div>\n"
        '    <div class="code-toolbar-actions">\n'
        '      <button type="button" class="code-edit-btn" data-original-text="编辑" aria-label="编辑代码">编辑</button>\n'
        '      <button type="button" class="code-reset-btn" data-original-text="恢复原样" aria-label="恢复代码原样">恢复原样</button>\n'
        '      <button type="button" class="code-copy-btn" data-original-text="复制" aria-label="复制代码">复制</button>\n'
        "    </div>\n"
        "  </div>\n"
        '  <div class="code-scroll">\n'
        f"    <pre><code>{raw_code}</code></pre>\n"
        "  </div>\n"
        "</div>"
    )


def preprocess_code_fences(md_text: str) -> str:
    """将 Markdown 围栏代码块替换为带元信息的原始 HTML 块。"""
    result: list[str] = []
    lines = md_text.splitlines()
    in_fence = False
    fence_meta = CodeFenceMeta(lang="", title="", highlight_lines="")
    buffer: list[str] = []
    block_index = 0

    for line in lines:
        start = re.match(r"^```([^\s`]*)\s*(.*?)\s*$", line)
        if not in_fence and start:
            in_fence = True
            info = (start.group(1) + " " + start.group(2)).strip()
            fence_meta = parse_fence_meta(info)
            buffer = []
            continue

        if in_fence and re.match(r"^```\s*$", line):
            if fence_meta.lang == "mermaid":
                result.append("")
                result.append(f'<div class="mermaid">\n{chr(10).join(buffer)}\n</div>')
                result.append("")
            else:
                block_index += 1
                result.append("")
                result.append(render_code_block("\n".join(buffer), fence_meta, block_index))
                result.append("")
            in_fence = False
            buffer = []
            continue

        if in_fence:
            buffer.append(line)
        else:
            result.append(line)

    if in_fence:
        result.append("```" + fence_meta.lang)
        result.extend(buffer)

    return "\n".join(result)


def markdown_to_html(md_text: str) -> str:
    """将 Markdown 文本渲染为 HTML 正文。"""
    try:
        import markdown as md
    except Exception as exc:
        raise SystemExit(
            "Missing dependency: markdown. Install with: pip install markdown"
        ) from exc

    processed = preprocess_code_fences(md_text)
    return cast(
        str,
        md.markdown(
            processed,
            extensions=[
                "tables",
                "sane_lists",
                "attr_list",
                "def_list",
                "footnotes",
            ],
        ),
    )


def embed_local_images(html_text: str, base_dir: Path) -> str:
    """将 HTML 中本地图片路径替换为 base64 data URI，使 HTML 自包含。"""
    import base64
    import mimetypes

    def replace_src(m: re.Match[str]) -> str:
        before, src, after = m.group(1), m.group(2), m.group(3)
        if src.startswith(("data:", "http://", "https://", "//")):
            return m.group(0)
        img_path = (base_dir / src).resolve()
        if not img_path.exists():
            print(f"Warning: image not found: {img_path}", flush=True)
            return m.group(0)
        mime, _ = mimetypes.guess_type(str(img_path))
        if not mime:
            mime = "application/octet-stream"
        b64 = base64.b64encode(img_path.read_bytes()).decode()
        return f'{before}data:{mime};base64,{b64}{after}'

    return re.sub(r'(<img[^>]+src=")([^"]+)(")', replace_src, html_text)


def resolve_template_path(explicit: str | None) -> Path:
    """解析 HTML 模板文件的绝对路径。

    若未显式指定，则使用脚本同级 ``../assets/doc-template.html``。

    Args:
        explicit: 命令行传入的模板路径；``None`` 表示使用默认模板。

    Returns:
        模板文件的绝对路径（不保证文件存在）。
    """
    if explicit:
        return Path(explicit).expanduser().resolve()
    return (
        Path(__file__).resolve().parent.parent / "assets" / "doc-template.html"
    ).resolve()


def main() -> None:
    """执行完整的 Markdown → HTML 渲染流程并输出文件路径。

    流程：解析参数 → 提取标题与代码语言 → 渲染 HTML → 注入锚点与语言标签
    → 生成目录 → 填充模板 → 写入文件；若指定 ``--open`` 则在浏览器中打开结果。

    Raises:
        SystemExit: 当输入文件或模板文件不存在时。
    """
    args = parse_args()

    input_path = Path(args.input).expanduser().resolve()
    if not input_path.exists():
        raise SystemExit(f"Input markdown not found: {input_path}")

    template_path = resolve_template_path(args.template)
    if not template_path.exists():
        raise SystemExit(f"Template not found: {template_path}")

    output_path = (
        Path(args.output).expanduser().resolve()
        if args.output
        else input_path.with_suffix(".html")
    )

    md_text = input_path.read_text(encoding="utf-8")
    frontmatter_data, body_md = split_frontmatter(md_text)
    sections = extract_heading_sections(body_md)
    headings = [(section.level, section.title, section.anchor) for section in sections]
    section_md_map = build_section_md_map(body_md, sections)

    title = args.title
    if not title:
        for level, text, _ in headings:
            if level == 1:
                title = text
                break
    if not title:
        title = input_path.stem

    content_html = markdown_to_html(body_md)
    content_html = apply_heading_ids(content_html, headings)
    toc_html = render_toc(headings, args.toc_min_level, args.toc_max_level)

    template = template_path.read_text(encoding="utf-8")
    rendered = (
        template.replace("{{TITLE}}", html.escape(title))
        .replace("{{TOC}}", toc_html)
        .replace("{{CONTENT}}", content_html)
        .replace("{{MARKDOWN_SOURCE}}", json.dumps(md_text))
        .replace("{{SECTION_MD_MAP}}", json.dumps(section_md_map, ensure_ascii=False))
        .replace(
            "{{FRONTMATTER_JSON}}",
            json.dumps(frontmatter_data, ensure_ascii=False),
        )
        .replace(
            "{{UPDATED_AT}}",
            dt.datetime.now(BEIJING_TZ).strftime("%Y-%m-%d %H:%M"),
        )
    )

    rendered = embed_local_images(rendered, input_path.parent)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    _ = output_path.write_text(rendered, encoding="utf-8")
    output_exists = output_path.exists()

    print(f"Markdown: {input_path}")
    print(f"Template: {template_path}")
    print(f"HTML: {output_path}")
    print(f"HTML exists: {'yes' if output_exists else 'no'}")

    if not output_exists:
        raise SystemExit(f"Failed to create output HTML: {output_path}")

    if not args.keep_md:
        input_path.unlink()
        print("md文件已删除, 可在html内复制")

    if args.open:
        import webbrowser

        webbrowser.open(output_path.as_uri())


if __name__ == "__main__":
    main()
