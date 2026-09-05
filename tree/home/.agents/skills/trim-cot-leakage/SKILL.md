---
name: trim-cot-leakage
description: 审计或修复仓库文档、注释和指令中的编写过程残留、过期快照与不可解析引用。普通 prose 编辑使用 prose-standard。
---

# Trim Chain-of-Thought Leakage

检查文字是否依赖只有编写会话、PR 或 review 参与者才知道的上下文。读者只拥有当前仓库、可达历史与公共资料时，应能解析每个引用并验证事实。

## 判断与范围

- 普通文档描述当前行为。活动 Spec 可以表达未来计划，ADR、postmortem、changelog 和完成态 evidence 可以保留真实历史，不能因出现过去时就删除。
- 修改前按 [prose-standard](../prose-standard/SKILL.md) 保留完整含义；事实和来源不能随编写过程一并删除，也不能把假设改成承诺。
- 遵循用户的审查或修改权限，只处理指定范围或自然形成的 diff。第三方、fixture、snapshot、录制输出和冻结历史保留原文；生成物先定位源头。

## 审计

明确的局部问题可直接阅读并修正。对范围性审计，用 [recall-batteries.md](references/recall-batteries.md) 召回候选，再阅读范围内 prose 密集处，避免仅凭关键词判断。需要判别类型、引用或历史材料时读 [review-guide.md](references/review-guide.md)；边界不确定时用 [examples.md](references/examples.md) 校准。

把事实改成可核验的当前行为，把长期解释放到拥有它的文档，把尚未完成的工作放到可解析的活动 Spec、issue 或 TODO。不要为修正文档中的引用而自行重构代码。

修改后检查相关候选与引用，确认条件、modality、测量来源和失败后果仍完整。范围性审计可重跑相关 batteries；局部改动用对应最窄检查，不重复扫描无关内容。报告实际修改与未解决项，不把关键词无命中当作语义正确的证明。
