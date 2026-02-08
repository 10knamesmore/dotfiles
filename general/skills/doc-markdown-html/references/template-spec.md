# 模板规范

## 占位符

固定模板 `assets/doc-template.html` 支持以下占位符：

- `{{TITLE}}`：页面 `<title>` 与顶部 `<h1>`。
- `{{TOC}}`：自动生成的目录 HTML。
- `{{CONTENT}}`：Markdown 渲染后的正文内容。
- `{{UPDATED_AT}}`：渲染器生成的 UTC 时间戳。

## 目录规则

- 仅包含 `--toc-min-level` 到 `--toc-max-level` 范围内的标题。
- 为标题生成稳定锚点 ID。
- 重复标题会自动加后缀（`-2`、`-3`...）。

## 渲染行为

- 使用 Python `markdown` + `pygments` 进行渲染与代码高亮。
- fenced code block 只要标注 ` ```language ` 就按对应语言高亮。
- 若缺少依赖，脚本会直接报错并提示安装命令。

## 代码复制行为

- 模板会为每个代码块（`.codehilite`）注入“复制”按钮。
- 点击按钮时复制对应 `pre` 文本内容。
- 优先使用 `navigator.clipboard.writeText`，不可用时回退到 `textarea + document.execCommand('copy')`。
- 复制成功时按钮短暂显示“已复制”，失败时显示“复制失败”。

## 推荐用法

```bash
python3 scripts/render_markdown_html.py docs/my-guide.md --output docs/my-guide.html
```
