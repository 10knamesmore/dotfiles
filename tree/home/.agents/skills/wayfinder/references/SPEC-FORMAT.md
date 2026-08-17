# Spec format

Spec 是一个 effort 的 canonical contract。先遵守 repo 已有模板；没有模板时使用本格式。

Spec 文件路径为 `specs/<yy-mm-dd>-<spec-slug>.md`。优先单文件：只有内容确实太大、单文件无法容纳时才拆分，Subspec 放在同名目录 `specs/<yy-mm-dd>-<spec-slug>/<n>-<subspec-slug>.md`（`n` 从 1 开始递增）。

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
- `status`：`draft`、`ready`、`in-progress`、`complete` 或 `paused`。

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

- [<Subspec title>](<yy-mm-dd>-<spec-slug>/<n>-<subspec-slug>.md)

## Decisions so far

- [<Resolved Subspec title>](<yy-mm-dd>-<spec-slug>/<n>-<subspec-slug>.md) — <一句话摘要>

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
- `complete` 必须同时满足 required Subspec resolved 和 acceptance criteria 已验证。
- 存在真实 trade-off 时，可以增加 `Alternatives considered` 与 `Risks and consequences` section；只记录实际考虑过的 alternative，不为填模板编造陪衬。
