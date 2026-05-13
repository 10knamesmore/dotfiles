# 自研组件库契约

自研组件库的目标：**把 a11y + testid + 状态投射做成默认行为**，让业务工程师调用 wrapper 就自动获得 Playwright-friendly 的 DOM。

自研组件有三种典型形态：

- **A. 包装 AntD**（最常见）：`<AppButton>` 内核是 `<AntdButton>`，自研层注入业务约定。
- **B. 基于原生 HTML 元素**：业务上 AntD 没有对应件，或刻意不用 AntD（性能、定制度、依赖洁癖）。如自研 `<DataGrid>`、`<Workflow>`、`<TimelineEditor>`、`<MarkdownPreview>`。内核是 `<button>` / `<div role="...">` / `<table>` 等原生标签。
- **C. 混合**：复合组件外层自研、内部部分用 AntD（如自研 `<FormBuilder>` 内部 input 用 AntD `Input`）。

本文档面向**写自研组件**的人，不是用自研组件的人。三种形态共享下面的契约。

## 五条不变量（破坏其一就是 bug）

1. **始终落在合适的语义标签上**：交互元素必须是 `<button>` / `<a>` / `<input>` / `<select>`，或 AntD 已经包好的等价件；不要用 `<div onClick>` 假装按钮。形态 A 直接吃 AntD 的语义，形态 B/C 自己写时严格遵循 `semantics-aria.md`。
2. **`data-testid` / `aria-*` / `className` / `style` / `ref` 必须透传到根 DOM**。单测里加一条契约测试守住。
3. **业务状态必须投射成 DOM 属性**：`data-state="idle|loading|success|error|empty|disabled"` 与 `aria-busy` / `aria-invalid` / `aria-disabled` / `aria-expanded` 等。
4. **复合组件用 sub-component 暴露内层锚点**（`<AppTable.Toolbar>` / `<AppForm.Item>`），不要把整块做成黑盒。
5. **每个组件都有 Storybook 故事覆盖所有状态**：loading / error / empty / disabled / success / focused / open。视觉回归直接对故事拍。

## 选型：什么时候包 AntD，什么时候不包

| 场景 | 选 |
|------|-----|
| AntD 有同语义组件且无明显短板 | **包 AntD**。继承 a11y / 键盘 / 表单关联 / locale。不要重复造。 |
| AntD 没有（如自研甘特图、流程编排、富文本扩展） | **基于原生 HTML 自己写**。严格按 `semantics-aria.md`。 |
| AntD 有但行为/性能/样式不满足 | 先尝试 `theme` / `slotProps` / `componentsProps` 改造；改不了再换原生。**不要**在 AntD 外套一层 div 然后所有事件转发——会丢 a11y。 |
| 同一类组件需要"AntD 版"和"高性能版"并存 | 拆两个：`<AppTable>` 包 AntD，`<AppVirtualTable>` 自研。Props 类型分开。 |

## 组件粒度

| 反模式 | 正解 |
|--------|------|
| `<UserDashboard>` 一个组件 800 行 | 拆为 `<UserDashboard>` 容器 + `<UserStats>` + `<UserActivity>` + `<UserActions>` |
| `<DataTable>` 把表头、筛选、分页、空态都塞内部，不可拆 | sub-component 暴露：`<DataTable.Toolbar>` `<DataTable.Filters>` `<DataTable.Empty>` `<DataTable.Pagination>` |
| 一个 prop 接 12 个 boolean 控制内部分支 | 拆为多个组件，或用 `variant: 'compact' \| 'expanded'` string union |
| 把 fetch 写在组件内 | UI 接受数据 + 回调；fetch 放在 hook（`useUsers`），便于测试 mock 与组件单独渲染 |

## Props 命名（必须遵守）

| 类型 | 命名 | 例子 |
|------|------|------|
| 布尔标志 | `isX` / `hasX` / `canX` / `shouldX` | `isLoading`、`hasError`、`canEdit` |
| 事件回调 | `onX`（X 是动词） | `onSubmit`、`onSelect`、`onChange` |
| 渲染插槽 | `renderX` / `children` | `renderEmpty`、`renderHeader` |
| 数据 | 业务名词，复数加 `s` | `users`、`selectedUserId` |
| 受控值 | `value` + `onChange`（与原生 input 对齐） | `value`、`onChange` |
| 测试锚点 | `data-testid`（必填或强烈推荐） | — |

**禁止**：`data` / `info` / `item` / `flag` / `mode` / `type`（笼统）、`btn` / `msg` / `usr`（缩写）。

## TS 类型必须显式

```ts
import type { ButtonProps as AntdButtonProps } from 'antd';

export interface AppButtonProps extends Omit<AntdButtonProps, 'loading'> {
  /** 异步状态；自动注入 loading + aria-busy + disabled */
  isLoading?: boolean;
  /** 错误状态；自动 danger + aria-invalid */
  hasError?: boolean;
  /** 测试 / agent 用的稳定锚点 */
  'data-testid'?: string;
}
```

继承 AntD 的 Props 类型，**收窄或扩展**业务约定。`Omit` 那些自研层要重写语义的字段（如 `loading` 改名为 `isLoading`），避免两套并存。

## 状态投射标准模板

写每个自研组件都按这个模板。

### 形态 A：包装 AntD

```tsx
import { Button as AntdButton } from 'antd';
import type { AppButtonProps } from './types';

export function AppButton({
  isLoading,
  hasError,
  disabled,
  'data-testid': testId,
  children,
  ...rest
}: AppButtonProps) {
  const state = isLoading ? 'loading' : hasError ? 'error' : disabled ? 'disabled' : 'idle';
  return (
    <AntdButton
      {...rest}
      loading={isLoading}
      disabled={disabled || isLoading}
      danger={hasError || rest.danger}
      data-testid={testId}
      data-state={state}
      aria-busy={isLoading || undefined}
      aria-invalid={hasError || undefined}
    >
      {children}
    </AntdButton>
  );
}
```

要点：

- AntD 原件的 props（`loading`、`disabled`、`danger`）继续传，让 AntD 处理视觉与 disabled 行为。
- 在外层补 AntD **没自动加的** `aria-busy`、`aria-invalid`、`data-state`。
- 用 `value || undefined` 而不是 `value || false`：false 也会渲染成 `aria-busy="false"`，noise；只在 true 时存在更干净。
- `disabled || isLoading` 双重保险：loading 时也不可点。

### 形态 B：基于原生 HTML

```tsx
export const AppIconToggle = React.forwardRef<HTMLButtonElement, AppIconToggleProps>(
  function AppIconToggle(
    { isPressed, isLoading, disabled, icon, label, 'data-testid': testId, onClick, ...rest },
    ref
  ) {
    const state = isLoading ? 'loading' : disabled ? 'disabled' : isPressed ? 'pressed' : 'idle';
    return (
      <button
        ref={ref}
        type="button"
        {...rest}
        aria-label={label}
        aria-pressed={isPressed}
        aria-busy={isLoading || undefined}
        disabled={disabled || isLoading}
        data-testid={testId}
        data-state={state}
        onClick={onClick}
      >
        {icon}
      </button>
    );
  }
);
```

要点：

- 内核是真正的 `<button>`，键盘 / 焦点 / 表单提交全部免费。
- ARIA 状态自己投射（AntD 不在这条路上）。
- 别忘了 `disabled` 同时阻止点击和 Tab 焦点（原生行为）。
- 对自定义 role 类组件（如 `role="tab"`、`role="option"`），按 WAI-ARIA Authoring Practices 实现完整键盘行为（方向键、Home/End）；不要只挂 role 不补键盘。

### 形态 C：混合

复合外层自研、内部子节点用 AntD。规则：

- 外层 div 投射 `data-state`、`aria-busy` 等容器级状态。
- 内部 AntD 组件按形态 A 规则处理（透传 testid、`aria-*`）。
- sub-component 暴露内层锚点（见下文）。

## 必须透传的字段

```ts
// 这一组永远透传到根：
'data-testid'
'data-*'      // 业务可能传 data-user-id 等
'aria-*'
'className'
'style'
'id'
'ref'         // 用 forwardRef
'onClick' / 'onKeyDown' / 其他原生事件 // 通过 ...rest 自动透传
```

实现：

```tsx
export const AppCard = React.forwardRef<HTMLDivElement, AppCardProps>(
  function AppCard({ title, children, 'data-testid': testId, ...rest }, ref) {
    return (
      <div ref={ref} {...rest} data-testid={testId} className={cn('app-card', rest.className)}>
        {title && <div className="app-card__title">{title}</div>}
        <div className="app-card__body">{children}</div>
      </div>
    );
  }
);
```

## sub-component：复合组件的标准结构

业务里 90% 复合组件（Table / Form / Card / List）都需要在内部某个节点上挂锚点。**不要**靠 props 一个个塞，做成 sub-component：

```tsx
function AppTable<T>({ children, ...rest }: AppTableProps<T>) {
  return <div {...rest}><AntdTable {...rest} /></div>;
}

AppTable.Toolbar = function Toolbar({ children, ...rest }: HTMLProps<HTMLDivElement>) {
  return <div className="app-table__toolbar" {...rest}>{children}</div>;
};

AppTable.Filters = function Filters({ children, ...rest }: HTMLProps<HTMLDivElement>) {
  return <div className="app-table__filters" {...rest}>{children}</div>;
};

AppTable.Empty = function Empty({ children = '暂无数据', ...rest }: HTMLProps<HTMLDivElement>) {
  return <div role="status" {...rest}>{children}</div>;
};
```

调用方：

```tsx
<AppTable data-testid="user-table" dataSource={users}>
  <AppTable.Toolbar data-testid="user-table-toolbar">
    <Search data-testid="user-search" />
  </AppTable.Toolbar>
</AppTable>
```

每层都能独立挂 testid，Playwright 用 `within(toolbar)` 限定后续查询。

## 锚点契约：写在文档与类型上

每个自研组件的 README / TSDoc 顶部，必须列「锚点契约」：

```tsx
/**
 * AppButton
 *
 * Test anchors（稳定，破坏需 major bump）:
 *   - 根：透传 data-testid
 *   - data-state: 'idle' | 'loading' | 'error' | 'disabled'
 *   - aria-busy="true" 当 isLoading
 *   - aria-invalid="true" 当 hasError
 *
 * Locator 优先级：
 *   1. page.getByRole('button', { name: '<children 文本>' })
 *   2. page.getByTestId('<your-testid>')
 *
 * 示例：
 *   await expect(submit).toHaveAttribute('data-state', 'loading');
 */
export function AppButton(props: AppButtonProps) { ... }
```

测试团队、AI agent、业务 RD 都能查到。

## testid 命名规范

格式：`<feature>-<element>-<role>`，全小写 kebab-case。

```
✅ login-form
✅ login-form-email-input
✅ login-form-submit-button
✅ user-table
✅ user-table-row             (每行一个，靠 data-user-id 区分)
✅ user-actions-edit-button
✅ avatar-upload-input

❌ btn1
❌ Button
❌ user-list-item-3           (用 data-user-id 不要序号入 testid)
❌ div                        (元素名不是名字)
❌ submitBtn                  (camelCase)
```

**列表项**：testid 表示"这是哪一类元素"，业务 id 单独用 `data-*`：

```tsx
<tr data-testid="user-row" data-user-id={user.id}>
```

查询：`page.getByTestId('user-row').filter({ has: page.locator('[data-user-id="42"]') })` 或 wrapper 内提供 helper。

## Form 自研封装：自动生成 testid

最容易出错的就是 Form 字段——每个调用方手动给 input 加 testid 不现实。封装一层自动注入：

```tsx
const FormTestIdContext = React.createContext<string | undefined>(undefined);

export function AppForm({ 'data-testid': testId, children, ...rest }: AppFormProps) {
  return (
    <FormTestIdContext.Provider value={testId}>
      <Form {...rest} data-testid={testId}>{children}</Form>
    </FormTestIdContext.Provider>
  );
}

export function AppFormItem({ name, children, ...rest }: AppFormItemProps) {
  const prefix = useContext(FormTestIdContext);
  const childTestId = prefix && name ? `${prefix}-${name}-input` : undefined;

  const child = React.Children.only(children) as React.ReactElement;
  const injected = React.cloneElement(child, {
    'data-testid': child.props['data-testid'] ?? childTestId,
  });

  return <Form.Item name={name} {...rest}>{injected}</Form.Item>;
}
```

业务调用：

```tsx
<AppForm data-testid="login-form">
  <AppFormItem name="email" label="邮箱"><Input /></AppFormItem>
  <AppFormItem name="password" label="密码"><Input.Password /></AppFormItem>
</AppForm>
```

生成的 testid：`login-form-email-input` / `login-form-password-input`。零样板，业务工程师不会忘记。

## 副作用与异步：抽到 hook

```tsx
// useUsers.ts
export function useUsers() {
  const [data, setData] = useState<User[]>([]);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    setStatus('loading');
    fetchUsers().then(
      (users) => { if (!cancelled) { setData(users); setStatus('success'); } },
      (err)   => { if (!cancelled) { setError(err); setStatus('error');   } },
    );
    return () => { cancelled = true; };
  }, []);

  return { data, status, error };
}

// UserList.tsx
function UserList() {
  const { data, status, error } = useUsers();
  return (
    <AppTable data-testid="user-list" data-state={status} aria-busy={status === 'loading'}>
      {status === 'loading' && <AppTable.Skeleton data-testid="user-list-skeleton" />}
      {status === 'error'   && <AppTable.Empty role="alert">{error?.message}</AppTable.Empty>}
      {status === 'success' && data.length === 0 && <AppTable.Empty>暂无用户</AppTable.Empty>}
      {status === 'success' && data.length  >  0 && /* render rows */}
    </AppTable>
  );
}
```

收益：

- Playwright 等 `data-state="success"` 就能继续，不用 sleep。
- 单测可 mock `useUsers` 直接渲染各状态。
- Storybook 故事直接传假数据覆盖所有状态。

## 文件结构（每个自研组件）

```
packages/ui/src/AppButton/
├── AppButton.tsx              # 组件本体
├── AppButton.types.ts         # Props / 锚点契约（也可合在 .tsx）
├── AppButton.test.tsx         # 单测（Testing Library）
├── AppButton.stories.tsx      # Storybook（覆盖所有状态）
├── README.md                  # 锚点契约 + 使用示例
└── index.ts                   # 只导出公开 API
```

## 契约测试（守住透传）

每个自研组件都加一条：

```tsx
import { render, screen } from '@testing-library/react';
import { AppButton } from './AppButton';

test('AppButton 透传 data-testid 到根 DOM', () => {
  render(<AppButton data-testid="my-btn">点我</AppButton>);
  expect(screen.getByTestId('my-btn')).toBeInTheDocument();
});

test('AppButton 在 isLoading 时投射 aria-busy 与 data-state', () => {
  render(<AppButton isLoading data-testid="x">点我</AppButton>);
  const btn = screen.getByTestId('x');
  expect(btn).toHaveAttribute('aria-busy', 'true');
  expect(btn).toHaveAttribute('data-state', 'loading');
  expect(btn).toBeDisabled();
});

test('AppButton 在 hasError 时投射 aria-invalid', () => {
  render(<AppButton hasError data-testid="x">点我</AppButton>);
  expect(screen.getByTestId('x')).toHaveAttribute('aria-invalid', 'true');
});

test('AppButton 透传任意 aria-*', () => {
  render(<AppButton aria-label="自定义" data-testid="x">点我</AppButton>);
  expect(screen.getByTestId('x')).toHaveAttribute('aria-label', '自定义');
});
```

这四条契约一旦失守，调用方所有的 Playwright 测试都会挂——这就是为什么必须有契约测试。

## Storybook：状态覆盖

每个组件至少这几个 story：

```tsx
// AppButton.stories.tsx
export default { component: AppButton };

export const Idle      = { args: { children: '保存' } };
export const Loading   = { args: { children: '保存', isLoading: true } };
export const Disabled  = { args: { children: '保存', disabled: true } };
export const Error     = { args: { children: '保存', hasError: true } };
export const WithIcon  = { args: { children: '保存', icon: <SaveIcon /> } };
export const LongLabel = { args: { children: '保存一个特别长的标签文本以测试换行行为' } };
```

视觉回归引擎（Chromatic）会自动对每个 story 拍快照。

## 检查清单（写完一个自研组件）

- [ ] 内核语义正确？形态 A 是 AntD 原件、形态 B 是 `<button>` / `<a>` / `<input>` 等原生元素（不是 `<div onClick>`）？
- [ ] `data-testid` / `aria-*` / `className` / `style` / `id` / `ref` 透传到根 DOM？
- [ ] 业务状态都投射成了 `data-state` + 对应 `aria-*`？
- [ ] 复合组件拆成了 sub-component，每层可挂锚点？
- [ ] Props 类型显式，命名遵循 `isX`/`onX` 等规范？
- [ ] 副作用抽到 hook，UI 组件纯展示？
- [ ] README / TSDoc 写了锚点契约？
- [ ] 单测覆盖了所有契约（透传、状态投射）？
- [ ] Storybook 故事覆盖所有状态（loading / error / empty / disabled / success）？
- [ ] 用 `npx playwright eval` 在 Storybook URL 上验证关键 locator 可命中？
