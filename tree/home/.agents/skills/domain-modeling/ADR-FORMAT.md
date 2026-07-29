# ADR Format

ADR 位于 `docs/adr/`，使用连续编号：`0001-slug.md`、`0002-slug.md` 等。

延迟创建 `docs/adr/` 目录，只在首次需要 ADR 时创建。

## 模板

```md
# {Short title of the decision}

{1-3 句话：背景是什么、决定了什么，以及为什么。}
```

仅此而已。ADR 可以只有一个 paragraph。它的价值在于记录确实作出了某项决策，以及 *why*，而不在于填满各个 section。

## 可选章节

只有在确实有价值时才包含。大多数 ADR 不需要这些章节。

- **Status** frontmatter（`proposed | accepted | deprecated | superseded by ADR-NNNN`）：重新审视决策时有用
- **Considered Options**：只有被拒绝的替代方案值得记住时才加入
- **Consequences**：只有需要特别指出不明显的下游影响时才加入

## 编号

扫描 `docs/adr/`，找到当前最大编号后加一。

## 何时建议 ADR

以下三点必须全部成立：

1. **难以逆转**：之后改变决定的成本很高
2. **缺少上下文会令人意外**：未来读者会看着代码疑惑“为什么偏偏要这样做？”
3. **来自真实 trade-off**：存在真正的替代方案，而你基于具体原因选择了其中一个

如果一个决定容易逆转，就跳过它，因为你之后会直接逆转。如果它并不令人意外，就没人会疑惑原因。如果不存在真正的替代方案，除了“我们选择了显而易见的做法”之外，没有什么值得记录。

### 哪些情况符合条件

- **架构形态。**“我们使用 monorepo。”“write model 使用 event sourcing，read model 投影到 Postgres。”
- **context 之间的集成模式。**“Ordering 和 Billing 通过 domain events 通信，而不是同步 HTTP。”
- **会造成 lock-in 的技术选择。**数据库、message bus、auth provider、deployment target。不是每个 library，而是那些替换需要花上一个季度的选择。
- **边界和范围决策。**“Customer data 由 Customer context 所有；其他 context 只能通过 ID 引用。”明确的 no 和 yes 一样有价值。
- **有意偏离显而易见的路径。**“因为 X，我们使用 manual SQL，而不是 ORM。”任何合理读者会默认相反做法的情况都属于此类。这能阻止下一个工程师“修正”一个本来有意为之的决定。
- **代码中不可见的约束。**“由于合规要求，我们不能使用 AWS。”“由于 partner API contract，响应时间必须低于 200ms。”
- **拒绝替代方案的原因并不明显。**如果你考虑过 GraphQL，却因微妙原因选择了 REST，就记录下来；否则六个月后又会有人建议 GraphQL。
