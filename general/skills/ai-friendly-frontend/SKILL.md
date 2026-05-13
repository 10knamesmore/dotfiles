---
name: ai-friendly-frontend
description: 编写或修改前端代码（React + AntD + 自研组件库 / TSX / JSX）时调用。让代码可被 microsoft/playwright-cli（`playwright-cli` 命令）直接维护、最终沉淀为 `@playwright/test` 的 `.spec.ts`。预期工作循环：写前端代码 → `playwright-cli snapshot` 看 a11y 树 + `generate-locator` 出稳定 locator → 沉淀为 spec。写完前端代码（新增或修改 .tsx/.jsx/.vue/.html/.css）后必须读取此技能：核对场景 checklist、跑一遍 `playwright-cli` 验证；snapshot 输出不干净就回去补 testid/aria/state，不要改测试代码绕过。
---

# AI Friendly 前端规范

主栈：**React + Ant Design + 自研组件库**。极少数情况遇到 Vue / 原生 HTML，规则见 `references/framework-notes.md`。

## 目标

让 agent 写的前端代码同时满足 4 类自动化消费者：

1. **`playwright-cli`（首要）**—— coding agent 写完代码后用 [microsoft/playwright-cli](https://github.com/microsoft/playwright-cli) 抓 a11y snapshot、操作、生成 locator，结束时把交互沉淀为 `@playwright/test` 的 `.spec.ts`。
2. **组件单测**（Vitest / Jest + Testing Library）—— 通过 role / label 而非实现细节查询。
3. **视觉回归**（Chromatic / Percy）—— 确定性渲染，无动画 / 字体抖动。
4. **AI Agent 操作**（Computer-Use 等）—— 语义可读、点击区域足够、状态可观察。

> `playwright-cli` 与 `npx playwright test` 不是一回事：前者是 agent 操作浏览器的轻量 CLI（按 ref 点击、抓 snapshot），后者是测试 runner。本 skill 先用前者验证，再用后者沉淀。

## 核心工作循环（必须按此顺序）

```
写 / 改前端代码
       │
       ▼
playwright-cli open <url>
playwright-cli snapshot            ← AI agent 看到的页面就是这个
       │
       ├── snapshot 干净（button "登录" / textbox "邮箱"）？
       │     ├─ 否 → 回去补 aria-label / semantic tag / Form.Item label
       │     └─ 是 → 继续
       ▼
playwright-cli fill/click/...      ← 按 ref 操作
playwright-cli snapshot            ← 异步态投射到 DOM 了吗？(aria-busy / data-state)
       │     └─ 否 → 回去补状态投射
       ▼
playwright-cli generate-locator    ← 输出 getByRole / getByLabel？
       │     └─ 退化为 css/text → 前端还缺锚点，回去补
       ▼
沉淀为 tests/<feature>.spec.ts → npx playwright test 跑
```

两个金标准信号：
1. **`snapshot` 输出干净**：关键元素都以正确的 `role` + `name` 出现，不是无名 `generic` / `text`。
2. **`generate-locator` 给出 `getByRole` / `getByLabel`**：而不是退化为 CSS / xpath / text。

任一信号通不过，**回去补前端代码**，不要在测试侧绕过。详见 `references/playwright-workflow.md`。

## 三种典型场景

写代码前先识别你在哪个场景，规则与 checklist 不同：

| 场景 | 你在做什么 | 谁负责 a11y 基础 | 你的工作重点 |
|------|----------|----------------|------------|
| **A. 用 AntD / 自研基础组件拼业务** | 业务页面、表单、列表、详情页 | AntD / 自研层（多数能 trust） | 验证 props 透传、补 testid、用对组件库的 `label` 系列 props |
| **B. 写自研基础组件** | 给全公司复用的 `<AppButton>` / `<AppTable>` / `<AppForm>` 等 | **你** | 完整契约：testid 透传 + 状态投射 + sub-component + 契约测试 |
| **C. 极少：从零写原生 HTML** | 邮件模板、第三方嵌入页、改造老页面 | **你** | 完整语义化 + ARIA + 状态属性 |

判别捷径：JSX 主要是大写标签（`<Button>`、`<Form>`、`<AppTable>`）= 场景 A；你在**实现**一个大写组件的内部 = 场景 B；全小写（`<button>`、`<dialog>`）= 场景 C。

## 通用核心原则（三场景都适用）

- **状态可观察**：loading / error / success / disabled / expanded 必须能在 DOM 上读出来。或靠组件库已注入的属性（场景 A），或自己注入（场景 B/C）。
- **锚点稳定**：测试 / agent 要操作的元素必须有 accessible name 或 `data-testid`。CSS class、`nth-child`、CSS-in-JS 哈希禁止用作测试钩子。
- **Locator 优先级**：`getByRole` > `getByLabel` > `getByText` > `getByTestId`。testid 是兜底，不是首选。如果你第一选择就是 testid，说明语义化没做好。

## 场景 A：用 AntD / 自研组件拼业务

绝大多数业务代码属于此类。AntD + 自研层已经把 `<button>` / `aria-*` / 键盘行为做好了。**你的核心任务**是用对它们的 props，并在它们不给的地方补 testid。

详细的 AntD 各组件用法 / locator 模板 / 已知坑 → `references/library-adaptation.md`。

### A 场景核心规则

1. **可访问名优先用组件库的 `label` 系列 props**：
   - AntD `<Form.Item label="邮箱">`、`<Tooltip title="...">`、`<Modal title="...">`。
   - 自研组件遵循同样的约定（看自研组件的锚点契约文档）。
   - 库会自动把 label 渲染到正确位置；自己手贴 `aria-*` 反而可能冲突。

2. **testid 补在组件库不会自动给的位置**：
   - 业务关键容器（卡片根、对话框根、列表区域根、tab panel）。
   - 同视觉重复的元素（表格每行操作按钮、菜单项）。
   - 动态列表项（`onRow={(record) => ({ 'data-testid': 'user-row', 'data-user-id': record.id })}`）。

3. **AntD `Form.Item` 不会透传 testid 到 input**，要写在子组件上：
   ```tsx
   <Form.Item name="email" label="邮箱">
     <Input data-testid="login-email-input" />
   </Form.Item>
   ```

4. **i18n + 图标按钮的标配**：所有纯图标按钮**必须**有翻译过的 `aria-label`：
   ```tsx
   <Button
     icon={<DeleteIcon />}
     aria-label={t('cart.removeItem', { name: item.name })}
     onClick={() => remove(item.id)}
   />
   ```

5. **异步态用库的 `loading` prop**，不要自己条件渲染替换文本：
   ```tsx
   // ❌
   <Button>{submitting ? '提交中…' : '提交'}</Button>

   // ✅ AntD 自动加 disabled + spinner（但不加 aria-busy，必要时自研 wrapper 补）
   <Button loading={submitting}>提交</Button>
   ```

6. **错误态用库的 `validateStatus` / `error` props**：自动加 `aria-invalid` + `aria-describedby` 指向错误消息。

### A 场景 checklist

写完一段业务代码后自查：

- [ ] 业务关键容器（卡片、对话框、列表区、tab panel）是否有 `data-testid`？
- [ ] 所有纯图标按钮是否有 `aria-label`（含 i18n）？
- [ ] 同视觉重复元素（表格每行操作按钮）是否能被独立选中（testid 或 label 含上下文）？
- [ ] 表单字段是否用了 AntD `Form.Item label`，子组件上有 testid？
- [ ] 异步操作按钮是否用了 `loading` prop，没有手动替换文本？
- [ ] 错误提示是否走 AntD `Form.Item` 校验链路（自动有 `role="alert"`）？
- [ ] 透传到组件的 `data-testid` 是否真的到了 DOM？（DevTools 验证一次）
- [ ] 是否避免了 `.ant-*` class、CSS-in-JS 哈希作 locator？

## 场景 B：写自研基础组件

收益最大的场景。一个底层 `<AppButton>` 把 testid / aria / state 做好，全公司所有调用方自动受益。

详细规则见 `references/component-contract.md`。

### B 场景核心规则（简版）

1. **三种内核形态都允许**：
   - 包 AntD（常见）：`<AppButton>` 内核是 `<AntdButton>`。
   - 基于原生 HTML：AntD 没对应件时（自研甘特图、流程编排等）用 `<button>` / `<div role="...">` 等原生元素严格按 `semantics-aria.md` 实现。
   - 混合：外层自研 + 内部部分用 AntD。
   - **禁止**用 `<div onClick>` 重写已存在的语义控件。

2. **必须透传**：`data-testid`、`aria-*`、`className`、`style`、`id`、`ref` 一律到根 DOM。单测里加契约测试守住。

3. **必须投射业务状态到 DOM**：`data-state="idle|loading|success|error|empty|disabled"` + 对应 `aria-busy` / `aria-invalid` / `aria-disabled` / `aria-expanded`。AntD 没自动加的（如 `aria-busy`）自己补。

4. **复合组件用 sub-component 暴露内层锚点**：
   ```tsx
   <AppTable data-testid="user-table">
     <AppTable.Toolbar data-testid="user-toolbar" />
     <AppTable.Filters data-testid="user-filters" />
   </AppTable>
   ```

5. **暴露锚点契约**（README / TSDoc 顶部）：testid 命名约定、状态属性、aria slot。让测试与 AI agent 按图索骥。

### B 场景 checklist

- [ ] 内核语义正确？包 AntD / 用原生标签 / 混合，都不能 `<div onClick>` 假装。
- [ ] `data-testid` / `aria-*` / `className` / `style` / `id` / `ref` 全部透传到根 DOM？
- [ ] 业务状态投射成 `data-state` + 对应 `aria-*`？
- [ ] 复合组件拆成了 sub-component？
- [ ] Props 类型显式（TS interface），命名遵循 `isX`/`onX`/具体业务名词？
- [ ] 副作用抽到 hook，UI 组件纯展示？
- [ ] README / TSDoc 写了锚点契约？
- [ ] 单测覆盖了透传契约 + 状态投射？
- [ ] Storybook 覆盖所有状态（loading / error / empty / disabled / success）？

## 场景 C：极少数情况

邮件模板、第三方嵌入页、Web Component 等。规则最严，没有库兜底。完整规则见 `references/semantics-aria.md`。最少做到：

- 交互元素用原生语义标签（`<button>`、`<a href>`、`<input>`、`<select>`）。
- 所有按钮 / 输入框 / 链接有 accessible name。
- 异步态投射到 DOM（`aria-busy="true"`、`role="alert"`）。
- 表单字段绑 `<label htmlFor>`，错误态 `aria-invalid` + `aria-describedby`。
- 模态有 `role="dialog"` + `aria-modal="true"` + focus trap。
- 图片有 `alt`（装饰图 `alt=""` 且 `aria-hidden="true"`）。

## Locator 优先级（所有场景）

| 优先级 | Playwright | Testing Library | 用例 |
|--------|-----------|----------------|------|
| 1 | `page.getByRole(role, { name })` | `getByRole(role, { name })` | 默认 |
| 2 | `page.getByLabel` | `getByLabelText` | 表单 |
| 3 | `page.getByText` | `getByText` | 唯一可见文本 |
| 4 | `page.getByTestId` | `getByTestId` | 兜底：动态列表、纯图标、i18n 变动 |

## Reference 导航

| 文件 | 加载时机 |
|------|--------|
| `references/playwright-workflow.md` | **每次都要看**：`playwright-cli` 完整命令、snapshot/ref/generate-locator 三件套、沉淀为 `.spec.ts` 的步骤、修挂测试的流程 |
| `references/library-adaptation.md` | 场景 A：AntD 各核心组件（Form / Table / Select / Modal / Notification / Upload 等）用法与 locator |
| `references/component-contract.md` | 场景 B：自研基础组件契约（含包装 AntD、基于原生 HTML、混合三种形态） |
| `references/semantics-aria.md` | 场景 C 与 B 形态二参考：原生 HTML 语义与 ARIA 详规则 |
| `references/testing-anchors.md` | 三场景共用：`data-testid` 命名、状态属性投射、视觉回归与 Agent 专项 |
| `references/framework-notes.md` | React 通用陷阱（key、useEffect、Portal、RSC）+ 跨组件库常见陷阱 + 极少数 Vue/原生情况 |

## 触发约定

本 skill 有两个触发路径：

1. **显式调用**：用户提出「写前端」「写组件」「优化 a11y」「让 playwright-cli 能操作」「修复测试」等需求时主动读取本 skill。
2. **写完代码自动核对**：完成对 `.tsx` / `.jsx` / `.vue` / `.html` / `.css` 文件的新增或修改后，自动：
   - 判断场景（A/B/C），跑对应 checklist；
   - 按 `playwright-workflow.md` 跑 `playwright-cli` 三件套：`open` → `snapshot`（看 a11y 树干不干净）→ 操作 → `generate-locator`（看输出是否 `getByRole`/`getByLabel`）；
   - 任一信号通不过，**回去改前端代码**（补 aria-label / 用语义标签 / 投射状态属性），**不要**在测试侧绕过；
   - 最终把通过的交互沉淀为 `tests/<feature>.spec.ts`，用 `npx playwright test` 跑通。

## 不做什么

- 不替代 `gen-test-plan`：本 skill 让被测代码可测，不写测试计划文档。
- 不替代具体 a11y 审计工具（axe / Lighthouse）：本 skill 只规范产出时的代码形态。
- 不强求 TypeScript，但项目已用 TS，组件 props 必须有显式类型。
- 不要求所有组件都走场景 B 的完整契约：业务页面用场景 A 足够，过度抽象反而是负担。
- 不限定自研组件必须包 AntD：包 AntD 是默认推荐，但 AntD 没对应件时按场景 B 的形态二（基于原生 HTML）实现。
