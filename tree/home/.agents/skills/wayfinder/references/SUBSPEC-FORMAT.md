# Subspec format

Subspec 是一个 agent session 内可以独立推进的工作单元。一个结论只保存在一个 Subspec 中。

## Frontmatter

```yaml
---
id: queue-mutation-semantics
title: Define Queue Mutation Semantics
kind: decision
status: ready
depends_on: []
---
```

- `id`：在所属 Spec 内稳定、唯一的 kebab-case identifier。
- `title`：面向人类的名称。
- `kind`：`decision`、`research`、`prototype`、`task` 或 `implementation`。
- `status`：`draft`、`ready`、`in-progress`、`resolved` 或 `cancelled`。
- `depends_on`：同一 Spec 下的 Subspec `id` 列表。空列表表示没有 dependency。
- `owner`：只在 claim 后添加，用可辨认的 human 或 agent session label 表示当前负责人。

`blocked` 不属于 status。只要任一 `depends_on` 尚未 `resolved`，该 Subspec 就是 derived blocked。

## Body

```markdown
# <title>

Parent: [<Spec title>](../SPEC.md)

## Objective

<本 Subspec 唯一要回答的问题、验证的假设或交付的结果。>

## Context

<开始工作所需的局部背景和已 resolved dependency 链接。>

## Scope

- <本 Subspec 包含的内容。>

## Acceptance criteria

- <判断本 Subspec resolved 的可验证条件。>

## Resolution

<完成前保持为空；完成后记录 decision、research finding、prototype feedback、task result 或 implementation outcome。>

## Evidence

- <源码、documentation、测试命令与结果、artifact 或其他可复查证据。>

## Follow-ups

- <新浮现但不属于本 Subspec 的工作；清晰时链接到新 Subspec，否则回写 Spec fog。>
```

## State transitions

```text
draft -> ready -> in-progress -> resolved
                    |
                    +-> ready
draft/ready/in-progress -> cancelled
```

- claim 前把 `ready` 改为 `in-progress` 并添加 `owner`。
- claim 失效或主动释放时可以回到 `ready`，并删除 stale `owner`。
- 只有写完 `Resolution` 与 `Evidence` 并完成实际验证后，才能改为 `resolved`。
- `cancelled` 必须在 `Resolution` 记录原因，并同步 Spec 的 scope 或路线。
