# 固化 Spec（从当前对话）

当前对话里方向与决策已经聊清楚，直接整理为 Spec 与初始 Subspec。不要重新访谈用户，也不要把不确定内容伪装成 decision。如果还不了解代码库，先 explore 当前实现。

1. 读取 repo instructions、现有 spec 约定、domain glossary 和相关 ADR。
2. 从对话中提取 destination、problem、solution、user stories、acceptance criteria、共享实现决策、测试决策和 out-of-scope。
3. 确定测试 feature 的 seam。优先使用已有 seam，并使用项目现有测试范例。没有充分证据时，把 seam 问题写入 `Not yet specified` 或 decision Subspec，不要编造。
4. 按 SPEC-FORMAT 创建 Spec 目录与 `main.md`。
5. 为当前可以精确表达的工作创建初始 Subspec：
   - 已经可以精确表达、但尚未决定的问题写成 `decision`、`research` 或 `prototype`。
   - 为解除决策阻塞必须完成的前置操作写成 `task`。
   - 只有 contract 与 acceptance criteria 已稳定时，才创建 `implementation` Subspec。
   - 不要按 user story 一对一机械拆分；按可独立验收、dependency 明确的工作切片拆分。
6. 全部 Subspec 创建后，再写 `depends_on`。Spec 的 index 只保存 title 与相对链接，不复制 Subspec status 或 dependency。
7. 仍有 planning Subspec 或 fog 时，Spec status 为 `draft`；所有共享决策稳定且至少一个 implementation Subspec 位于 frontier 时，才设为 `ready`。
8. 最后检查 Spec 中每项事实都有对话、源码、documentation 或 ADR 依据。不确定项必须显式留在 fog 或 Subspec 中。

固化只综合已有上下文并落盘，不推进任何 Subspec。
