#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["markdown", "pygments"]
# ///
"""将 Markdown 渲染为基于固定模板的带语法高亮 HTML 页面。

提供从 Markdown 源文件到完整 HTML 的一站式转换，包含目录生成、
标题锚点注入、代码块语言标注与 Pygments 语法高亮。
"""

from __future__ import annotations

import argparse
import datetime as dt
import html
import json
import re
from pathlib import Path
from typing import cast, final


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


def extract_headings(md_text: str) -> list[tuple[int, str, str]]:
    """从 Markdown 文本中提取所有标题及其锚点。

    重复标题自动追加 ``-2``、``-3`` 等后缀以保证锚点唯一。
    会跳过围栏代码块中的内容，避免把代码注释误识别为标题。

    Args:
        md_text: Markdown 源文本。

    Returns:
        按出现顺序排列的列表，每项为 ``(级别, 标题文本, 锚点 slug)`` 三元组。
    """
    headings: list[tuple[int, str, str]] = []
    used: dict[str, int] = {}
    in_fence = False
    for line in md_text.splitlines():
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
        headings.append((level, title, anchor))
    return headings


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


def markdown_to_html(md_text: str) -> tuple[str, str]:
    """将 Markdown 文本渲染为 HTML 正文与语法高亮 CSS。

    启用 fenced_code、tables、sane_lists、codehilite、attr_list、
    def_list、footnotes 扩展；输出亮色（default）与暗色（github-dark）
    两套高亮 CSS，暗色部分以 ``[data-theme="dark"]`` 作用域限定。

    Args:
        md_text: Markdown 源文本。

    Returns:
        ``(content_html, highlight_css)`` 二元组：
        前者为渲染后的正文 HTML，后者为 Pygments 注入所需的 CSS 字符串。

    Raises:
        SystemExit: 当 ``markdown`` 或 ``pygments`` 依赖未安装时。
    """
    try:
        import markdown as md
    except Exception as exc:
        raise SystemExit(
            "Missing dependency: markdown. Install with: pip install markdown pygments"
        ) from exc

    try:
        from pygments.formatters import HtmlFormatter
        from pygments.style import Style
        from pygments.token import (
            Comment,
            Error,
            Generic,
            Keyword,
            Name,
            Number,
            Operator,
            Punctuation,
            String,
            Token,
        )
    except Exception as exc:
        raise SystemExit(
            "Missing dependency: pygments. Install with: pip install markdown pygments"
        ) from exc

    @final
    class TokyoNightDayStyle(Style):  # type: ignore[misc]
        """Pygments 样式：Tokyo Night Day（亮色主题）。"""

        background_color = "#e1e2e7"
        default_style = ""
        styles = {
            Token: "#3760bf",
            Comment: "italic #848cb5",
            Comment.Special: "italic bold #848cb5",
            Keyword: "italic bold #9854f1",
            Keyword.Constant: "italic #9854f1",
            Keyword.Type: "italic #0f4b6e",
            Name.Builtin: "#007197",
            Name.Builtin.Pseudo: "italic #9854f1",
            Name.Class: "#0f4b6e",
            Name.Decorator: "#2496be",
            Name.Exception: "#c64343",
            Name.Function: "#2496be",
            Name.Function.Magic: "#2496be",
            Name.Tag: "italic #9854f1",
            Name.Variable: "#3760bf",
            Number: "#b15c00",
            Operator: "#3760bf",
            Operator.Word: "italic bold #9854f1",
            Punctuation: "#3760bf",
            String: "italic #587539",
            String.Doc: "italic #848cb5",
            String.Escape: "#b15c00",
            String.Interpol: "italic #2496be",
            Generic.Deleted: "#c64343",
            Generic.Emph: "italic",
            Generic.Heading: "bold #3760bf",
            Generic.Inserted: "#587539",
            Generic.Output: "#848cb5",
            Generic.Strong: "bold",
            Generic.Subheading: "bold #2496be",
            Error: "#c64343",
        }

    @final
    class TokyoNightDarkStyle(Style):  # type: ignore[misc]
        """Pygments 样式：Tokyo Night Dark（暗色主题）。"""

        background_color = "#1a1b26"
        default_style = ""
        styles = {
            Token: "#c0caf5",
            Comment: "italic #565f89",
            Comment.Special: "italic bold #565f89",
            Keyword: "italic bold #bb9af7",
            Keyword.Constant: "italic #bb9af7",
            Keyword.Type: "italic #2ac3de",
            Name.Builtin: "#7dcfff",
            Name.Builtin.Pseudo: "italic #bb9af7",
            Name.Class: "#2ac3de",
            Name.Decorator: "#7aa2f7",
            Name.Exception: "#f7768e",
            Name.Function: "#7aa2f7",
            Name.Function.Magic: "#7aa2f7",
            Name.Tag: "italic #bb9af7",
            Name.Variable: "#c0caf5",
            Number: "#ff9e64",
            Operator: "#89ddff",
            Operator.Word: "italic bold #bb9af7",
            Punctuation: "#c0caf5",
            String: "italic #9ece6a",
            String.Doc: "italic #565f89",
            String.Escape: "#ff9e64",
            String.Interpol: "italic #7aa2f7",
            Generic.Deleted: "#f7768e",
            Generic.Emph: "italic",
            Generic.Heading: "bold #c0caf5",
            Generic.Inserted: "#9ece6a",
            Generic.Output: "#565f89",
            Generic.Strong: "bold",
            Generic.Subheading: "bold #7aa2f7",
            Error: "#f7768e",
        }

    content_html = cast(
        str,
        md.markdown(
            md_text,
            extensions=[
                "fenced_code",
                "tables",
                "sane_lists",
                "codehilite",
                "attr_list",
                "def_list",
                "footnotes",
            ],
            extension_configs={
                "codehilite": {
                    "guess_lang": False,
                    "linenums": False,
                    "css_class": "codehilite",
                    "use_pygments": True,
                }
            },
        ),
    )

    light_css = cast(
        str,
        HtmlFormatter(style=TokyoNightDayStyle, cssclass="codehilite").get_style_defs(
            ".codehilite"
        ),
    )  # pyright: ignore[reportUnknownMemberType]
    dark_css = cast(
        str,
        HtmlFormatter(style=TokyoNightDarkStyle, cssclass="codehilite").get_style_defs(
            ".codehilite"
        ),
    )  # pyright: ignore[reportUnknownMemberType]

    # 暗色 CSS 以 [data-theme="dark"] 限定作用域，与主题切换联动
    scoped_dark = "\n".join(
        f'[data-theme="dark"] {line.strip()}'
        for line in dark_css.splitlines()
        if line.strip()
    )
    highlight_css = light_css + "\n" + scoped_dark

    return content_html, highlight_css


def extract_code_langs(md_text: str) -> list[str]:
    """按顺序提取 Markdown 中所有围栏代码块的语言标识。

    Args:
        md_text: Markdown 源文本。

    Returns:
        语言标识列表，与文档中代码块出现顺序一致；未指定语言时对应项为空字符串。
    """
    langs: list[str] = []
    in_block = False
    for m in re.finditer(r"^```(\w*)", md_text, re.MULTILINE):
        if not in_block:
            langs.append(m.group(1) or "")
            in_block = True
        else:
            in_block = False
    return langs


def inject_code_langs(html_text: str, langs: list[str]) -> str:
    """按顺序将语言标识作为 ``data-lang`` 属性注入 ``.codehilite`` 容器。

    与 ``extract_code_langs`` 配合使用，保证注入顺序与源码一致。
    无语言或列表已耗尽时跳过对应块，不修改原始标签。

    Args:
        html_text: 由 ``markdown_to_html`` 生成的 HTML 字符串。
        langs: ``extract_code_langs`` 返回的语言标识列表。

    Returns:
        注入 ``data-lang`` 后的 HTML 字符串。
    """
    idx = 0

    def repl(match: re.Match[str]) -> str:
        nonlocal idx
        lang = langs[idx] if idx < len(langs) else ""
        idx += 1
        if lang:
            return f'<div class="codehilite" data-lang="{html.escape(lang)}">'
        return match.group(0)

    return re.sub(r'<div class="codehilite">', repl, html_text)


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
    headings = extract_headings(md_text)

    title = args.title
    if not title:
        for level, text, _ in headings:
            if level == 1:
                title = text
                break
    if not title:
        title = input_path.stem

    code_langs = extract_code_langs(md_text)
    content_html, highlight_css = markdown_to_html(md_text)
    content_html = apply_heading_ids(content_html, headings)
    content_html = inject_code_langs(content_html, code_langs)
    toc_html = render_toc(headings, args.toc_min_level, args.toc_max_level)

    template = template_path.read_text(encoding="utf-8")
    rendered = (
        template.replace("{{TITLE}}", html.escape(title))
        .replace("{{TOC}}", toc_html)
        .replace("{{CONTENT}}", content_html)
        .replace("{{HIGHLIGHT_CSS}}", highlight_css)
        .replace("{{MARKDOWN_SOURCE}}", json.dumps(md_text))
        .replace(
            "{{UPDATED_AT}}",
            dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC"),
        )
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    _ = output_path.write_text(rendered, encoding="utf-8")

    print(f"Markdown: {input_path}")
    print(f"Template: {template_path}")
    print(f"HTML: {output_path}")

    if args.open:
        import webbrowser

        webbrowser.open(output_path.as_uri())


if __name__ == "__main__":
    main()
