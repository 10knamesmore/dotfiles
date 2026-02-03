---
name: test-intent-parser
description: 从 ones 测试管理系统导出的 CSV 用例生成结构化 YAML 测试轨迹 IR。必须通过清洗脚本解析 CSV 并结合页面语义索引输出逐步执行轨迹，不包含 selector 或代码。
---

0. **IMPORTANT – Path Resolution**

该 skill 可能被安装在不同路径。

调用脚本时必须使用：

```
$SKILL_DIR
```

表示 skill 根目录（即本 SKILL.md 所在目录）。

不得写死绝对路径。

---

# 测试意图解析器（ones CSV 专用）

## 输入约束（强制）

本 skill **只允许**输入：

👉 ones 导出的 CSV 测试用例文件

用户必须提供：

```
CSV_PATH
```

如果用户没有提供 CSV_PATH：

不得继续执行清洗流程。

必须直接输出失败 YAML：

```yaml
meta:
  description: missing_csv_path
  confidence: low
trace: []
```


---

## CSV 清洗（必须执行）

你不得直接读取 CSV。

必须执行：

```bash
python3 $SKILL_DIR/scripts/clean_case.py CSV_PATH
```

只允许读取：

👉 标准输出 JSON

禁止绕过脚本。

---

## 清洗后 JSON 结构

```json
[
  {
    "title": "...",
    "page": "...",
    "preconditions": [],
    "steps": ["...", "..."],
    "expected": ["..."]
  }
]
```

说明：

- 每个对象 = 一个测试用例
- title = 用例名称
- page = 页面语义路径
- steps = 动作描述
- expected = 断言描述

该 JSON 是自然语言语义层，不是最终 IR。

---

## 页面语义对齐

根据：

```
title
page
```

在项目目录：

```
page_behavior_index/
```

查找对应 YAML 文件。


文件的名称可能和 pages 不完全匹配, 

此时你需要根据语义和对应文件名筛选出部分可能匹配的文件, 并逐个阅读, 找到与测试意图相符合的页面行为索引文件。

该文件定义：

- 页面动作语义
- 断言语义
- 组件映射

你必须：

👉 使用该语义进行映射  
👉 禁止编造 UI 动作

---

## 工作流程

1. 要求用户提供 CSV_PATH
2. 执行清洗脚本
3. 读取 cleaned JSON
4. 查找页面语义索引
5. 映射步骤 → trace
6. 映射断言
7. 应用置信度规则
8. 输出 YAML

---

## 输出 Schema（严格遵守）

```yaml
meta:
  description: <测试摘要>
  confidence: <high|medium|low>

preconditions:
  - <语义条件>

trace:
  - step: <snake_case>
    action:
      type: <navigate|click|fill|submit|select|wait|custom>
      page: <semantic_page|unknown>
      target: <semantic_target>
      input:
        key: value
    expect:
      - location: <semantic_component>
        op: <visible|eq|contains|non_empty>
        value: <string|optional>
    notes: <optional>
```

! 注意: 你在json中解析到的每一个样例, 都需要输出一个单独的yml, 不允许合并为同一个文件.

注意, 所有样例的precondition都有一个默认情况: 就是登陆 + 

---

## 核心规则

- 不输出 selector
- 不输出代码
- 不猜 UI
- 只使用 page_behavior 语义
- 缺失值必须 `<TBD>`
- 不询问用户额外业务语义
- 必须输出 YAML
- 每个步骤必须可执行

---

## Typed TBD 规则

当语义缺失时必须使用：

<TBD:type:hint>

type 允许值：

param   → 参数缺失
ui      → UI 组件缺失
assert  → 断言缺失
page    → 页面缺失
data    → 外部数据缺失

不得使用裸 <TBD>

hint是自然语言的简短提示, 指导如何 fallback

## 可执行性规则

一个 step 被视为可执行，当且仅当：

- action.type 可映射到 page_behavior interaction
- target 在页面索引中存在
- expect 使用合法 op

若不满足：

必须：

- 保留 step
- 使用 <TBD>
- 降低 confidence

---

## 置信度规则

high  
完全匹配 page_behavior

medium  
部分推断

low  
主要语义猜测

---

## 失败输出

```yaml
meta:
  description: parse_failed
  confidence: low
trace: []
```

---

## 非目标

- 不生成 Playwright 代码
- 不生成 selector
- 不替代真实执行
- 不修复 CSV
- 不修改原始文件

---

## 输出性质

该 IR：

- 可丢弃
- 可重复生成
- 非事实源
- 仅作为自动化中间层

---

