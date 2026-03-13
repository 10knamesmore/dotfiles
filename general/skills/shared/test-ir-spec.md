# Test IR 共享规范

该规范适用于以下技能的输入/输出约束：

- `test-intent-parser`（产出 IR）
- `generate-test-spec`（消费 IR）

## Schema（严格）

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

## 字段语义

- `id`: 用例唯一标识。
- `title`: 用例标题。
- `precondition`: 前置条件列表, (⚠️除了登陆意外的页面, 应该至少包含一个`已经进入xx页面`的前置条件, 如果用户没有提供, 需要补齐)
- `flow`: 动作序列。
- `action`: 动作类型，必须为 `navigate|click|fill|select|wait`。
- `page`: 语义页面名，未知时使用 `unknown`。
- `target`: 语义目标（不允许 selector）。
- `value`: 输入或选择值，仅 `fill/select` 使用。
- `assertions`: 断言列表。
- `confidence`: 0~1 置信度。

## Typed TBD 规则（严格）

语义缺失时必须使用：`<TBD:type:hint>`

- `type` 允许值：`param|ui|assert|page|data`
- 禁止使用裸 `<TBD>`
- `hint` 必须是简短自然语言，指导 fallback

## 可执行性规则

一个 `flow` step 可执行当且仅当：

- `action` 合法
- `target` 为可执行语义描述
- `assertions` 字段齐全（允许内容为 `<TBD:...>`）

若不满足：

- 必须保留该 step
- 用 `<TBD:type:hint>` 补齐缺失
- 下调 `confidence`

## 置信度分层

- high: `0.8 ~ 1.0`
- medium: `0.5 ~ 0.79`
- low: `0.0 ~ 0.49`

## 失败输出

无法生成有效 IR 时输出：

```json
[]
```

## 约束

- 不输出 selector
- 不输出代码
- 不猜 UI

