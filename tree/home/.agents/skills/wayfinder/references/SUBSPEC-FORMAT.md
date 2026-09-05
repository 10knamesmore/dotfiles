# Subspec format

Subspec 是一个 agent session 内可以独立推进的工作单元。一个结论只保存在一个 Subspec 中。

Subspec 与 Spec 的 `main.md` 同级，路径为 `specs/<yy-mm-dd>-<spec-slug>/<n>-<subspec-slug>.md`（`n` 从 1 开始递增，与 frontmatter 的 `id` 无关）。只为可以独立推进的工作创建 Subspec，不预留空文件。

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
- `status`：`draft` 表示目标或验收标准尚不明确；`ready` 表示内容已清晰，可在依赖完成后推进；`in-progress` 表示已 claim；`resolved` 表示验收完成；`cancelled` 表示已确认不再属于当前路线。完成与后续修正遵守 [Wayfinder](../SKILL.md#完成与修正)。
- `depends_on`：同一 Spec 下的 Subspec `id` 列表。空列表表示没有 dependency。
- `owner`：只在 claim 后添加，用可辨认的 human 或 agent session label 表示当前负责人。 比如`claude code xxx`/`kimi xxx`

`blocked` 不属于 status。只要任一 `depends_on` 尚未 `resolved`，该 Subspec 就是 derived blocked。

## Body

```markdown
# <title>

Parent: [<Spec title>](main.md)

## Objective

<本 Subspec 唯一要回答的问题、验证的假设或交付的结果。>

## Context

<开始工作所需的局部背景和已 resolved dependency 链接。>

## Scope

- <本 Subspec 包含的内容。>

## Acceptance criteria

- <判断本 Subspec resolved 的可验证条件。>

## Resolution

<记录已确定的结果或当前差异；未完成时明确剩余项，不把部分结果写成已验收。>

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
resolved -> in-progress
draft/ready/in-progress -> cancelled
```

- claim 前把 `ready` 改为 `in-progress` 并添加 `owner`。
- claim 失效或主动释放时可以回到 `ready`，并删除 stale `owner`。
- 验收满足并写入 `Resolution` 与 `Evidence` 后，按 Wayfinder 的完成规则设为 `resolved`；未完成时保持 `in-progress` 并记录剩余项。
- 反馈或证据表明结果不符时，在原文件恢复 `in-progress`、更新 owner 与差异，并同步 Parent Spec，不另建 reopen 流程。
- `cancelled` 必须在 `Resolution` 记录原因，并同步 Spec 的 scope 或路线。
- decision 或 simplification proposal 存在真实 trade-off 时，可以增加 `Alternatives considered` 与 `Risks and consequences` section；不复制到 Spec，也不编造没有实际评估的 alternative。
