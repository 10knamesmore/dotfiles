---
name: implement
description: 根据 repo-local Spec 推进一个 implementation Subspec，完成代码、验证、resolution 与 Spec 状态更新。
---

# Implement

一次只实现一个 `kind: implementation` 的 Subspec。

## Process

1. 读取 repo instructions、完整 Spec、目标 Subspec，以及它直接依赖的 resolved Subspec。格式约定见 `../wayfinder/references/SPEC-FORMAT.md` 和 `../wayfinder/references/SUBSPEC-FORMAT.md`。
2. 如果用户只提供 Spec，读取各 Subspec frontmatter，按 Spec index 顺序选择第一个位于 frontier 的 implementation Subspec。不要让用户手动选择。
3. 验证目标 `status: ready` 且所有 `depends_on` 均为 `resolved`。如果 kind 不是 `implementation`，交回对应 workflow；如果没有 frontier，报告具体 dependency、draft Subspec 或 stale claim。
4. 开始实质工作前，把目标 status 改为 `in-progress` 并写入 `owner`。发现其他 owner 已 claim 时，跳过它。
5. 在约定的 seam 上实现 acceptance criteria。只有用户明确要求时才新增、修改或删除测试；测试未获授权时，使用现有测试和其他验证，不要用通用 test-first 流程覆盖 repo instructions。
6. 完成实现后，运行项目要求的 formatter、type check、lint 和现有 tests。使用 [`prose-standard`](../prose-standard/SKILL.md) 检查 touched code 是否需要同步注释，以及当前 diff 中新增或修改的其他持久 prose；发现 session、PR、review 或草稿视角时再使用 [`trim-cot-leakage`](../trim-cot-leakage/SKILL.md)。若安装了 code-review skill，使用它 review 当前 diff，并处理确认的问题。
7. 只有实际验证通过后，才在 Subspec 中填写：
   - `Resolution`：实现结果和重要 contract。
   - `Evidence`：运行过的命令、结果及必要的源码或 artifact 链接。
   - 只有用户明确指定本轮相关改动需要 commit，且该 commit 已实际创建后，才能写入 `status: resolved`。
   - commit gate 未满足时，保留 `status: in-progress`，可以写入已有的 `Resolution` 与 `Evidence`，但不得把它当作 resolved dependency。
8. 更新 Spec：
   - 第一个 implementation 开始后设为 `in-progress`。
   - 只有所有 required Subspec 均已满足 commit gate、状态为 `resolved`，且 Spec acceptance criteria 已验证后，才能设为 `complete`。
   - 新问题能精确表达时创建新 Subspec，否则写回 `Not yet specified`。

验证失败、工作未完成或 commit gate 未满足时保持 `in-progress`，在 `Resolution` 中记录 blocker 或当前结果，不得声称 resolved。

不要自动 commit 或 push。只有用户明确要求，或 repo instructions 明确授权当前 workflow 时才执行 Git 写操作；但 `resolved` commit gate 只接受用户明确指定本轮相关改动需要 commit 且该 commit 已实际创建的情形。
