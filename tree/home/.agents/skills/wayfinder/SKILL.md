---
name: wayfinder
description: 管理 repo-local Spec 的完整生命周期——将大而模糊的目标或已聊清楚的当前对话固化为 Spec 与 Subspec，并沿 frontier 逐个推进，直到 destination 达成。推进 feature 开发、优化、重构等工作时使用，也用于创建 spec、整理对话为 spec、继续推进已有 spec。
---

# Wayfinder

一个模糊想法可能大到无法由一个 agent session 完成，而且从当前位置到 **destination** 的路径还不可见。Wayfinder 先把这段工作组织成一个 **Spec**，再把当前已经看清的问题和执行切片写成 **Subspec**，逐个推进并持续清除 fog。

默认使用 repo 内的 Markdown 文件作为唯一 source of truth。不要创建或维护 issue、label、assignee、blocking relation、resolution comment，也不要通过 close issue 表示完成。只有用户明确要求同步到 issue tracker 时，才把 tracker 当作 mirror；文件中的 Spec 与 Subspec 仍是 canonical artifact。

## 路由

先判断本次调用属于哪种模式，然后**先读对应的 reference 文件，再开始行动**。不要仅凭本文件直接执行模式流程。

| 场景 | 必读文件 |
|---|---|
| 用户带着大而模糊的目标，需要 grill 收敛后创建 Spec | [references/create.md](references/create.md) |
| 当前对话已确认方向与决策，直接固化为 Spec，不重新访谈 | [references/distill.md](references/distill.md) |
| 用户提供已有 Spec 或 Subspec，要求继续推进 | [references/advance.md](references/advance.md) |

创建或修改任何 spec 文件前，另需读取格式约定：

- [Spec format](references/SPEC-FORMAT.md)
- [Subspec format](references/SUBSPEC-FORMAT.md)

## Domain model

- **Spec**：整个 effort 的 contract。它定义 destination、scope、共享决策、acceptance criteria，并索引 Subspec。
- **Subspec**：一个 agent session 内能够独立澄清、验证或实现的工作单元。详细问题、resolution 和 evidence 只保存在对应 Subspec 中。
- **Dependency**：Subspec frontmatter 中的 `depends_on`。只引用同一 Spec 下的 Subspec `id`。
- **Frontier**：`status: ready`、所有 dependency 都已 `resolved`、且尚未被其他 session claim 的 Subspec。
- **Fog**：确认属于 destination，但当前还无法精确表达成 Subspec 的范围。

不要把 Spec 当作 Subspec 内容的副本。Spec 只保留共享 contract、一句话 decision 摘要和链接；详细内容只存在于一个 Subspec。

## Storage

先遵守 repo 已有的 spec 根目录约定；没有约定时使用 `specs/`。每个 Spec 始终使用一个独立目录，主文件固定为 `main.md`，Subspec 与主文件同级：

```text
specs/
└── <yy-mm-dd>-<spec-slug>/
    ├── main.md                            ← canonical Spec
    ├── 1-<subspec-slug>.md
    ├── 2-<subspec-slug>.md
    └── ...
```

创建 Spec 时立即创建目录与 `main.md`，不根据内容大小切换存储结构。没有可独立推进的工作时，不创建空的 Subspec 文件。

写入任何 spec 文件前，先确认 repo 的 ignore 配置已排除 spec 目录（如 `specs/`）；没有就先加入。Spec 与 Subspec 是本地工作 artifact，不提交入库。

路径和 `id` 用于机器定位；面向人类的 narration、decision index 和交接说明始终使用 title，并把链接包在 title 上。

## Status and claim

Spec status：

- `draft`：仍有 planning Subspec 或 fog，执行 contract 尚未稳定。
- `ready`：共享决策已稳定，至少一个 implementation Subspec 位于 frontier。
- `in-progress`：已有 implementation Subspec 正在推进。
- `complete`：所有 required Subspec 均已满足 commit gate、状态为 `resolved`，且 Spec acceptance criteria 已验证。
- `paused`：用户明确暂停整个 effort。

Subspec status：

- `draft`：目标或 acceptance criteria 还不够清晰，不能进入 frontier。
- `ready`：内容清晰，可以在 dependency resolved 后推进。
- `in-progress`：已被一个 session claim。
- `resolved`：`Resolution` 与 `Evidence` 已写入文件，并满足下方的 commit gate。
- `cancelled`：已确认不再属于当前路线，并记录原因。

不要持久化 `blocked` status。是否 blocked 完全由 `depends_on` 指向的 Subspec status 推导，避免出现两份互相漂移的状态。

### Resolved gate

`resolved` 不是“验证通过”的同义词。只有在用户明确指定本轮相关改动需要 commit，且该 commit 已实际创建后，才能把任何 Subspec 标记为 `resolved`。用户未明确指定时不得自行 commit；即使实现或验证已经完成，也只能写入 `Resolution` 与 `Evidence`，保持 `status: in-progress`。Parent Spec 不得因这些 Subspec 而设为 `complete`，下游也不得把它们视为 resolved。

开始实质工作前，先把选中的 Subspec 从 `ready` 改为 `in-progress`，并写入 `owner`。这次 frontmatter 修改就是 claim。发现 Subspec 已是 `in-progress` 且 owner 不同，就跳过它。

## Subspec kinds

- **decision**（HITL）：需要用户参与的产品、domain 或架构决策。使用 grill-with-docs 与 domain-modeling，一次只解决一个问题。
- **research**（AFK）：通过源码、documentation、API 或其他可验证资料补齐决策所需事实。
- **prototype**（HITL）：制作低成本 artifact，让用户基于具体结果反馈。
- **task**（HITL 或 AFK）：解除后续决策阻塞所需的前置操作，本身不交付 destination。
- **implementation**（AFK，必要时 HITL）：实现已经稳定的 contract。交给 `implement` skill 推进。

Subspec 的 kind 描述工作性质，不表示 status。不要按 user story 机械拆分 Subspec；按可以独立验收、依赖明确的决策或实现切片拆分。

简化工作先使用 [`find-simplifications`](../find-simplifications/SKILL.md) 证明 consumer、ownership、behavior change 和净删除量。一个需要跨 session 推进的强候选可以形成一个独立 Spec；多个互不相关的候选不能只因来自同一次 audit 就共享 umbrella Spec。候选已经属于活动 Spec 的 destination 时，再按未知量写成 `research`、`decision` 或 `implementation` Subspec。

## Fog of war

Spec 故意允许不完整。判断内容应该成为 Subspec 还是留在 `Not yet specified`，只看问题现在能否被精确表达，而不是现在能否回答。

- 问题已经清晰：创建 Subspec，即使它仍被 dependency 阻塞。
- 问题还说不清：留在 `Not yet specified`，等 frontier 前进后再 graduate。

`Not yet specified` 不包含已作出的 decision、已有 Subspec 或 out-of-scope 内容。

destination 决定 scope。确认不属于 destination 的内容写入 Spec 的 `Out of scope`；如果已经存在对应 Subspec，将其改为 `cancelled` 并记录原因。只有重新定义 destination 时，才重新考虑这些内容。
