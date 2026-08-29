---
name: prose-standard
description: 编写、审查、补全、裁剪或重构仓库内持久 prose 时使用，包括 Markdown、代码注释、文档注释、Spec、ADR、prompt、diagnostic、CLI/UI 文本和 agent instruction；编写代码后也使用，检查新增或修改的代码是否需要同步注释。要求保留完整 contract，同时删除推理过程、重复和装饰性文字。
---

# Prose Standard

写到足以保存 contract，然后删除推理过程、重复和装饰。这里的 contract 是调用方、实现方、生产者或消费者会依赖的义务、不变量、前置条件、后置条件、失败行为或兼容承诺。

本 Skill 负责仓库内持久 prose 的编辑判断、代码注释语义和各语言的注释表达，不约束日常对话，也不承担专门的泄漏召回流程。使用 [`examples`](./references/examples.md) 校准完整命题和 surface 边界；发现 session、PR、review 或草稿视角，以及过期快照和落不到符号的指针时使用 [`trim-cot-leakage`](../trim-cot-leakage/SKILL.md)。本 Skill 是 guidance，不是关键词脚本或删字清单。

`contract`、`boundary`、`shape`、`surface`、`seam`、`canonical` 和 `provenance` 不是禁词，但不能代替具体事实。优先写清实际规则、字段、时机、失败状态和后果。

## Scope and authority

- 使用用户指定的文件、目录或 diff 作为 scope。未明确指定但当前 diff 已形成自然边界时，只检查当前 diff；不要自行扩成全仓库审计。
- review 和 audit 只报告发现；用户明确要求 write、fix、trim 或 implement 时才修改。
- 第三方源码、只读 vendored 内容、fixture、snapshot、录制输出和冻结历史保留原始声音。生成物先定位 owner，修改 owner 后再按仓库流程生成；不要直接修补派生文件。
- prompt、diagnostic 和 CLI/UI 文本的 wording 可能是行为。改动前先确认 owner 和现有验证方式，不能把语气整理伪装成无行为影响的文档修改。

## Preserve the complete proposition

编辑前枚举 passage 中的每个命题，保留所有相关内容：

- actor 与 action；
- condition、timing 与 ordering；
- `must`、`may`、`never` 等 modality；
- negative guarantee、exception 与有效范围；
- ownership、side effect、failure mode 与 consequence。

只有在所有事实仍然成立且表达更清楚时，才能删除形容词、重复和 narration。字数更少本身不是改进。

使用处应保留调用者或维护者当场需要的完整局部 contract；架构、算法、历史、长篇 rationale 和扩展示例链接到 owning document。一个解释只有一个 owner，但必要的局部行为可以重复。

省略 rationale 可能导致误用或错误简化时，保留最小但具体的原因和后果。代码或类型已经显然表达的事实不再用 prose 复述。

## Coverage by location

这不是单向缩短流程。代码、类型和结构没有表达所需事实时，应补充或恢复 prose。

- **Public API docs:** 返回值区别、错误、side effect、ownership、timing、cancellation、durability 和调用前提。
- **Internal comments:** 非局部结构、业务不变量、race ordering、ownership、安全边界和意外失败行为；删除控制流 narration。
- **Module comments:** 模块在系统中的角色、依赖、责任边界和非显然架构选择；不要罗列内部符号。
- **Tests:** 仅说明 fixture、assertion、平台处理、真实入口或间接观测为何必要；删除测试步骤 walkthrough。未经用户明确要求，不新增或修改测试。
- **README and cookbook:** prerequisite、真实入口、配置语义、失败、限制、extension point、可观察验证和简洁 warning。
- **Spec and Subspec:** destination、当前 contract、真实 decision、acceptance criteria、resolution 和 evidence。活动 proposal 可以使用未来时；完成状态改写成实际结果，不保留草稿轮次和编写过程。
- **ADR and postmortem:** 保留真实 alternatives、trade-off、incident sequence、证据、causal chain、impact 和 prevention；不虚构 alternative，也不重复 persuasion。
- **Skill and agent instruction:** 写清 trigger、scope、authority、guardrail 与必要 workflow，明确 guidance 与机械脚本的边界。
- **Prompt and visible string:** wording 视为行为，确认 owner、消费者和既有行为验证。
- **Diagnostic:** 指明失败对象或路径、违反的规则，以及不显然时的修复动作；删除内部执行 narration。
- **Configuration and example:** 解释 access limit、load order、security stance、replay behavior、exception 和常见误用；让配置本身展示自明 inventory。

保留可搜索的 mechanism name，以及有实际语义的 modal、temporal 和 negative emphasis。

## Code comments

编写、修改、补全或审查代码注释时，以只拥有当前代码和公共领域知识的维护者为读者。既处理 API 文档注释，也处理解释非显然约束的普通注释。完成代码编写后，检查 touched code 是否引入新的业务概念、字段语义、失败条件、不变量或非显然实现；需要时补齐注释，不为自明代码制造噪声。

按目标文件选择并只读取命中的语言规范；涉及多门语言时逐份按需读取：

| 语言 | 扩展名 | 规范文件 |
| --- | --- | --- |
| Rust | `*.rs` | [`rust.md`](./references/code-comments/rust.md) |
| TypeScript | `*.ts` `*.tsx` | [`typescript.md`](./references/code-comments/typescript.md) |
| JavaScript（纯 JS，JSDoc 承担类型） | `*.js` `*.mjs` `*.cjs` | [`javascript.md`](./references/code-comments/javascript.md) |
| Python | `*.py` | [`python.md`](./references/code-comments/python.md) |
| Lua | `*.lua` | [`lua.md`](./references/code-comments/lua.md) |
| QML | `*.qml` | [`qml.md`](./references/code-comments/qml.md) |
| Shell（bash/zsh） | `*.sh` `*.zsh`、无扩展名脚本（看 shebang） | [`shell.md`](./references/code-comments/shell.md) |

表外语言先遵循目标仓库和语言自身的文档惯例；可以参考最接近的规范组织内容，但不要照搬不适用的标签。

注释必须提供代码签名和类型系统没有表达的信息，只写与当前对象相关的部分，不为凑模板堆砌段落：

- 类型、类、状态对象：说明它代表的业务概念、为何存在、由谁在何时构造、数据从哪里来及生命周期、保留或组合了哪些信息、参与什么判断、保证什么不变量，以及缺失、冲突或能力不足时如何处理。
- 字段、属性、枚举成员：说明它表达的业务事实、来源、单位、范围、默认值或有效条件，以及它与其他状态、权限或字段的关系和对最终行为的影响。
- 函数、方法：说明调用产生的业务结果；按需补充输入语义、前置条件、返回值、错误与降级、外部副作用、幂等性、并发和取消行为。
- 模块、文件：说明模块提供的业务能力、责任边界，以及明确不负责什么；不要罗列内部符号。
- 非显然实现：解释为什么需要该分支、顺序、锁、缓存或兼容处理，以及误改会导致什么；不要逐句翻译代码。

`typed projection`、`authorization snapshot`、`canonical state`、`用于 provenance`、`host-owned catalog` 等实现标签不能单独构成说明。若确实需要这些术语，必须紧接具体业务含义、来源、用途和失败行为。删掉类型名、字段名和 `typed`、`canonical`、`snapshot`、`registry`、`profile` 等实现词后，注释仍应能说明真实业务行为。

写注释前阅读定义、调用方、构造方、持久化边界、错误路径和相关测试，确认真实行为；无法从源码证实的内容不编造。找出缺失、空洞、过期、泄漏编写过程或与行为冲突的注释；正确注释保留，错误注释直接修正，不叠加补丁式说明。复核所有新增或修改的类型、字段、函数、错误分支和关键约束，确保注释与当前代码一致。

只修改需要同步的注释，避免无关格式化和实现改动。注释必须自包含，不依赖 task 编号、未提交 Spec 章节、方案代号、PR/review 轮次、先前实现或只有编写者知道的上下文。公共 API、领域类型、权限与状态字段、持久化数据、错误与降级路径、并发和安全不变量优先完整说明。

## Current-state perspective

README、代码注释、API docs、Skill 和普通设计文档从当前 checkout 的视角陈述事实。不要使用只有编写 session、未提交草稿、review round 或 PR stack 才能解析的引用。

历史不是一律删除：ADR、postmortem、changelog、已完成 Spec 的 evidence 等明确承担历史或决策记录的 surface，可以保留可解析的时间线、alternatives 和来源。历史材料仍必须让只拥有仓库与公共资料的读者验证。

## Workflow

1. 确认 scope、write authority、目标仓库 instructions，以及 prose 的 owner 和消费者。
2. 阅读 owning code、document、schema 或行为入口；不能从源码、文档或可验证资料证实的内容不编造。
3. 完整检查 scope，不只处理关键词命中。将候选分类为 keep、add、trim、restore、restructure 或 defer。
4. 先改 owner，再处理派生 artifact。学到新规则后复查 scope 内的 analogous passages。
5. 逐段检查所有命题是否保留，引用是否可解析，完成态 prose 是否站在当前状态。
6. 运行目标 surface 已有的最窄验证和 `git diff --check`；只报告实际执行的检查。
7. 说明检查范围、明确修改、刻意保留、仍待决定的 borderline case 和验证结果。

## Borderline decisions

只有至少两个版本都保留完整命题、但在已接受原则之间存在真实 trade-off 时，case 才是 borderline。存在唯一 proposition-preserving 改法时直接采用；不要为了提供选项制造较差版本。

用户要求自动处理时，修改明确项并报告真正的 borderline case。用户要求 calibration 时，按同一原则分组，提供两到三个可行版本，先推荐一项，并说明事实或结构差异。
