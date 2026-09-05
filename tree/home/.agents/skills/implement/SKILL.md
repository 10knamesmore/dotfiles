---
name: implement
description: 根据已有本地 Spec 实现一个 implementation Subspec，完成代码、验证、结果记录与状态更新。
---

# Implement

一次实现一个 `kind: implementation` 的 Subspec，再由 Wayfinder 继续授权范围内的下一个目标。

1. 读取项目指令、Parent Spec、目标 Subspec 和它直接依赖的结论；根据实现问题读取相关源码与验证入口。
2. 用户只提供 Spec 时，优先继续当前执行者已 claim 的目标，否则按 Spec index 选择授权范围内第一个 frontier。没有可继续目标时报告具体 dependency、draft 或 stale claim，不让用户手动挑选可推导的任务。
3. 确认目标所有 `depends_on` 均为 `resolved`；未 claim 的目标从 `ready` 改为 `in-progress` 并写入 owner。当前执行者可继续自己的 claim；不覆盖其他 owner。非 implementation 目标交回对应 workflow。
4. 实现 acceptance criteria，遵守项目的测试约定。使用风险相称的既有验证；不引入通用 test-first 流程，也不为流程本身增加测试。
5. 检查改动是否引入需要说明的业务概念、字段语义、失败条件或不变量；需要编辑注释或其他持久 prose 时使用 [prose-standard](../prose-standard/SKILL.md)。检查真实 diff 与用户可见结果，修复属于本次改动的问题。
6. 在 `Resolution` 与 `Evidence` 中记录结果、实际验证和未完成项，按 [Wayfinder 的完成与修正规则](../wayfinder/SKILL.md#完成与修正) 更新 Subspec 与 Parent Spec。验收失败时继续修正，不能以状态更新代替完成工作。

新问题属于当前验收要求时直接解决；独立的新范围写入对应 Subspec，尚不清晰的写入 Spec 的 `Not yet specified`。只按已有授权执行 commit 或 push。
