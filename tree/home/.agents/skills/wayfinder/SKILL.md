---
name: wayfinder
description: 将无法由一个 agent session 容纳的大块工作组织为 repo-local Spec 与依赖明确的 Subspec，并逐个推进，直到通往 destination 的路径清晰或工作完成。
---

# Wayfinder

一个模糊想法可能大到无法由一个 agent session 完成，而且从当前位置到 **destination** 的路径还不可见。Wayfinder 先把这段工作组织成一个 **Spec**，再把当前已经看清的问题和执行切片写成 **Subspec**，逐个推进并持续清除 fog。

默认使用 repo 内的 Markdown 文件作为唯一 source of truth。不要创建或维护 issue、label、assignee、blocking relation、resolution comment，也不要通过 close issue 表示完成。只有用户明确要求同步到 issue tracker 时，才把 tracker 当作 mirror；文件中的 Spec 与 Subspec 仍是 canonical artifact。

## Domain model

- **Spec**：整个 effort 的 contract。它定义 destination、scope、共享决策、acceptance criteria，并索引 Subspec。
- **Subspec**：一个 agent session 内能够独立澄清、验证或实现的工作单元。详细问题、resolution 和 evidence 只保存在对应 Subspec 中。
- **Dependency**：Subspec frontmatter 中的 `depends_on`。只引用同一 Spec 下的 Subspec `id`。
- **Frontier**：`status: ready`、所有 dependency 都已 `resolved`、且尚未被其他 session claim 的 Subspec。
- **Fog**：确认属于 destination，但当前还无法精确表达成 Subspec 的范围。

不要把 Spec 当作 Subspec 内容的副本。Spec 只保留共享 contract、一句话 decision 摘要和链接；详细内容只存在于一个 Subspec。

## Storage

先遵守 repo 已有的 spec 目录约定。没有约定时使用：

```text
specs/
└── <spec-slug>/
    ├── SPEC.md
    └── subspecs/
        ├── <id>-<slug>.md
        └── ...
```

创建或修改文件前，读取：

- [Spec format](references/SPEC-FORMAT.md)
- [Subspec format](references/SUBSPEC-FORMAT.md)

路径和 `id` 用于机器定位；面向人类的 narration、decision index 和交接说明始终使用 title，并把链接包在 title 上。

## Status and claim

Spec status：

- `draft`：仍有 planning Subspec 或 fog，执行 contract 尚未稳定。
- `ready`：共享决策已稳定，至少一个 implementation Subspec 位于 frontier。
- `in-progress`：已有 implementation Subspec 正在推进。
- `complete`：required Subspec 均已 resolved，Spec acceptance criteria 已验证。
- `paused`：用户明确暂停整个 effort。

Subspec status：

- `draft`：目标或 acceptance criteria 还不够清晰，不能进入 frontier。
- `ready`：内容清晰，可以在 dependency resolved 后推进。
- `in-progress`：已被一个 session claim。
- `resolved`：resolution 与 evidence 已写入文件。
- `cancelled`：已确认不再属于当前路线，并记录原因。

不要持久化 `blocked` status。是否 blocked 完全由 `depends_on` 指向的 Subspec status 推导，避免出现两份互相漂移的状态。

开始实质工作前，先把选中的 Subspec 从 `ready` 改为 `in-progress`，并写入 `owner`。这次 frontmatter 修改就是 claim。发现 Subspec 已是 `in-progress` 且 owner 不同，就跳过它。完成前不要预先改为 `resolved`。

## Subspec kinds

- **decision**（HITL）：需要用户参与的产品、domain 或架构决策。使用 grilling 与 domain-modeling，一次只解决一个问题。
- **research**（AFK）：通过源码、documentation、API 或其他可验证资料补齐决策所需事实。
- **prototype**（HITL）：制作低成本 artifact，让用户基于具体结果反馈。
- **task**（HITL 或 AFK）：解除后续决策阻塞所需的前置操作，本身不交付 destination。
- **implementation**（AFK，必要时 HITL）：实现已经稳定的 contract。交给 `implement` skill 推进。

Subspec 的 kind 描述工作性质，不表示 status。不要按 user story 机械拆分 Subspec；按可以独立验收、依赖明确的决策或实现切片拆分。

## Fog of war

Spec 故意允许不完整。判断内容应该成为 Subspec 还是留在 `Not yet specified`，只看问题现在能否被精确表达，而不是现在能否回答。

- 问题已经清晰：创建 Subspec，即使它仍被 dependency 阻塞。
- 问题还说不清：留在 `Not yet specified`，等 frontier 前进后再 graduate。

`Not yet specified` 不包含已作出的 decision、已有 Subspec 或 out-of-scope 内容。

destination 决定 scope。确认不属于 destination 的内容写入 Spec 的 `Out of scope`；如果已经存在对应 Subspec，将其改为 `cancelled` 并记录原因。只有重新定义 destination 时，才重新考虑这些内容。

## Invocation

有两种模式。除并行 research 外，每个 session 最多推进一个 Subspec。

### 创建 Spec

用户带着一个大而模糊的目标调用。

1. 读取 repo instructions、现有 glossary、相关 ADR，以及已有 spec 约定。
2. 使用 grilling 和 domain-modeling 命名 destination。destination 决定整个 Spec 的 scope。
3. breadth-first 扫描整个问题空间，找出当前可明确表达的 decision、research、prototype 和 task。如果工作已经清晰且一个 session 可以完成，不创建 Spec，直接说明无需拆分。implementation Subspec 只在 contract 与 acceptance criteria 稳定后创建。
4. 创建 `SPEC.md`，写入 destination、共享约束、acceptance criteria、fog 和 out-of-scope。
5. 为当前可以精确表达的工作创建 Subspec；全部文件创建后，再在 second pass 写 `depends_on`，避免引用不存在的 `id`。
6. 在 Spec 的 Subspec index 中只写 title 与相对链接，不复制 status、dependency 或详细内容。
7. 可以并行启动互不依赖的 research subagents；每个 subagent 只修改自己的 Subspec。
8. 停止。创建 Spec 的 session 不继续解决 decision 或 implementation Subspec。

### 推进 Spec

用户提供 Spec 或 Subspec path；如果只提供 Spec，由你选择 frontier，不让用户手动挑选。

1. 读取完整 Spec，再只读取 Subspec index 中各文件的 frontmatter，计算 frontier。
2. 如果用户指定 Subspec，验证它位于 frontier；否则按 Spec index 顺序选择第一个 frontier。
3. claim 选中的 Subspec，再读取它的完整内容和 dependency resolution。不要一次加载所有 Subspec body。
4. 按 kind 推进：decision 使用 grilling 与 domain-modeling；research 查证事实；prototype 产出并链接 artifact；task 完成前置操作；implementation 交给 `implement`。
5. 将结论写入该 Subspec 的 `Resolution`，把验证结果写入 `Evidence`，实际验证后再改为 `resolved`。
6. 更新 Spec 的 decision summary、Subspec index、fog、out-of-scope 和整体 status。新浮现的问题能精确表达时创建新 Subspec，否则留在 fog。
7. 如果没有 frontier，但仍有未 resolved Subspec，报告 dependency cycle、`draft` Subspec 或 stale claim；不要猜测下一步。

并发 session 应只长期编辑各自 claim 的 Subspec。对 `SPEC.md` 的更新保持短小，并在写入前重新读取，降低 merge conflict。
