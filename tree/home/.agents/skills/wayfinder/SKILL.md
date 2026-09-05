---
name: wayfinder
description: 为需要跨会话推进的复杂目标创建和维护本地 Spec/Subspec，或继续用户指定的已有 Spec。单次可完成的明确改动不使用，除非用户要求写 Spec。
---

# Wayfinder

用 repo 内的 Spec 保存整体目标，用 Subspec 组织可独立澄清、验证或实现的工作。工作已明确且一次会话可完成时直接推进；用户明确要求写 Spec 时按要求落盘。

## 路由

只读取当前模式对应的 reference：

| 场景 | 文件 |
| --- | --- |
| 从大而模糊的目标收敛设计并创建 Spec | [create.md](references/create.md) |
| 将已确认的对话整理为 Spec，不重新访谈 | [distill.md](references/distill.md) |
| 推进已有 Spec 或 Subspec | [advance.md](references/advance.md) |

创建或修改 Spec 时读取 [SPEC-FORMAT.md](references/SPEC-FORMAT.md)；创建或修改 Subspec 时读取 [SUBSPEC-FORMAT.md](references/SUBSPEC-FORMAT.md)。不要为了读取已有文件而加载全部模板。

## 工作模型

- **Spec**：整个目标的约定，包括 destination、scope、共享决策、验收标准和 Subspec 索引。
- **Subspec**：可独立推进的工作单元，详细结论和证据保存在该文件，不复制到 Spec。
- **Frontier**：`ready`、所有 `depends_on` 均为 `resolved`、尚未被其他执行者 claim 的 Subspec。
- **Fog**：属于目标但还无法精确表达成 Subspec 的问题，保存在 `Not yet specified`。已有结论、已有 Subspec 和 out-of-scope 不属于 fog。

Subspec 按可独立验收的决策或实现切片划分，不按 user story 机械拆分。`decision` 使用 [grill-with-docs](../grill-with-docs/SKILL.md)；`research` 查证事实；`prototype` 产出可反馈的 artifact；`task` 完成前置操作；`implementation` 使用 [implement](../implement/SKILL.md)。简化候选的调查使用 [find-simplifications](../find-simplifications/SKILL.md)。

先遵守 repo 的 spec 根目录约定，没有约定时用 `specs/`。每个 Spec 使用独立目录和 `main.md`，Subspec 与主文件同级。写入前确认 ignore 配置排除了该目录；Spec/Subspec 是本地工作文件，不提交入库。只有用户要求时才同步到 issue tracker，tracker 作为镜像，不取代本地文件。

路径和 `id` 用于定位；面向人的说明使用 title 并链接到文件。明确不属于目标的内容写入 `Out of scope`，已有对应 Subspec 则标为 `cancelled` 并记录原因。

## 完成与修正

- Subspec 的验收标准满足，且 `Resolution`、`Evidence` 已记录实际结果，才设为 `resolved`。需要用户选择的决策或反馈必须已取得；实现必须完成约定的验证，不能用编译通过代替用户可见效果。
- 用户明确要求先 review 时，保留 `in-progress` 等待该 review；没有这种要求时，不增加默认的人工验收步骤。
- commit 与验收状态独立，只按用户或项目对当前工作的明确授权执行 commit/push。未提交不会阻止已验收的 Subspec 解除依赖。
- 所有 required Subspec 均为 `resolved`、整体验收标准已验证，才将 Spec 设为 `complete`。用户明确暂停时设为 `paused`；其余状态见格式文件。
- 用户反馈或新证据表明结果不符合已定要求时，在原 Subspec 继续修正，更新为 `in-progress` 并记录当前 owner、差异和待验证项；Parent Spec 恢复为 `in-progress`。不要求用户办理 reopen，也不另建重复 Subspec。只复查受该差异影响的依赖结论；若发现其验收也失效，同样修正状态。

一次只 claim 一个 Subspec，除非用户明确授权并行 research。claim 将 `ready` 改为 `in-progress` 并写入 owner；当前执行者可继续自己的 claim，不覆盖其他执行者的工作。`blocked` 由 dependency 状态推导，不另存 status。

在授权范围内按依赖继续推进，不把一个 Subspec 或一次会话当作交付上限。只有用户暂停、出现必须由用户决定的事项或无法继续的实际阻塞时停下依赖该事项的工作，并明确已完成与未完成的结果。
