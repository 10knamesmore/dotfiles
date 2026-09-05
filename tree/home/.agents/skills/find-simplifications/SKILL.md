---
name: find-simplifications
description: 调查并证明值得删除、合并或简化的代码与依赖，形成有证据的 proposal。用于简化审计；审计本身不授权实施。
---

# Find Simplifications

把宽泛的 simplify 请求转成少量、证据充分、可以独立决定和验收的 proposal。目标是删除、合并、降级或重新归属真实 surface area，而不是制造 cleanup inventory。本 Skill 是 guidance，不是 dead-code 工具的包装或候选数量清单。

本 Skill 负责发现和证明候选。用户只要求 audit、review 或 proposal 时，不修改实现；用户批准实施后，单次会话可完成的明确工作直接实现；已有 Spec 使用 [`implement`](../implement/SKILL.md)，需要跨会话组织的目标使用 [`wayfinder`](../wayfinder/SKILL.md)。

## Start with repository context

1. 读取 repo instructions 和候选附近的代码、调用方与验证入口；涉及领域约定、跨模块结构或持久化行为时，再查对应 glossary、架构文档、ADR 或活动 Spec。
2. 先识别项目明确保护的 seam、兼容承诺、durable format、安全边界和产品能力。不能因为实现复杂就把有 owner 的能力当成 cleanup。
3. 检查项目已有依赖、标准库和 runtime floor，再考虑新增 dependency 或自写替代。
4. 明确 scope。用户未给范围但当前 diff 或目标模块形成自然边界时使用该边界；不要自行扩成全仓库重构。

## Strong candidates

强候选必须删除、折叠、降级或重新归属真实成本，并有证据说明当前设计付出的成本高于收益：

- public method、event、config knob、registry notification、helper、package、durable event 或 test artifact 没有 production consumer；
- 私有函数只有一个消费方，却占用模块级作用域；实施时把它定义在该消费函数或对应代码块内，同一消费函数内多次调用仍算一个消费方。trait 方法、公开 API 和宏或属性要求的具名入口不属于这类私有 helper；
- tests 或 docs 是唯一 consumer，且其行为不是产品、兼容或安全 contract；
- 两个 representation 镜像同一事实，尤其跨 durable 与 transient state；
- seam 强迫所有 implementation 支持没有 consumer 的 method；
- package 只承载 test、demo 或 support code，却带来发布和 dependency overhead；
- feature 为没有 product owner 的未来一般性支付持续复杂度；
- rollback path、validator、expected output 或 special case 只保护未使用 API；
- hand-rolled parser、framer、retry、glob、diff 或类似 infrastructure 已由当前依赖、成熟维护库或 runtime builtin 覆盖；
- 多个 sentinel、promise、flag 或 disposer 表达同一个 lifecycle transition，可以由一个明确 owner 取代。

仅删除 typo、运行一次 unused-symbol 工具、指出代码看起来复杂，或移除已有明确 rationale 的 seam，不足以形成强 proposal。

任何 observable behavior 变化都必须显式列为 decision 和 trade-off，不能伪装成 cleanup。不要自行设计 fallback 或 compatibility path。

## Survey broadly enough

从 production delta、largest modules、public surfaces 和跨模块 lifecycle 开始，不要在找到第一个 unused symbol 后停止。breadth 与用户请求成比例：目标是少量高置信候选，不是扫描报告越长越好。

用户明确要求大范围并行调查时，可以按互不重叠的 domain 分配 subagent，并要求每个结果提供源码证据；未得到明确请求时不要为了流程本身启动 subagent。

静态工具只用于召回：先用 `rg` 搜 exact symbol、wire string、config key、event name 和调用形式，再阅读定义、构造方、caller、持久化边界和错误路径。unused-symbol 工具、dependency graph 和 coverage 不能替代 dynamic registration、loader path、public interface 与文档 contract 的语义判断。

## Trust, ownership, and lifecycle

对每个 defensive copy、freeze、validator 和 callback capture，说明 value 从哪里来、交给谁、下一 owner 是否跨越 process、wire、queue、persistence 或 untyped input 边界。

same-process typed call 通常可以借用 readonly value；parser、config loader、queue、model/tool JSON、durable file、worker、process 和 wire decoder 应拥有或验证输入。围绕 hostile getter、伪造 typed object、handoff 后 mutation 构造的 tests，可能证明 speculative contract，而不是自动证明 defensive machinery 必须保留。

对复杂 async code 绘制 ownership graph，将每个 sentinel、readiness promise、cancellation path、disposer 和 state flag 映射到 distinct owner 或 transition。多个机制镜像同一 liveness 或 settlement fact 时，优先提出一个 transaction 或 lifecycle controller；同步 publication/rollback、callback containment、first-terminal-outcome arbitration、process ownership 和 dispose-to-quiescence 可能需要独立机制。

## Dependency instead of hand rolling

dependency swap 是有效简化，但必须证明净删除而不是搬家：

- 读取 hand-rolled implementation，列出成熟库、当前依赖或 builtin 精确覆盖的 surface；
- 列出 package 未覆盖而仍需保留的 semantics；
- 查证 maintenance、adoption、transitive footprint、license 和 runtime compatibility；
- 先检查 repo 现有 dependency 是否已经提供能力；
- 比较删除的 implementation、专属 tests 和 docs，与新增 glue 和 dependency cost；
- wrapper 只是把同样复杂度换位置时，拒绝该候选。

库/API 行为不确定时读官方文档和源码，不依赖记忆。

## Prove or reject every candidate

先分类 consumer：

- **Production:** application/library source、真实入口、runtime script、loader、config 和部署路径；
- **Non-production:** tests、README、docs、Spec、ADR、snapshot、generated expected output 和 comments；
- **Ambiguous:** example、benchmark、migration 和 smoke script。阅读调用方式后再分类。

每个 proposal 至少回答：

1. 当前 owner、输入、输出和 lifecycle 是什么？
2. production 与 non-production consumer 分别有哪些？
3. 哪些行为、类型、持久化数据或 compatibility obligation 会保留或消失？
4. 删除哪些 code、test artifact、docs、config 和 generated output，剩余 glue 是什么？
5. 哪个现有 decision 或 invariant 支持或反对该候选？
6. 可观察 end state 和验证方式是什么？

遇到下列情况时拒绝或降级：

- 存在 production caller，删除属于 feature decision 而不是 cleanup；
- 现有 ADR、Spec resolution 或 hard-won defensive pattern 已说明理由，新证据没有推翻它；
- 需要大量无关 churn，却不减少 public API、state 或 required behavior；
- idea 正确但太局部，不值得形成 durable proposal；只有用户授权写入时才添加 actionable TODO/FIXME；
- 证据只来自 tests。passing test 证明当前行为存在，不单独证明它仍值得保留或删除。

## Route the proposal into the workflow

持久 design proposal 与本地 Spec 的信息模型接近，但 persistence 和粒度不同：proposal 都需要 problem、solution、alternatives、acceptance criteria 与 risk；Spec 还组织跨 session 的 dependency 和 implementation frontier。

- **已有活动 Spec，且候选属于同一 destination:** 按当前未知量写入 `research`、`decision` 或 `implementation` Subspec，不固定塞进某一种 kind。
- **没有活动 Spec，候选需要跨 session:** 每个 cohesive destination 交给 Wayfinder 创建一个独立 Spec。多个互不相关候选不能只因来自同一次 audit 就塞进一个 umbrella Spec。
- **候选已清晰且一个 session 可完成:** 在 response 中给出完整 proposal，用户确认后直接实现；不要为了模仿 durable design note 强制创建 Spec。
- **候选只是后续 decision 的证据:** 放进 owning decision Subspec 的 Context、Resolution 或 Evidence，不另建重复文档。
- **完成后的 rationale 需要长期提交进仓库:** 只有满足 [grill-with-docs 的 ADR 条件](../grill-with-docs/SKILL.md#领域术语与-adr)时才创建 ADR；本地 complete Spec 不是 committed decision record。

proposal 应包含：

- **Problem:** 当前 API/state/mechanism、production 与 non-production consumer 证据，以及实际维护成本；
- **Proposal:** 精确说明 remove、fold、demote 或 rehome 的内容；
- **Alternatives considered:** 只写真实考虑过的 alternative 和未采用原因，不编造陪衬；
- **Acceptance criteria:** 可观察 end state、调用面、状态和文档变化；
- **Risks and consequences:** behavior/API change、失去的能力、未来重新引入条件和为什么 trade-off 仍成立；
- **Evidence:** 精确源码路径、symbol、搜索结果、文档、dependency source 和实际验证。

## Validation and report

审计结束时报告：

- inspected scope 和刻意排除范围；
- retained、rejected、downgraded 与推荐候选；
- 每个推荐候选的 production/non-production consumer 证据；
- 建议的 Spec/Subspec 路由；
- 实际运行的 read-only checks。

实施获批后，验证范围由目标 repo instructions 和 accepted Spec 决定。运行 formatter、type/lint、已有最窄 behavior checks 和 `git diff --check`；没有实际运行的步骤不能声称通过。
