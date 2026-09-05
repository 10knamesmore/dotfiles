---
name: grill-with-docs
description: 通过追问澄清尚未确定的产品或设计决策，并记录到当前 Subspec 或领域文档；已确认的决定不重新访谈。
---

# Grill with Docs

一次只问一个会改变当前决策的问题。先利用已有对话、源码与文档回答事实问题，把需要用户选择的行为、范围和 trade-off 留给用户；方向已经明确时继续工作，不为走流程追问。

有活动 Spec 时，每个独立决策放在一个 `kind: decision` 的 Subspec，用户确认后及时更新它的 Context 或 Resolution。Spec 只保存跨 Subspec 的约定摘要和链接。新问题清晰时创建 Subspec，否则写入 Spec 的 `Not yet specified`。

没有活动 Spec 时，确认的事实保存在对话与相关领域文档中，不为一次讨论创建 Spec；需要跨会话推进时再使用 [Wayfinder](../wayfinder/SKILL.md)。

决策经用户确认、验收条件满足且证据已记录后，按 [Wayfinder 的完成与修正规则](../wayfinder/SKILL.md#完成与修正) 更新状态。确认决策不表示实现已经完成。

## 领域术语与 ADR

- 新术语或现有术语含义改变时，更新项目已有的 glossary 或领域文档，写清业务含义及与相近概念的区别。没有独立 glossary 时，先放在拥有该概念的文档中，不默认创建额外文件。
- 只有决策同时满足以下条件时才创建 ADR：难以逆转、缺少上下文会令人意外、存在真实 trade-off。普通局部实现选择不写 ADR。
- ADR 使用项目已有位置与格式；没有约定时放在 `docs/adr/`，包含问题、已选方案、实际考虑过的替代方案、取舍后果和证据。不要编造替代方案或抄录访谈轮次。
- 本地 Spec 保存当前工作的决策和证据；需要随源码长期保存的设计原因由领域文档或 ADR 持有，不复制整份 Spec。写入文档不授予 commit 或 push 权限。
