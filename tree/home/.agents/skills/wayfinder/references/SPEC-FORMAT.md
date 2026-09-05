# Spec format

Spec 是一个 effort 的 canonical contract。目录与主文件命名始终遵守本格式；正文先遵守 repo 已有模板，没有模板时使用下文结构。

Spec 主文件路径为 `specs/<yy-mm-dd>-<spec-slug>/main.md`。每个 Spec 始终使用一个独立目录；Subspec 与 `main.md` 同级，路径为 `specs/<yy-mm-dd>-<spec-slug>/<n>-<subspec-slug>.md`（`n` 从 1 开始递增）。

写入前先确认 repo 的 ignore 配置已排除 spec 目录；没有就先加入。Spec 与 Subspec 是本地工作 artifact，不提交入库。

## Frontmatter

```yaml
---
id: editable-queue
title: Editable Queue
status: draft
---
```

- `id`：在 repo 内稳定、唯一的 kebab-case identifier。
- `title`：面向人类的名称。
- `status`：`draft` 表示仍有未定决策或 fog；`ready` 表示共享决策已稳定且至少一个 implementation 位于 frontier；`in-progress` 表示实现已开始；`complete` 表示整体验收完成；`paused` 表示用户明确暂停。完成与后续修正遵守 [Wayfinder](../SKILL.md#完成与修正)。

## Body

```markdown
# <title>

## Destination

<完成时真实世界中发生了什么变化。>

## Problem statement

<从用户或系统视角描述当前问题。>

## Solution

<从用户或系统视角描述预期结果，不写容易过时的逐文件实现步骤。>

## User stories

1. 作为 <actor>，我希望 <capability>，从而获得 <benefit>。

## Acceptance criteria

- <可观察、可验证的完成条件。>

## Shared implementation decisions

- <跨多个 Subspec 生效的 contract 或 architecture decision。>

## Testing decisions

- <测试 seam、外部行为和已有测试范例。>

## Subspecs

- [<Subspec title>](<n>-<subspec-slug>.md)

## Decisions so far

- [<Resolved Subspec title>](<n>-<subspec-slug>.md) — <一句话摘要>

## Not yet specified

- <属于 destination、但还无法精确表达成 Subspec 的 fog。>

## Out of scope

- <明确排除的内容及原因。>

## Notes

<每个 session 都应知道的 domain、skills 或长期约束。>
```

## Invariants

- Spec 不复制 Subspec 的 status、dependency、完整 resolution 或 evidence。
- `Subspecs` 是稳定 index；完成后不删除条目。
- `Decisions so far` 只索引已 resolved 的 decision，不替代原 Subspec。
- `complete` 必须同时满足所有 required Subspec 均为 `resolved`，以及 acceptance criteria 已验证。
- 存在真实 trade-off 时，可以增加 `Alternatives considered` 与 `Risks and consequences` section；只记录实际考虑过的 alternative，不为填模板编造陪衬。
