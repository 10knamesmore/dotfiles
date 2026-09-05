# 创建 Spec（从模糊目标）

用户带着一个大而模糊的目标调用。先创建 Spec 骨架，再 grill 清楚 destination——grilling 的纪律是确认即写盘，必须始终有 Spec 文件作为落点，不允许 grill 完凭回忆批量补写。

1. 读取 repo instructions 与已有 spec 约定；按目标涉及的业务概念查阅 glossary、相关 ADR 和源码。
2. 如果工作已经清晰且一个 session 可以完成，且用户未要求写 Spec，不创建 Spec；用户已授权实施时直接完成工作，否则说明方案。
3. 创建 Spec 骨架：destination 先写当前理解的 draft，acceptance criteria、fog 和 out-of-scope 能写多少写多少，status 为 `draft`。
4. 使用 [grill-with-docs](../../grill-with-docs/SKILL.md) 打磨 destination。destination 决定整个 Spec 的 scope。用户每确认一项事实，立即更新 Spec 的对应 section。
5. destination 收敛后，breadth-first 扫描整个问题空间，找出当前可明确表达的 decision、research、prototype 和 task。implementation Subspec 只在 contract 与 acceptance criteria 稳定后创建。目标是寻找代码简化时，使用 [`find-simplifications`](../../find-simplifications/SKILL.md) 证明候选；互不相关的候选分别形成 cohesive Spec，不创建全仓库 cleanup umbrella。
6. 为当前可以精确表达的工作创建 Subspec；全部文件创建后，再在 second pass 写 `depends_on`，避免引用不存在的 `id`。
7. 在 Spec 的 Subspec index 中只写 title 与相对链接，不复制 status、dependency 或详细内容。
8. 用户明确要求并行 agent 工作时，可以启动互不依赖的 research subagent；每个 subagent 只修改自己的 Subspec。
9. 用户只要求创建 Spec 时，完成文档后停止。用户已授权实施或推进完整目标时，转入 `advance.md`，按依赖顺序继续授权范围内的工作。
