---
name: python-style
description: Python 交付规范——basedpyright 0 error 0 warning、ruff 格式与 lint、常见 reportXxx 诊断的修法
whenToUse: 编写、修改或审查任何 .py 文件时；交付前必须用 uvx basedpyright / ruff 验证
---

# Python

- 新写或改过的 `.py` 交付前必须跑 `uvx basedpyright <file>` 到 **0 error 0 warning**；跑不干净就改代码，禁止 `# type: ignore` / 放宽规则蒙混
- 格式与基础 lint 走 `uvx ruff format` + `uvx ruff check`（basedpyright 只管类型）
- basedpyright 默认 recommended 档，比 mypy 严得多，常踩的几条与修法：
  - `reportMissingTypeArgument` / `reportUnknown*`：容器写全类型参数；JSON 这类异构结构用 `dict[str, object]`，**别用 `Any`**（`Any` 另触发 `reportAny`）
  - `reportAny`：stdlib / 三方返回 `Any` 的地方就地 `cast(...)` 收窄，例如 `urllib.request.urlopen` → `http.client.HTTPResponse`
  - `reportUnusedCallResult`：有返回值却不用的调用赋给 `_`，如 `_ = parser.add_argument(...)`
  - argparse 的 `args.x` 是 `Any`：定义 `class Args(argparse.Namespace)` 声明字段类型**并给默认值**，`parse_args(namespace=Args())`；默认值只写在类体（argparse 见 namespace 已有该属性就不套自己的 default），`add_argument` 那边别再写 `default=` 重复一遍
  - `reportImplicitStringConcatenation`：相邻字符串字面量隐式拼接不允许，用显式 `+`
  - `reportUninitializedInstanceVariable`：类体里光写注解不给值会报，要么给默认值要么在 `__init__` 里赋
