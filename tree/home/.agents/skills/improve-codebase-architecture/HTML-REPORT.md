# HTML Report Format

架构 review 渲染为 OS temp directory 中的单个 self-contained HTML file。Tailwind 和 Mermaid 都来自 CDN。Mermaid 擅长可靠处理 graph-shaped diagrams；手工 div 和 inline SVG 负责更具 editorial 感的 visual（mass diagrams、cross-sections）。混合使用两者，不要所有内容都依赖 Mermaid，否则视觉会变得千篇一律。

## Scaffold

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Architecture review — {{repo name}}</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({ startOnLoad: true, theme: "neutral", securityLevel: "loose" });
    </script>
    <style>
      /* small custom layer for things Tailwind doesn't cover cleanly:
         dashed seam lines, hand-drawn-feeling arrow heads, etc. */
      .seam { stroke-dasharray: 4 4; }
      .leak { stroke: #dc2626; }
      .deep { background: linear-gradient(135deg, #0f172a, #1e293b); }
    </style>
  </head>
  <body class="bg-stone-50 text-slate-900 font-sans">
    <main class="max-w-5xl mx-auto px-6 py-12 space-y-12">
      <header>...</header>
      <section id="candidates" class="space-y-10">...</section>
      <section id="top-recommendation">...</section>
    </main>
  </body>
</html>
```

## Header

Repo name、日期，以及紧凑的 legend：实线框 = module，虚线 = seam，红色箭头 = leakage，粗深色框 = deep module。不要 introduction paragraph，直接进入候选项。

## Candidate card

diagram 承担主要表达任务。Prose 要简短、直白，并直接使用 `/codebase-design` skill 的 glossary terms。

每个候选项对应一个 `<article>`：

- **Title**：简短，命名 deepening（例如 “Collapse the Order intake pipeline”）。
- **Badge row**：recommendation strength（`Strong` = emerald，`Worth exploring` = amber，`Speculative` = slate），加上 dependency category tag（`in-process`、`local-substitutable`、`ports & adapters`、`mock`）。
- **Files**：使用等宽字体的列表，`font-mono text-sm`。
- **Before / After diagram**：核心内容。两列并排。见下面的 patterns。
- **Problem**：一句话，说明痛点。
- **Solution**：一句话，说明改变。
- **Wins**：每条不超过 6 个词。例如 “Tests hit one interface”、“Pricing logic stops leaking”、“Delete 4 shallow wrappers”。
- **ADR callout**（如适用）：在琥珀色背景框中写一行。

不要写解释段落。如果 diagram 需要段落才能被理解，就重新绘制它。

## Diagram patterns

选择适合候选项的 pattern。混合使用。不要让每个 diagram 看起来都一样，变化本身就是目的的一部分。

### Mermaid graph（dependencies / call flow 的主力）

当重点是 “X calls Y calls Z，看看这团乱麻” 时，使用 Mermaid `flowchart` 或 `graph`。将它放在 Tailwind-styled card 中，避免显得突兀。使用 classDef 将 leakage edges 设为红色、deep module 设为深色。Sequence diagrams 很适合表达 “before: 6 round-trips; after: 1”。

```html
<div class="rounded-lg border border-slate-200 bg-white p-4">
  <pre class="mermaid">
    flowchart LR
      A[OrderHandler] --> B[OrderValidator]
      B --> C[OrderRepo]
      C -.leak.-> D[PricingClient]
      classDef leak stroke:#dc2626,stroke-width:2px;
      class C,D leak
  </pre>
</div>
```

### Hand-built boxes-and-arrows（Mermaid layout 不合适时）

使用带 border 和 label 的 `<div>` 表示 modules。使用 inline SVG 的 `<line>` 或 `<path>` 元素表示箭头，并在 relative container 上绝对定位。当你希望 “after” diagram 看起来像一个带粗边框、内部内容淡化的 deep module 时使用它，Mermaid 无法呈现正确的视觉重量。

### Cross-section（适合分层 shallow 结构）

堆叠横向 band（`h-12 border-l-4`），展示一次调用经过的 layers。Before：6 个各自无所作为的薄 layer。After：1 个标注整合后职责的厚 band。

### Mass diagram（适合 “interface 和 implementation 一样宽”）

每个 module 使用两个 rectangle：一个表示 interface surface area，一个表示 implementation。Before：interface rectangle 几乎和 implementation rectangle 一样高（shallow）。After：interface rectangle 很短，implementation rectangle 很高（deep）。

### Call-graph collapse

Before：将 function calls 的 tree 渲染为嵌套 boxes。After：将同一棵 tree collapse 为一个 box，并将现在属于内部的 calls 淡化显示在其中。

## Style guidance

- 保持 editorial 风格，不要像 corporate dashboard。留出充足 whitespace。heading 可以选择 serif（`font-serif` 与 stone/slate 搭配良好）。
- 克制使用颜色：一个 accent（emerald 或 indigo），加上用于 leakage 的 red 和用于 warning 的 amber。
- diagram 高度保持约 320px，让 before/after 可以舒适地并排显示而无需滚动。
- diagram 内的 module labels 使用 `text-xs uppercase tracking-wider`，它们应该像 schematic，而不是 UI。
- 唯一的 scripts 是 Tailwind CDN 和 Mermaid ESM import。report 其余部分保持 static，不包含 app code，除了 Mermaid 自身渲染外不添加 interactivity。

## Top recommendation section

一张更大的 card。候选项名称、一句话说明原因，以及指向其 card 的 anchor link。仅此而已。

## Tone

使用 plain English，保持简洁，但架构名词和动词必须直接来自 `/codebase-design` skill。简洁不是偏离术语的借口。

**必须准确使用：** module、interface、implementation、depth、deep、shallow、seam、adapter、leverage、locality。

**绝不替换为：** component、service、unit（指 module 时）；API、signature（指 interface 时）；boundary（指 seam 时）；layer、wrapper（指 module 时）。

**Phrasings that fit the style:**

- “Order intake module is shallow — interface nearly matches the implementation.”
- “Pricing leaks across the seam.”
- “Deepen: one interface, one place to test.”
- “Two adapters justify the seam: HTTP in prod, in-memory in tests.”

**Wins bullets** 使用 glossary terms 命名收益：*“locality: bugs concentrate in one module”*、*“leverage: one interface, N call sites”*、*“interface shrinks; implementation absorbs the wrappers”*。不要写 *“easier to maintain”* 或 *“cleaner code”*，这些术语不在 glossary 中，不值得出现。

不要 hedging，不要 throat-clearing，不要写 “it's worth noting that…”。如果一句话可以成为 bullet，就把它写成 bullet。如果一个 bullet 可以删掉，就删掉。如果某个术语不在 `/codebase-design` glossary 中，先使用其中已有的术语，不要发明新术语。
