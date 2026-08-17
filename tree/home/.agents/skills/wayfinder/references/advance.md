# 推进 Spec

用户提供 Spec 或 Subspec path；如果只提供 Spec，由你选择 frontier，不让用户手动挑选。

除并行 research 外，每个 session 最多推进一个 Subspec。

1. 读取完整 Spec，再只读取 Subspec index 中各文件的 frontmatter，计算 frontier。
2. 如果用户指定 Subspec，验证它位于 frontier；否则按 Spec index 顺序选择第一个 frontier。
3. claim 选中的 Subspec，再读取它的完整内容和 dependency resolution。不要一次加载所有 Subspec body。
4. 按 kind 推进：decision 使用 grill-with-docs 与 domain-modeling；research 查证事实；简化候选调查使用 [`find-simplifications`](../../find-simplifications/SKILL.md)；prototype 产出并链接 artifact；task 完成前置操作；implementation 交给 `implement` skill。
5. 将结论写入该 Subspec 的 `Resolution`，把验证结果写入 `Evidence`，实际验证后再改为 `resolved`。
6. 更新 Spec 的 decision summary、Subspec index、fog、out-of-scope 和整体 status。新浮现的问题能精确表达时创建新 Subspec，否则留在 fog。
7. 如果没有 frontier，但仍有未 resolved Subspec，报告 dependency cycle、`draft` Subspec 或 stale claim；不要猜测下一步。

并发 session 应只长期编辑各自 claim 的 Subspec。对 Spec 文件的更新保持短小，并在写入前重新读取，降低 merge conflict。
