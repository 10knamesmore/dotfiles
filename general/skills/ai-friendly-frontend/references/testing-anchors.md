# 测试锚点：data-testid、状态属性、选择器优先级

本文档定义**哪些元素需要被自动化系统稳定识别**，以及如何标记它们。

## 总原则：能用语义就不用 testid

`data-testid` 是兜底，不是首选。任何已经能被以下查询稳定命中的元素，**不要**加 testid：

1. `getByRole('button', { name: '保存' })`
2. `getByLabelText('邮箱')`
3. `getByText('找回密码')`（仅当文本是稳定锚点，不会因 i18n 变动时）

testid 的成本：增加 DOM 噪音、与设计强耦合、容易过期。所以语义化先做到位，再考虑 testid。

## 什么时候必须加 `data-testid`

满足以下任一条件就加：

| 场景 | 例子 |
|------|------|
| 同语义元素重复出现且无法用文本区分 | 列表中每行的删除按钮 |
| 纯图标控件且 i18n 多语言 | 工具栏的"分享"按钮 |
| 容器/区域作为后续查询起点 | `data-testid="cart-summary"` 包住一整块再 `within()` 查询 |
| 动态生成的可重复结构的根 | `data-testid="todo-item"` 每个 todo 卡片根 |
| 与业务无关的状态指示器 | 加载骨架根 `data-testid="user-list-skeleton"` |
| 文案频繁变动 | 营销 banner、A/B 实验文案 |

## `data-testid` 命名规范

格式：`<feature>-<element>-<role>`，全小写 kebab-case。

| 反例 | 正解 | 原因 |
|------|------|------|
| `data-testid="btn1"` | `data-testid="cart-checkout-button"` | 索引号不稳定 |
| `data-testid="Button"` | `data-testid="login-submit-button"` | 大小写、无业务上下文 |
| `data-testid="user-list-item-3"` | `data-testid="user-list-item"` + 内部 `data-user-id={id}` | 不要把数据写进 testid，用 data-* 单独标 |
| `data-testid="div"` | 删掉，加 role/label 或具体语义名 | 元素类型不是好名字 |

**列表项**模板：

```html
<ul data-testid="todo-list">
  <li data-testid="todo-item" data-todo-id="42">
    <input type="checkbox" aria-label="完成「买菜」" />
    <span>买菜</span>
    <button aria-label="删除「买菜」">×</button>
  </li>
</ul>
```

测试侧：`within(getByTestId('todo-list')).getAllByTestId('todo-item')`，再用 `data-todo-id` 或 `aria-label` 定位具体项。

## 状态投射：把内部 state 映射到 DOM

所有自动化系统都只能看到 DOM。React state、Vue ref、Redux store 它们**看不见**。任何需要被等待、被断言的状态，必须有对应 DOM 属性。

### 标准属性表

| 状态 | 标准 DOM 表现 | 何时用 |
|------|--------------|--------|
| 异步加载中 | `aria-busy="true"` 加在容器上 | 任何 fetch / 提交期间 |
| 通用四态机 | `data-state="idle\|loading\|success\|error"` | 数据请求、表单提交、按钮等 |
| 表单字段无效 | `aria-invalid="true"` + `aria-describedby` 指错误 | 校验失败 |
| 字段必填 | `required` 属性 / `aria-required="true"` | 表单 |
| 字段禁用 | `disabled` 属性（原生）/ `aria-disabled="true"`（自定义） | 表单与按钮 |
| 折叠/展开 | `aria-expanded="true\|false"` | accordion、dropdown trigger |
| 选中态 | `aria-selected="true\|false"` | tab、option |
| 当前页/项 | `aria-current="page\|step\|true"` | 导航、面包屑、stepper |
| 按下态（toggle） | `aria-pressed="true\|false"` | 收藏、订阅切换按钮 |
| 可见性变化 | 增减 DOM 节点优于 `display:none`，但模态用 `inert` | dialog、popover |

### 状态投射的标准模式

```jsx
// React 示例
function SubmitButton({ status, onClick }) {
  return (
    <button
      type="submit"
      data-testid="login-submit-button"
      data-state={status}
      aria-busy={status === 'loading'}
      disabled={status === 'loading'}
      onClick={onClick}
    >
      {status === 'loading' ? '登录中…' : '登录'}
    </button>
  );
}
```

测试侧（Playwright）：

```ts
const btn = page.getByRole('button', { name: /登录/ });
await btn.click();
await expect(btn).toHaveAttribute('data-state', 'loading');
// 或：
await expect(btn).toHaveAttribute('aria-busy', 'true');
await page.waitForFunction(
  () => document.querySelector('[data-testid="login-submit-button"]')
    ?.getAttribute('data-state') === 'success'
);
```

## 选择器优先级（必须遵守）

| 优先级 | Testing Library | Playwright | 用例 |
|--------|----------------|-----------|------|
| 1 | `getByRole(role, { name })` | `page.getByRole(role, { name })` | 默认 |
| 2 | `getByLabelText(text)` | `page.getByLabel(text)` | 表单 |
| 3 | `getByPlaceholderText` / `getByText` | `page.getByPlaceholder` / `page.getByText` | 唯一可见文本 |
| 4 | `getByAltText` | `page.getByAltText` | 图片 |
| 5 | `getByTitle` | `page.getByTitle` | 兜底，几乎不用 |
| 6 | `getByTestId('id')` | `page.getByTestId('id')` | 最后兜底 |

**禁止**：

- `container.querySelector('.btn-primary')` —— CSS class 不是契约。
- `cy.get('button:nth-child(3)')` —— DOM 顺序不稳定。
- `cy.get('[class*="hash_abc123"]')` —— CSS-in-JS 哈希每次构建可能变。
- `await page.locator('//div[2]/button[1]')` —— XPath 索引脆弱。

## 视觉回归专项

视觉回归测试（Chromatic / Percy / Loki）要求**像素级确定性**。代码侧要做到：

### 禁用动画

在测试 / 视觉回归构建里加全局 CSS：

```css
*, *::before, *::after {
  animation-duration: 0s !important;
  animation-delay: 0s !important;
  transition-duration: 0s !important;
  transition-delay: 0s !important;
}
```

或尊重系统设置：

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 字体加载

视觉回归截图前必须等字体就绪，否则 fallback 字体 vs 目标字体宽度不同会导致大量假阳性：

```js
await document.fonts.ready;
```

设计层面：优先 `font-display: optional` 或预加载字体，减少 FOUT/FOIT。

### 时间相关 UI

「3 分钟前」「下午 4:32」这类动态文本要么 mock 时间（`Date.now`、`new Date`），要么用 `data-testid` 跳过截图对比。组件接受可注入的 `now` prop，便于 Story / 测试固定值。

### 随机 ID

`React.useId` 在 React 18+ 是稳定的（同次渲染稳定，不同环境可能不同）；但如果你用 `Math.random()` / `crypto.randomUUID()` 生成 DOM `id`，视觉回归会因 id 不同而判定差异。统一用 `useId` 或外部传入。

## AI Agent 专项

Computer-Use 类 agent 走视觉 + DOM 双通道，对前端的额外要求：

- **可点击区域 ≥ 40×40 CSS px**。小图标按钮要扩大可点击区（`padding` 或 `::before` 透明扩展）。
- **不要把唯一入口藏在 hover 后面**。AI agent 看不到 hover 态；移动端用户也用不了。改用常驻按钮 + 可选的「更多」菜单。
- **文字描述清晰**。`<button>X</button>` 不如 `<button aria-label="关闭对话框">×</button>`。
- **避免相同视觉的不同语义元素**。两个看起来一样的按钮如果功能不同，必须 label 不同。
- **关键状态用文字 + 图标双通道**。只用红色边框表示错误，色盲用户和图像识别都可能漏。

## 等待策略：让测试别 sleep

写组件时主动留出"可等待的信号"：

| 你提供 | 测试就能 |
|--------|---------|
| `aria-busy="true"` → 完成后变 `"false"` | `expect(el).toHaveAttribute('aria-busy', 'false')` |
| 列表完成加载后增删 `data-testid="skeleton"` | `expect(skeleton).not.toBeInTheDocument()` |
| Toast 出现是 `role="status"` 容器内插入新节点 | `findByRole('status')` 自动等待 |
| 路由跳转后新页面挂 `data-page="dashboard"` | `expect(page.locator('[data-page="dashboard"]')).toBeVisible()` |

**禁止**：让测试用 `setTimeout(500)` / `cy.wait(1000)` 来等。这是组件没提供可观察信号的结果，应该改组件而不是改测试。
