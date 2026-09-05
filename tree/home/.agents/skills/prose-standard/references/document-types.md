# 按文档用途保留信息

这不是单向缩短流程。代码、类型和结构没有表达所需事实时，应补充或恢复 prose。

- **Public API docs:** 返回值区别、错误、side effect、ownership、timing、cancellation、durability 和调用前提。
- **Internal comments:** 非局部结构、业务不变量、race ordering、ownership、安全边界和意外失败行为；删除控制流 narration。
- **Module comments:** 模块在系统中的角色、依赖、责任边界和非显然架构选择；不要罗列内部符号。
- **Tests:** 不写没有意义的测试(只是代码逻辑的另一种表达, assert 文案), 只关注关键逻辑
- **README and cookbook:** prerequisite、真实入口、配置语义、失败、限制、extension point、可观察验证和简洁 warning。
- **Spec and Subspec:** destination、当前 contract、真实 decision、acceptance criteria、resolution 和 evidence。活动 proposal 可以使用未来时；完成状态改写成实际结果，不保留草稿轮次和编写过程。
- **ADR and postmortem:** 保留真实 alternatives、trade-off、incident sequence、证据、causal chain、impact 和 prevention；不虚构 alternative，也不重复 persuasion。
- **Skill and agent instruction:** 写清 trigger、scope、authority、guardrail 与必要 workflow，明确 guidance 与机械脚本的边界。
- **Prompt and visible string:** wording 视为行为，确认 owner、消费者和既有行为验证。
- **Diagnostic:** 指明失败对象或路径、违反的规则，以及不显然时的修复动作；删除内部执行 narration。
- **Configuration and example:** 解释 access limit、load order、security stance、replay behavior、exception 和常见误用；让配置本身展示自明 inventory。

保留可搜索的 mechanism name，以及有实际语义的 modal、temporal 和 negative emphasis。

