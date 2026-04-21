---
name: doc-markdown-html
description: 撰写、生成或更新任何文档时必须触发——包括"写文档/写说明/文档化/写一份指南/教程/README/API 文档/项目说明/技术文档/使用手册"等任意表述。
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
**脚本会默认删除原markdown文件** 这是符合预期的, 一般若用户无强调, 删除即可

参数：

| 参数              | 简写 | 说明                                   | 默认值                          |
| ----------------- | ---- | -------------------------------------- | ------------------------------- |
| `<input>`         | —    | 输入 Markdown 文件路径（必填）         | —                               |
| `--output`        | `-o` | 输出 HTML 文件路径                     | 同输入路径，扩展名改为 `.html`  |
| `--template`      | `-t` | HTML 模板文件路径                      | `assets/doc-template.html`      |
| `--title`         | —    | 强制指定页面标题                       | 取第一个 `# 标题`，无则用文件名 |
| `--toc-min-level` | —    | 目录包含的最小标题级别                 | `2`                             |
| `--toc-max-level` | —    | 目录包含的最大标题级别                 | `3`                             |
| `--open`          | `-O` | 渲染完成后在浏览器中打开               | 否                              |
| `--keep-md`       | —    | 保留源 Markdown 文件（默认渲染后删除） | 否                              |

## Markdown 支持范围

渲染脚本支持以下内容：

| 类型             | 说明                                                         |
| ---------------- | ------------------------------------------------------------ |
| 标准 Markdown    | 标题、段落、粗斜体、行内代码、链接、图片                     |
| 表格             | GFM 风格，支持列对齐                                         |
| 有序/无序列表    | 含嵌套                                                       |
| 代码块           | 围栏代码块，支持语言标注、`title="…"` 和 `{行号}` 高亮元信息 |
| Mermaid 图表     | ```mermaid``` 块自动渲染为 SVG，跟随亮/暗主题切换           |
| 本地图片         | `![](相对路径)` 自动 base64 内嵌，生成的 HTML 完全自包含     |
| 块引用           | 单层与嵌套                                                   |
| 定义列表         | `term\n: definition` 格式                                    |
| 脚注             | `[^key]` / `[^key]: …` 格式                                  |
| YAML frontmatter | 文件开头 `---` 块，任意字段，渲染为文档信息面板              |

## 规则

- 始终以 Markdown 作为事实来源。
- 除非用户明确要求，否则始终生成 HTML。
- 支持解析 Markdown 文件开头的 YAML frontmatter。
- 若用户已提供现成 Markdown 文件路径，则不允许修改原有的 Markdown 文件。
- 不允许修改 `assets/doc-template.html`
- frontmatter 中的日期值必须加引号（如 `date: "2026-04-01"`），否则 YAML 会将其解析为 `datetime.date` 对象导致 JSON 序列化失败。

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
