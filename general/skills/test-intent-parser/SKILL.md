---
name: test-intent-parser
description: 从前端已解析的测试用例 JSON 生成结构化 JSON 测试轨迹 IR，不包含 selector 或代码。
---

# 测试意图解析器（前端解析结果输入）

## 输入约束（强制）

本 skill **只允许**输入：

👉 前端解析完成的测试用例 JSON

如果用户没有提供解析后的 JSON：

必须直接输出失败 JSON：

```json
[]
```


---

## 输入 JSON 结构

```json
[
  {
    "title": "<string>",
    "page": "<string>",
    "preconditions": ["<string>"],
    "steps": [
      {
        "step": "<string>",
        "expected"?: ["<string>"]
      }
    ]
  }
]
```

说明：

- 每个对象 = 一个测试用例
- title = 用例名称
- page = 页面语义路径
- preconditions = 前置条件列表（可为空）
- steps = 步骤列表（每个步骤可携带 expected 列表）

该 JSON 是自然语言语义层，不是最终 IR。

---

## 页面语义来源

只使用清洗后 JSON 中的 `page` 字段作为页面语义。

若缺失或为空，必须填 `unknown`。

---

## 工作流程

1. 要求用户提供前端解析后的 JSON
2. 生成 Test IR（flow + assertions）
3. 应用置信度规则
4. 输出 JSON

---

## 输出 Schema（严格遵守）

```json
[
  {
    "id": "<string>",
    "title": "<string>",
    "precondition": ["<string>"],
    "flow": [
      {
        "action": "<navigate|click|fill|select|wait>",
        "page": "<semantic_page|unknown>",
        "target": "<semantic_target>",
        "value": "<string|optional>",
        "assertions": [
          {
            "location": "<semantic_component>",
            "field": "<string>",
            "expect": "<string>"
          }
        ]
      }
    ],
    "confidence": 0.0
  }
]
```

注意：
- 输出为 **JSON 数组**，每个用例一个对象。
- `precondition` 默认包含 `login`（若原始数据未提供前置条件）。

---

## 核心规则

- 不输出 selector
- 不输出代码
- 不猜 UI
- 缺失值必须 `<TBD>`
- 不询问用户额外业务语义
- 必须输出 JSON
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

- action.type ∈ {navigate, click, fill, select, wait}
- target 可用语义描述表示
- assertions 字段齐全

若不满足：

必须：

- 保留 step
- 使用 <TBD>
- 降低 confidence

---

## 置信度规则

使用 0~1 的数值：

- high：0.8 ~ 1.0  
- medium：0.5 ~ 0.79  
- low：0.0 ~ 0.49

---

## 失败输出

```json
[]
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
