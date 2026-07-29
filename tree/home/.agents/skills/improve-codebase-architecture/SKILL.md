---
name: improve-codebase-architecture
description: 扫描代码库中的 deepening opportunities，以可视化 HTML report 呈现，再把选中的机会整理为 Spec 与 Subspec 并完成 grilling。
---

# Improve Codebase Architecture

发现架构摩擦并提出 **deepening opportunities**：将 shallow modules 转变为 deep modules 的 refactor。目标是提升可测试性和 AI navigability。

这个 command 以项目的 domain model 为依据，并建立在共享的设计词汇之上：

- 运行 `/codebase-design` skill，获取架构词汇（**module**、**interface**、**depth**、**seam**、**adapter**、**leverage**、**locality）及其原则（deletion test、“interface 是 test surface”、“one adapter = hypothetical seam, two = real”）。每条建议都必须准确使用这些术语，不要滑向 “component”、“service”、“API” 或 “boundary”。
- `CONTEXT.md` 中的 domain language 会为良好的 seam 命名；`docs/adr/` 中的 ADR 记录本 command 不应重新争论的决策。

## Process

### 1. Explore

**先确定范围，再扫描——YAGNI。**Deepening module 的价值在于让未来修改更容易，因此应重点关注近期有变化的代码库区域。在查看之前先决定 *where*：

- 如果用户指定了方向，例如 module、subsystem 或 pain point，就以此为准，跳过下面的推断。
- 否则回看一段 commit history（`git log --oneline`），寻找代码库的 hot spots，即反复出现的文件和区域，并优先关注这些路径。如果改动分散、没有明确 hot spot，就扩大范围。

首先阅读项目的 domain glossary（`CONTEXT.md`）以及目标区域的 ADR。

然后使用 `subagent_type=Explore` 的 Agent tool 遍历代码库。不要遵循僵化的启发式规则，要自然 explore，并记录你感受到摩擦的位置：

- 理解一个概念是否必须在许多小 module 之间来回跳转？
- 哪些 module **shallow**，interface 几乎和 implementation 一样复杂？
- 哪些 pure functions 只是为了可测试性被抽出，但真正的 bug 藏在调用方式中（缺少 **locality**）？
- 哪些紧耦合 module 会跨 seam 泄漏？
- 代码库哪些部分没有测试，或难以通过当前 interface 测试？

对所有疑似 shallow 的对象应用 **deletion test**：删除它会集中复杂度，还是只是把复杂度挪走？“会集中”就是你要找的信号。

### 2. 以 HTML report 呈现候选项

将 self-contained HTML file 写入 OS temp directory，避免任何文件进入 repo。优先从 `$TMPDIR` 解析 temp dir，回退到 `/tmp`（Windows 使用 `%TEMP%`），写入 `<tmpdir>/architecture-review-<timestamp>.html`，确保每次运行都有新文件。为用户打开它：Linux 使用 `xdg-open <path>`，macOS 使用 `open <path>`，Windows 使用 `start <path>`，并告知绝对路径。

report 使用 **Tailwind via CDN** 负责布局和样式，使用 **Mermaid via CDN** 绘制适合表达 graph/flow/sequence 结构的 diagram。将 Mermaid 与手工制作的 CSS/SVG visual 混用：关系呈 graph 形态时使用 Mermaid（call graphs、dependencies、sequences），需要更具 editorial 感的效果时使用手工 div/SVG（mass diagrams、cross-sections、collapse animations）。每个候选项都要有 **before/after visualisation**。要有视觉表现。

为每个候选项渲染一张 card，包含：

- **Files**：涉及哪些 files/modules
- **Problem**：当前架构为什么造成摩擦
- **Solution**：用 plain English 描述会发生什么变化
- **Benefits**：从 locality 和 leverage 以及测试如何改善的角度解释
- **Before / After diagram**：并排、自定义绘制，展示 shallow 状态和 deepening 之后的状态
- **Recommendation strength**：`Strong`、`Worth exploring`、`Speculative` 三者之一，以 badge 渲染

report 以 **Top recommendation** section 结尾：说明最先处理哪个候选项，以及原因。

**domain 使用 CONTEXT.md vocabulary，架构使用 `/codebase-design` vocabulary。**如果 `CONTEXT.md` 定义了 “Order”，就说 “the Order intake module”，不要说 “the FooBarHandler”，也不要说 “the Order service”。

**ADR conflicts**：如果候选项与现有 ADR 冲突，只有当摩擦确实严重到值得重新审视 ADR 时才提出。要在 card 中清楚标记，例如 warning callout：_“contradicts ADR-0007 — but worth reopening because…”_。不要列出 ADR 禁止的每一种理论 refactor。

See [HTML-REPORT.md](HTML-REPORT.md) for the full HTML scaffold, diagram patterns, and styling guidance.

暂时不要提出 interfaces。文件写完后，询问用户：“Which of these would you like to explore?”

### 3. Grilling loop

用户选定候选项后，先找到与该 architecture effort 对应的 Spec；不存在时按 `../wayfinder/references/SPEC-FORMAT.md` 创建。为候选项创建一个 `kind: decision` 的 Subspec，并把 HTML report path 作为 evidence。不要创建 issue。

claim 该 Subspec 后运行 `/grilling` skill，与用户一起走过决策树：约束、依赖、deepened module 的形态、seam 后面是什么，以及哪些测试能够保留。Subspec 格式见 `../wayfinder/references/SUBSPEC-FORMAT.md`。

随着决策明确，inline 处理副作用；运行 `/domain-modeling` skill，持续保持 domain model 最新：

- **将 deepened module 命名为 `CONTEXT.md` 中没有的概念？**将该术语加入 `CONTEXT.md`。文件不存在时延迟创建。
- **对话中打磨了模糊术语？**立即更新 `CONTEXT.md`。
- **用户因为 load-bearing reason 拒绝候选项？**可以这样提出 ADR：_“要不要把它记录成 ADR，避免未来的架构 review 再次提出它？”_ 只有未来 explorer 确实需要这个理由来避免重复建议时才提出，跳过临时理由（“现在不值得做”）和不言自明的理由。
- **想探索 deepened module 的替代 interfaces？**运行 `/codebase-design` skill，使用它的 design-it-twice parallel sub-agent pattern。

决策稳定后，将 reasoning 与 report link 写入 Subspec 的 `Resolution` 和 `Evidence`，标记为 `resolved`，并在 Spec 中写一行 decision summary。只有 contract 与 acceptance criteria 已稳定时，才创建后续 `kind: implementation` Subspec；实际实现交给 `implement` skill。
