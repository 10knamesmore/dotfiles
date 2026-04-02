---
name: doc-markdown-html
description: 使用 Markdown 编写结构化文档并生成 HTML 页面。适用于撰写技术文档、产品说明、操作手册、规范、教程等场景。
---

# Doc Markdown HTML

## 工作流

1. 确认文档主题和输出文件名。
2. 完整撰写 Markdown 内容，保存到工作目录(或者复用用户已有的md文件)。
3. 运行渲染脚本生成 HTML（默认会删除源 .md 文件，仅保留 HTML）。
4. 返回 HTML 文件的绝对路径，并简要说明核心章节。
 - 返回路径格式如: `file:///User/foo/bar.html`
 - 若用户要求保留 md 文件，使用 `--keep-md` 参数，同时返回 md 路径

## 渲染命令

```bash
uv run scripts/render_markdown_html.py <input.md> --output <output.html>
```

当你发现在沙箱环境下无法运行时， 你应当向用户要求提权运行uv， 而不是尝试用其他方式运行脚本

参数：

| 参数 | 简写 | 说明 | 默认值 |
|---|---|---|---|
| `<input>` | — | 输入 Markdown 文件路径（必填） | — |
| `--output` | `-o` | 输出 HTML 文件路径 | 同输入路径，扩展名改为 `.html` |
| `--template` | `-t` | HTML 模板文件路径 | `assets/doc-template.html` |
| `--title` | — | 强制指定页面标题 | 取第一个 `# 标题`，无则用文件名 |
| `--toc-min-level` | — | 目录包含的最小标题级别 | `2` |
| `--toc-max-level` | — | 目录包含的最大标题级别 | `3` |
| `--open` | `-O` | 渲染完成后在浏览器中打开 | 否 |
| `--keep-md` | — | 保留源 Markdown 文件（默认渲染后删除） | 否 |


## 规则

- 始终以 Markdown 作为事实来源。
- 除非用户明确要求，否则始终生成 HTML。
- 支持解析 Markdown 文件开头的 YAML frontmatter。
- 若用户已提供现成 Markdown 文件路径，则不允许修改原有的 Markdown 文件。
- 不允许修改 `assets/doc-template.html`

frontmatter 示例(支持解析任意字段)：

```yaml
---
name: Foo
description: 一个用foo的brr
biz:
  - markdown
  - html
nested:
  baz: true
  boo: wanger
---
```
