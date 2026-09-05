# 推进 Spec

用户提供 Spec 或 Subspec path；如果只提供 Spec，由你选择 frontier，不让用户手动挑选。

按依赖顺序逐个推进授权范围内的 Subspec，不以 session 限制交付范围。除用户明确授权的并行 research 外，一次只 claim 一个 Subspec；当前 Subspec 满足 验收条件 后，再推进下一个。用户暂停时停止；出现必须由用户决定的阻塞时，只暂停依赖该决定的工作。

1. 读取完整 Spec，再只读取 Subspec index 中各文件的 frontmatter，计算 frontier。
2. 优先继续当前执行者在授权范围内已 claim 的 `in-progress` Subspec。否则，如果用户指定 Subspec，验证它位于 frontier；未指定时按 Spec index 顺序选择授权范围内第一个 frontier。
3. 对选中的 `ready` Subspec 执行 claim；由当前执行者持有的 `in-progress` Subspec 直接继续，不重复 claim。读取它的完整内容和 dependency resolution；不要一次加载所有 Subspec body，不覆盖其他执行者的 claim。
4. 按 kind 推进：decision 使用 [grill-with-docs](../../grill-with-docs/SKILL.md)；research 查证事实；简化候选调查使用 [`find-simplifications`](../../find-simplifications/SKILL.md)；prototype 产出并链接 artifact；task 完成前置操作；implementation 交给 `implement` skill。
5. 将结论写入该 Subspec 的 `Resolution`，把验证结果写入 `Evidence`，按 [完成与修正规则](../SKILL.md#完成与修正) 更新状态。
6. 更新 Spec 的 decision summary、Subspec index、fog、out-of-scope 和整体 status。只有所有 required Subspec 均为 `resolved` 且整体验收标准已验证时，才可将 Spec 设为 `complete`。新浮现的问题能精确表达时创建新 Subspec，否则留在 fog。
7. 如果没有 frontier，但仍有未 resolved Subspec，报告 dependency cycle、`draft` Subspec 或 stale claim；不要猜测下一步。
8. 当前 Subspec 已 resolved 且授权范围尚未完成时，重新计算 frontier 并继续。已完成结果出现偏差时，在原 Subspec 继续修正，不要求用户单独申请 reopen。

并发 session 应只长期编辑各自 claim 的 Subspec。对 Spec 文件的更新保持短小，并在写入前重新读取，降低 merge conflict。
