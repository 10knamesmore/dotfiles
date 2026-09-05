# 泄漏类型与保留边界

## Taxonomy

1. **Dead design-session citation:** `(decision 7)`、`audit C2`、`design §4.7`、`task T4`、`B 方案`。存在 committed owner 时改成可解析的名称和路径；不存在时删除 citation，但保留其中事实。
2. **Stack and PR vantage:** `this PR adds`、`later PR in this stack`、`previous commit`。改写成 shipped mechanism 或 extension point；尚未完成的工作放入有 owner 的 issue、TODO 或活动 Spec。
3. **Change narration and version stamp:** `used to`、`no longer`、`the old implementation`、`v1`、`this cut`、`today`、对比过去状态的 `now`。普通 current-state surface 直接写现有行为；regression pin 改成 present-tense counterfactual，例如 `Without X, Y happens`。
4. **Review choreography:** `rejected in review`、`the reviewer confirmed`、`round 3`、`v5 of this note`。ADR 或 decision record 可保留真实 alternative 与 rationale，但不保留谁在哪一轮说了什么。
5. **Reviewer-addressed justification:** `the cast is safe`、`this is correct because`。写明使其安全的 invariant 和误改后果；代码已经显然表达时删除。
6. **Restatement and derivation transcript:** `first X, then Y`、测试 walkthrough、显然分支的 proof。删除；只保留非显然 contract、assertion rationale 或 invariant。
7. **Hedge and planning residue:** `probably fine for now`、`should be enough`、没有 owner 的 deferral。改成可验证 bound 和 failure behavior，或交给明确的 issue、TODO、Spec；不要保留 hedge。
8. **Authoring-language slip:** 英文 prose 中混入未翻译工作语言、私有分隔符和 session shorthand，或中文 prose 中出现相反情况。翻译为目标 surface 的语言，或删除无事实内容。
9. **Stale snapshot:** 把比页面变化更快的值写进长寿命文档：`production currently runs 2.3.1`、deployed commit hash、`currently live`、`not yet shipped`、手工同步的 object 或 migration 计数。修复方向是记录去哪里读当前答案（命令、console、符号），而不是答案本身；页面必须展示当前值时，从权威源构建期生成，使静默漂移不可能。版本与部署事实的合法归宿是 changelog、release ticket 和 git history——携带日期正是那些 surface 的职责。

## Pointer resolution

指向代码事实的指针必须落到一个可搜索符号：具体文件加函数、常量、数据键或标题。`see the source`、repo 根链接和裸目录让读者重新推导句子承诺的内容，不算解析。目录只在两种情况下是合法目标：事实由有序集合涌现（migrations 按序 replay），或目录是 loader 枚举的 catalog（locales、plugins、maps）；两种仍须给出可搜索的选择键——命名约定、loader 函数或对象名。没有单一符号拥有该事实时，给出最小的相关符号集合与关系；若需要代码重构才能改善归属，另行报告，不把 prose 审计扩成实现修改。

## What is not leakage

- issue、TODO、FIXME 或其他在当前仓库流程中能解析的 follow-up owner；
- ADR、postmortem、changelog 和 completed Spec evidence 中可验证的历史与 trade-off；
- suppression、coverage ignore、empty catch 等工具要求的理由；理由错误时修正，不能直接删除；
- `Without X, Y happens` 等 counterfactual-present regression pin；
- 带来源的 measured bound；`measured`、benchmark path 和测量条件不能丢；
- runtime lifecycle 中的 old/new object，例如旧连接 drain 后新连接接管；
- RFC section、标准文档、committed design doc、Figma frame 等按设计在仓库外或仓库内可解析的引用；
- proposal 的 alternatives、future tense 和未完成 acceptance criteria；
- recorded model output、fixture、snapshot 和冻结历史中的原始声音；
- changelog、release ticket 和 git history 中承担日期意义的版本与部署记录；
- 构建期从权威源生成的当前值表格；被禁止的是手工维护的第二份拷贝，不是表格本身。

使用 [`examples`](./examples.md) 校准这些边界，尤其检查 proposition 被误删、modality 翻转和假设被升级成事实的情况。

