# CONTEXT.md Format

## Structure

```md
# {Context Name}

{One or two sentence description of what this context is and why it exists.}

## Language

**Order**:
{A one or two sentence description of the term}
_Avoid_: Purchase, transaction

**Invoice**:
交付后发送给 customer 的付款请求。
_Avoid_: Bill, payment request

**Customer**:
下单的个人或组织。
_Avoid_: Client, buyer, account
```

## 规则

- **保持明确立场。**同一个概念存在多个词时，选择最合适的一个，并将其他词列在 `_Avoid_` 下。
- **定义保持紧凑。**最多一到两句话。定义它是什么，而不是它做什么。
- **只包含本项目 context 特有的术语。**通用编程概念（timeout、error type、utility pattern）即使被项目广泛使用，也不属于这里。添加术语前先问：这是本 context 独有的概念，还是通用编程概念？只有前者才应加入。
- **自然形成聚类时使用子标题分组。**如果所有术语属于一个连贯领域，使用平铺列表即可。

## Single context 与 multi-context repo

**Single context（大多数 repo）：**repo 根目录有一个 `CONTEXT.md`。

**Multiple contexts：**repo 根目录的 `CONTEXT-MAP.md` 列出各个 contexts、它们的位置以及相互关系：

```md
# Context Map

## Contexts

- [Ordering](./src/ordering/CONTEXT.md) — 接收并跟踪 customer orders
- [Billing](./src/billing/CONTEXT.md) — 生成 invoices 并处理 payments
- [Fulfillment](./src/fulfillment/CONTEXT.md) — 管理仓库拣货和 shipping

## Relationships

- **Ordering → Fulfillment**：Ordering 发出 `OrderPlaced` events；Fulfillment 消费它们并开始拣货
- **Fulfillment → Billing**：Fulfillment 发出 `ShipmentDispatched` events；Billing 消费它们并生成 invoices
- **Ordering ↔ Billing**：共享 `CustomerId` 和 `Money` types
```

skill 会推断适用哪种结构：

- 如果存在 `CONTEXT-MAP.md`，读取它以查找 contexts
- 如果只有根目录 `CONTEXT.md`，则是 single context
- 如果两者都不存在，在第一个术语确定时延迟创建根目录 `CONTEXT.md`

存在多个 contexts 时，推断当前主题属于哪一个。如果不明确，就询问。
