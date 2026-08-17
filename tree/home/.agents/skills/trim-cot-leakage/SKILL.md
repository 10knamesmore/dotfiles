---
name: trim-cot-leakage
description: 审计或修复仓库 prose 中泄漏的编写过程时使用，包括无法解析的 decision/task/Spec 编号、PR 或 review 视角、版本与改动 narration、面向 reviewer 的辩护、控制流 walkthrough、无 owner 的规划残留和混入的工作语言。适用于 comments、docs、Spec 完成态、ADR、Skill 和 agent instruction。
---

# Trim Chain-of-Thought Leakage

Chain-of-thought leakage 是从编写 session 而不是当前仓库观察事实的 prose：引用只有当时可见的 artifact，叙述编辑过程而不是最终状态，或与已经离场的 reviewer 对话。

修复不能只做删除。passage 含有事实时，先按 [`prose-standard`](../prose-standard/SKILL.md) 枚举并保留完整命题，再从当前 checkout 的视角重述；只有 audit code、控制流 narration 等不承载事实的内容才直接删除。本 Skill 是 guidance，不是正则替换脚本。

## The one test

对每个可疑 passage 问：只拥有当前 checkout、git 中可达历史和公共资料，不知道任何 session transcript、PR thread 或未提交草稿的读者，能否解析每个引用并验证每项 claim？

不能时，保留可验证事实，改成仓库视角，删除其余 transcript。能解析只表示没有泄漏；README、API docs 和代码注释等 current-state surface 中，可解析的改动故事仍可能放错位置。

活动 Spec、Subspec 和 proposal 的职责就是表达未来状态、open question 和计划，因此 future tense 本身不是泄漏。它们完成后必须记录真实 resolution，不能把草稿轮次、review choreography 或已失效计划当作最终事实。

## Taxonomy

1. **Dead design-session citation:** `(decision 7)`、`audit C2`、`design §4.7`、`task T4`、`B 方案`。存在 committed owner 时改成可解析的名称和路径；不存在时删除 citation，但保留其中事实。
2. **Stack and PR vantage:** `this PR adds`、`later PR in this stack`、`previous commit`。改写成 shipped mechanism 或 extension point；尚未完成的工作放入有 owner 的 issue、TODO 或活动 Spec。
3. **Change narration and version stamp:** `used to`、`no longer`、`the old implementation`、`v1`、`this cut`、`today`、对比过去状态的 `now`。普通 current-state surface 直接写现有行为；regression pin 改成 present-tense counterfactual，例如 `Without X, Y happens`。
4. **Review choreography:** `rejected in review`、`the reviewer confirmed`、`round 3`、`v5 of this note`。ADR 或 decision record 可保留真实 alternative 与 rationale，但不保留谁在哪一轮说了什么。
5. **Reviewer-addressed justification:** `the cast is safe`、`this is correct because`。写明使其安全的 invariant 和误改后果；代码已经显然表达时删除。
6. **Restatement and derivation transcript:** `first X, then Y`、测试 walkthrough、显然分支的 proof。删除；只保留非显然 contract、assertion rationale 或 invariant。
7. **Hedge and planning residue:** `probably fine for now`、`should be enough`、没有 owner 的 deferral。改成可验证 bound 和 failure behavior，或交给明确的 issue、TODO、Spec；不要保留 hedge。
8. **Authoring-language slip:** 英文 prose 中混入未翻译工作语言、私有分隔符和 session shorthand，或中文 prose 中出现相反情况。翻译为目标 surface 的语言，或删除无事实内容。

## What is not leakage

- issue、TODO、FIXME 或其他在当前仓库流程中能解析的 follow-up owner；
- ADR、postmortem、changelog 和 completed Spec evidence 中可验证的历史与 trade-off；
- suppression、coverage ignore、empty catch 等工具要求的理由；理由错误时修正，不能直接删除；
- `Without X, Y happens` 等 counterfactual-present regression pin；
- 带来源的 measured bound；`measured`、benchmark path 和测量条件不能丢；
- runtime lifecycle 中的 old/new object，例如旧连接 drain 后新连接接管；
- RFC section、标准文档、committed design doc、Figma frame 等按设计在仓库外或仓库内可解析的引用；
- proposal 的 alternatives、future tense 和未完成 acceptance criteria；
- recorded model output、fixture、snapshot 和冻结历史中的原始声音。

使用 [`examples`](./references/examples.md) 校准这些边界，尤其检查 proposition 被误删、modality 翻转和假设被升级成事实的情况。

## Workflow

1. 确认 scope、write authority 和 surface lifecycle。用户未给范围但当前 diff 是自然边界时只检查 diff；不要自行全仓库清洗。
2. 排除第三方和只读 vendored 内容、fixture、snapshot、录制输出、生成物和冻结历史。生成物从 owner 修复；model-visible wording 可能是行为，不能静默重写。
3. 先只读审计：运行 [`recall batteries`](references/recall-batteries.md)，再无关键词地阅读 scope 中 prose 最密集的部分。每个 hit 都要语义判断；pattern 不是定义。
4. 删除前枚举 passage 的 actor、condition、modality、negative guarantee、ownership、failure 和 consequence。
5. owner-first 修改：事实改在 owning source，引用改到可解析 owner，活动工作改到 issue、TODO 或 Spec。
6. 逐项检查 overcorrection：不能把 obligation 改成 endorsement，不能把 hypothetical 改成 shipped feature，不能随 transcript 删除真实事实，不能丢掉 measurement provenance。
7. 重新运行 batteries，确认剩余 hit 都是 deliberate keep；检查每个 citation 在当前 checkout 可解析。
8. 运行 touched surface 已有的最窄验证和 `git diff --check`。未经用户明确要求，不新增或修改测试。
