# React + 跨组件库陷阱

项目栈是 **React + AntD + 自研组件库**，本文档收口 React 通用陷阱与跨组件常见错误。AntD 具体用法见 `library-adaptation.md`，自研封装见 `component-contract.md`。

## React 列表 key

```tsx
// ❌ index 作 key
items.map((item, i) => <Row key={i} item={item} />)

// ✅ 业务稳定 ID
items.map(item => <Row key={item.id} item={item} />)
```

不稳定 key 的后果：

- 受控输入丢焦点 / 丢值。
- AntD `Table` 行选择状态错乱（rowKey 也要稳定 ID）。
- Playwright `locator` 可能拿到错的旧节点。

AntD Table：`rowKey="id"`（推荐传字符串字段名）或 `rowKey={(record) => record.id}`。

## useEffect 异步态

```tsx
useEffect(() => {
  let cancelled = false;
  setStatus('loading');
  fetchData().then(
    (data) => { if (!cancelled) { setData(data); setStatus('success'); } },
    (err)  => { if (!cancelled) { setError(err); setStatus('error');  } }
  );
  return () => { cancelled = true; };
}, []);
```

`status` 必须有，且必须投射到 DOM（`data-state` 或 `aria-busy`），否则测试无法等。fire-and-forget 的 useEffect 是 anti-pattern。

## dangerouslySetInnerHTML

避免使用。a11y、XSS、测试查询都受影响。必须用时：

- 内容 sanitize（DOMPurify）。
- 容器挂 `data-content-html="true"` 标识，测试侧明确知道是不受控内容。
- 不要在 testid 命名的元素上用。

## 受控组件丢焦点

```tsx
function Parent() {
  // ❌ 闭包组件每次重建，子组件每次卸载
  function Inner() { return <Input />; }
  return <Inner />;
}
```

把组件定义放在 render 外。或用 `useMemo`/`React.memo`。

## Form 与受控 input

AntD `Form` 自带值管理，**不要**在外面再 `useState` 同步同一个字段——双源会冲突。要监听值，用 `Form.useWatch`：

```tsx
const email = Form.useWatch('email', form);
```

## userEvent vs fireEvent

测试用 `@testing-library/user-event`，**不**用 `fireEvent`：

```tsx
import userEvent from '@testing-library/user-event';

const user = userEvent.setup();
await user.type(screen.getByLabelText('邮箱'), 'a@b.com');
await user.click(screen.getByRole('button', { name: '登录' }));
```

`userEvent` 触发完整事件链（focus → keydown → input → keyup → blur），更接近真实交互。`fireEvent.click` 只触发 click，AntD 一些组件依赖 mousedown / focus 行为，可能不响应。

## Portal 与测试

AntD `Modal` / `Drawer` / `Select` / `Tooltip` 全部用 portal 渲染到 `document.body`。这意味着：

- Testing Library 用 `screen.*`（查整个 document），**不**用 `container.querySelector`。
- Playwright `page.getByRole` 默认全 page 范围，没问题。

## React 19 / RSC

如果用 Server Components：

- 不能用 `useState` / `useEffect`，但**异步态依然要可观察**：用 `<Suspense fallback={<Skeleton data-testid="..." />}>` 包裹。
- 错误用 `error.tsx` boundary 渲染 `role="alert"`。
- 服务端渲染出的 HTML 直接要带 `data-state` 等属性，不能等客户端 hydration 才补。

## AntD 5 主题与动画

```tsx
import { ConfigProvider } from 'antd';

<ConfigProvider
  theme={{
    token: { motion: process.env.NODE_ENV !== 'test' },
  }}
  locale={zhCN}
>
  <App />
</ConfigProvider>
```

测试环境 / 视觉回归构建关掉 motion，否则动画期间 locator 命中与截图都不稳。

## 跨组件库常见陷阱

### 国际化文本变化导致 locator 失效

中文版 `'登录'`，英文版 `'Sign in'`。测试用 `getByText('登录')` 在英文环境挂。

对策：

- 关键交互（提交、删除、取消）加 `data-testid` 兜底。
- 用 `getByRole('button', { name: /登录|sign in/i })` 同时匹配。
- 项目内统一翻译 key 作 testid（`data-testid="auth.login.submit"`），i18n 体系里直接用。

### 同图标按钮重复出现

表格每行有 ✏️ 编辑、🗑️ 删除。无 label 时 AI 看不出区别、屏幕阅读器读"按钮按钮按钮"、`getByRole('button')` 返回多个。

**必须**给每个图标按钮带上下文 label：

```tsx
<AppButton
  type="text"
  icon={<EditIcon />}
  aria-label={`编辑用户 ${user.name}`}
  onClick={() => edit(user.id)}
/>
```

或 AntD Tooltip 包裹（Tooltip 会自动 `aria-describedby`，但**不一定**给 button 提供 accessible name，仍要 `aria-label`）。

### Toast / Notification 自动消失

AntD `notification` 默认 4.5s 消失。测试两条原则：

- 用 `await expect(page.getByRole('alert')).toContainText(...)` 内置自动等。
- 测试或 Storybook 里用 `duration={0}` 让它常驻。

视觉回归：完全屏蔽 notification 区域（截图 `mask`），别让消失时机干扰快照。

### CSS-in-JS 哈希

AntD 5 用 `@ant-design/cssinjs` 生成哈希 class（如 `.css-dev-only-do-not-override-1a2b3c`）。**禁止**作 locator。

### Skeleton 闪烁

数据 < 100ms 返回时 Skeleton 一闪而过，视觉回归会拍到不同状态。对策：

- 自研 wrapper 加最小展示时间（200ms）。
- 测试时固定数据延迟（mock 网络）。
- 视觉回归用 `mask` 屏蔽 skeleton 区域。

### Modal 关闭后焦点丢失

AntD `Modal` 默认会还原焦点到打开它的元素（如果在你提供的 `getContainer` 之外，请验证）。自定义模态 / popover 时务必：

- 打开前 `const trigger = document.activeElement;`
- 关闭时 `trigger?.focus();`

### 虚拟滚动

AntD `Table` 配 `virtual` 后，非视口内的行不在 DOM。Playwright 找不到。对策：

- 用搜索 / 筛选定位，不要靠 scroll。
- 测试场景用小数据集，避免虚拟滚动。
- 必须验证虚拟滚动行为时，用 `await row.scrollIntoViewIfNeeded()`。

## 极少数情况

### Vue / 原生 HTML

仓库里偶尔会遇到老页面（旧 Vue 子系统、邮件模板、第三方嵌入页）。规则：

- Vue：模板里别写 `<div @click>` 当按钮；`v-if` 优于 `v-show + aria-hidden`；`defineProps` + `defineEmits` 是契约。
- 原生 HTML：完全遵循 `semantics-aria.md` 与 `testing-anchors.md`。状态用 `data-*` 属性，事件用 `CustomEvent` 派发。
- Web Component：避免 closed shadow DOM；testid 挂在 host 上或用 `::part`。

这些场景占比小，遇到才查 `semantics-aria.md` 完整规则。
