---
name: domain-modeling
description: 构建并打磨项目的 domain model。用户需要确定 domain terminology 或 ubiquitous language、记录架构决策，或其他 skill 需要维护 domain model 时使用。
---

# Domain Modeling

在设计过程中主动构建并打磨项目的 domain model。这是一项主动的 discipline：质疑术语、设计 edge-case scenarios，并在 glossary 和决策明确的瞬间记录下来。（仅仅为了获取词汇而读取 `CONTEXT.md` 不属于这个 skill，那是任何 skill 都应该具备的一行习惯。本 skill 用于改变 model，而不只是消费它。）

## 文件结构

大多数 repo 只有一个 context：

```
/
├── CONTEXT.md
├── docs/
│   └── adr/
│       ├── 0001-event-sourced-orders.md
│       └── 0002-postgres-for-write-model.md
└── src/
```

如果根目录存在 `CONTEXT-MAP.md`，说明 repo 有多个 context。该 map 会指出每个 context 所在位置：

```
/
├── CONTEXT-MAP.md
├── docs/
│   └── adr/                          ← system-wide decisions
├── src/
│   ├── ordering/
│   │   ├── CONTEXT.md
│   │   └── docs/adr/                 ← context-specific decisions
│   └── billing/
│       ├── CONTEXT.md
│       └── docs/adr/
```

延迟创建文件，只在确实有内容可写时创建。如果不存在 `CONTEXT.md`，在第一个术语确定时创建；如果不存在 `docs/adr/`，在首次需要 ADR 时创建。

## Session 期间

### 对照 glossary 质疑术语

当用户使用的术语与 `CONTEXT.md` 中现有语言冲突时，立即指出：“你的 glossary 将 cancellation 定义为 X，但你似乎指的是 Y，到底是哪一个？”

### 打磨模糊语言

当用户使用含糊或过载的术语时，提出精确的 canonical term。“你说的是 account——你指的是 Customer 还是 User？这是两个不同的概念。”

### 讨论具体场景

讨论 domain relationships 时，用具体场景进行 stress-test。设计能够探测 edge cases 的场景，迫使用户精确说明概念之间的边界。

### 与代码交叉核对

当用户描述某件事的工作方式时，检查代码是否一致。如果发现矛盾，直接指出：“你的代码会取消整个 Order，但你刚才说可以 partial cancellation，哪一个才对？”

### 就地更新 CONTEXT.md

术语确定后立即更新 `CONTEXT.md`。不要批量积压，在术语确定时就记录。使用 [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md) 中的格式。

`CONTEXT.md` 必须完全不包含实现细节。不要把它当作 spec、scratch pad 或实现决策仓库。它只是 glossary。

### 谨慎提供 ADR

只有同时满足以下三点时，才建议创建 ADR：

1. **难以逆转**：之后改变决定的成本很高
2. **缺少上下文会令人意外**：未来读者会疑惑“为什么要这样做？”
3. **来自真实 trade-off**：存在真正的替代方案，而你基于具体原因选择了其中一个

缺少任何一项都跳过 ADR。使用 [ADR-FORMAT.md](./ADR-FORMAT.md) 中的格式。
