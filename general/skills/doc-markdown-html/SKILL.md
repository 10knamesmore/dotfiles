---
name: doc-markdown-html
description: 使用 Markdown 编写结构化文档，并基于固定 HTML 模板生成最终页面。适用于用户需要撰写技术文档、产品说明、操作手册、规范、教程，并要求同时产出 Markdown 源文件和可发布 HTML 文件的场景。
---

# Doc Markdown HTML

## 概述

先写 Markdown，再生成最终 HTML。
使用 `scripts/render_markdown_html.py` 保持输出稳定和可复用。

## 工作流

1. 确认文档主题和输出文件名。
2. 完整撰写 Markdown 内容。
3. 将 Markdown 文件保存到工作目录。
4. 运行渲染脚本，使用固定模板生成 HTML。
5. 确认 HTML 文件存在，并返回 Markdown 与 HTML 路径。

## 强制规则

- 始终以 Markdown 作为事实来源。
- 除非用户明确要求，否则始终生成最终 HTML。
- 默认始终使用 `assets/doc-template.html`。
- 模板结构保持稳定；只有用户明确要求改版时才调整模板。
- 生成的 HTML 必须支持代码高亮（fenced code block）。

## 渲染脚本

使用方式：

```bash
python3 scripts/render_markdown_html.py <input.md> --output <output.html>
```

依赖安装：

```bash
pip install markdown pygments
```

可选参数：

- `--template <path>`：指定其他模板路径。
- `--title "..."`：强制页面标题（默认使用第一个 `# Heading`）。
- `--toc-min-level <n>`：目录最小标题级别（默认 `2`）。
- `--toc-max-level <n>`：目录最大标题级别（默认 `3`）。

## 模板变量

渲染器会填充 `assets/doc-template.html` 中的占位符：

- `{{TITLE}}`：HTML 标题与页面主标题。
- `{{TOC}}`：自动生成的目录链接。
- `{{CONTENT}}`：Markdown 渲染后的正文 HTML。
- `{{UPDATED_AT}}`：UTC 渲染时间。

详细规则见 `references/template-spec.md`。

## 输出约定

完成文档任务时必须：

1. 返回 Markdown 文件路径。
2. 返回 HTML 文件路径。
3. 简要说明文档覆盖的核心章节。
