# 输出 Schema

结构化输出使用此字段规范，默认 JSON。

## 顶层

```json
{
  "page": "<string>",
  "source": [
    "<path>"
  ],
  "interactions": [
    {
      "id": "<snake_case>",
      "type": "<interaction_type>",
      "trigger": {
        "event": "<click|submit|confirm|auto>",
        "element": "<string>",
        "handler": "<string>"
      },
      "params": [
        {
          "name": "<string>",
          "source": "<string>",
          "required": "<true|false|unknown>",
          "notes": "<string>"
        }
      ],
      "result": {
        "ui": "<string>",
        "business": "<string>"
      },
      "confidence": "<high|medium|low>",
      "notes": "<string>"
    }
  ]
}
```

## 字段说明

- `id`: 使用业务语义动词，如 `create_order`、`submit_strategy`。
- `type`: 使用规范化交互类型。
- `trigger.event`: 选主要事件；若多事件，优先用户触发。
- `params`: 只保留业务相关字段，保持精简。
- `result.ui`: 必须是 UI 可观察变化或明确代码效果。
- `result.business`: 允许不完整或未知。
- `confidence`: 每个交互必填。

## 最小示例

```json
{
  "page": "OrderCreate",
  "source": [
    "src/pages/order/Create.tsx"
  ],
  "interactions": [
    {
      "id": "create_order",
      "type": "primary_action",
      "trigger": {
        "event": "submit",
        "element": "Form \"Create Order\"",
        "handler": "handleSubmit"
      },
      "params": [
        {
          "name": "symbol",
          "source": "form.field",
          "required": "true",
          "notes": "from input symbol"
        }
      ],
      "result": {
        "ui": "toast success + list refresh",
        "business": "POST /orders"
      },
      "confidence": "high",
      "notes": "explicit submit handler"
    }
  ]
}
```
