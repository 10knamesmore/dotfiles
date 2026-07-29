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
5. 在约定的 seam 上 test-first 实现 acceptance criteria。测试命令、运行范围和顺序以 repo instructions 为准；不要用通用流程覆盖项目约定。
6. 完成实现后，运行项目要求的 formatter、type check、lint 和 tests。若安装了 code-review skill，使用它 review 当前 diff，并处理确认的问题。
7. 只有实际验证通过后，才在 Subspec 中填写：
   - `Resolution`：实现结果和重要 contract。
   - `Evidence`：运行过的命令、结果及必要的源码或 artifact 链接。
   - `status: resolved`。
8. 更新 Spec：
   - 第一个 implementation 开始后设为 `in-progress`。
   - 所有 required Subspec resolved 且 Spec acceptance criteria 已验证后设为 `complete`。
   - 新问题能精确表达时创建新 Subspec，否则写回 `Not yet specified`。

验证失败或工作未完成时保持 `in-progress`，在 `Resolution` 中记录 blocker，不得声称 resolved。

不要自动 commit 或 push。只有用户明确要求，或 repo instructions 明确授权当前 workflow 时才执行 Git 写操作。
