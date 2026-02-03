# 输出 Schema

结构化输出使用此字段规范，默认 YAML。

## 顶层

```yaml
page: <string>                # 页面名或路由
source:
  - <path>                    # 分析的主要文件
interactions:
  - id: <snake_case>          # 业务语义名称
    type: <interaction_type>  # 见 interaction-types.md
    trigger:
      event: <click|submit|confirm|auto>
      element: <string>       # UI 元素，例如 Button "Create"
      handler: <string>       # 可选，handler/function 名称
    params:
      - name: <string>
        source: <string>      # form.field | url.param | selection | state | context
        required: <true|false|unknown>
        notes: <string>
    result:
      ui: <string>            # 可观察 UI 变化
      business: <string>      # 明确可见的业务结果
    confidence: <high|medium|low>
    notes: <string>
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

```yaml
page: OrderCreate
source:
  - src/pages/order/Create.tsx
interactions:
  - id: create_order
    type: primary_action
    trigger:
      event: submit
      element: Form "Create Order"
      handler: handleSubmit
    params:
      - name: symbol
        source: form.field
        required: true
        notes: "from input symbol"
    result:
      ui: "toast success + list refresh"
      business: "POST /orders"
    confidence: high
    notes: "explicit submit handler"
```
