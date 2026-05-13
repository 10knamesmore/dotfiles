# Ant Design + 自研组件库适配（React）

本项目栈：**Ant Design + 自研组件库（多数自研组件是对 AntD 的封装）**。本文档专项给出 AntD 各核心组件的 Playwright-friendly 用法、testid 注入点、状态属性、与自研封装的契合点。

读完本文你应该能直接写出：

- 用 `page.getByRole` / `page.getByLabel` 命中 AntD 组件的 locator。
- 知道在哪里插 `data-testid`、要不要插。
- 知道哪些组件 AntD 已经做好了 a11y、哪些要补。
- 在自研 wrapper 里把上面这些做成默认。

## AntD 整体心智

- 大多数 AntD 组件**会**透传 `data-testid` 到组件根 DOM，但**不会**透传到内部关键交互节点（input、option、菜单项）。这是最常见的坑。
- AntD 的 class 前缀 `ant-` 跨小版本稳定，但跨大版本（4→5）会变。**禁止**用 `.ant-btn` 作 locator。
- AntD 5 的 `wave` 动画、`motion` 过渡会干扰视觉回归，测试环境要禁。
- 弹出层（`Select` / `DatePicker` / `Modal` / `Drawer` / `Tooltip` / `Popover`）默认渲染到 `body` 下的 `.ant-{X}-content-holder`，Playwright `getByRole` 默认走整个 document，不影响；但若用 `within(container)` 限定容器要小心。

## 表单：Form / Form.Item / Input

### 写法

```tsx
import { Form, Input, Button } from 'antd';

function LoginForm() {
  const [form] = Form.useForm();
  return (
    <Form
      form={form}
      layout="vertical"
      onFinish={(values) => doLogin(values)}
      data-testid="login-form"
    >
      <Form.Item
        label="邮箱"
        name="email"
        rules={[{ required: true, type: 'email', message: '请输入合法邮箱' }]}
      >
        <Input data-testid="login-email-input" autoComplete="email" />
      </Form.Item>
      <Form.Item
        label="密码"
        name="password"
        rules={[{ required: true, message: '请输入密码' }]}
      >
        <Input.Password data-testid="login-password-input" autoComplete="current-password" />
      </Form.Item>
      <Form.Item>
        <Button
          type="primary"
          htmlType="submit"
          loading={submitting}
          data-testid="login-submit-button"
        >
          登录
        </Button>
      </Form.Item>
    </Form>
  );
}
```

### Playwright locator

```ts
page.getByLabel('邮箱')                          // ✅ 优先：AntD 把 label 绑到 input
page.getByLabel('密码')                          // ✅
page.getByRole('button', { name: '登录' })       // ✅ 提交按钮
page.getByRole('alert')                          // ✅ Form.Item 校验失败的错误（AntD 渲染为 .ant-form-item-explain，**带 role="alert"**）
page.getByTestId('login-email-input')            // ✅ 兜底（i18n 时用）
```

### 关键点

- `Form.Item` 的 `data-testid` 落在外层 div，**不会**到 `<input>`。要给 input 加 testid，直接写在 `<Input data-testid="...">` 上。
- `Form.Item` 校验失败时，AntD 渲染的错误信息容器**有 `role="alert"`**，可以直接 `page.getByRole('alert')`。
- `Input.Password` 切换显隐眼睛图标按钮有 `aria-label`（"切换密码可见状态"），多个并存时考虑加 testid。
- 校验态：失败时 input 自动加 `aria-invalid="true"` ✅；要等 invalid 出现可用 `await expect(page.getByLabel('邮箱')).toHaveAttribute('aria-invalid', 'true')`。
- 提交按钮 `loading={true}` 时 AntD 自动加 `disabled` + 转圈，但**不会**加 `aria-busy`。如果测试要等 loading 消失，用 `await expect(submit).not.toBeDisabled()` 或自研封装里手动注入 `aria-busy`。

### 自研 wrapper 推荐

```tsx
// AppForm.tsx：强制 testid 前缀
interface AppFormProps extends FormProps {
  'data-testid': string;   // 必填
}
function AppForm({ 'data-testid': prefix, children, ...rest }: AppFormProps) {
  return (
    <FormTestIdContext.Provider value={prefix}>
      <Form {...rest} data-testid={prefix}>{children}</Form>
    </FormTestIdContext.Provider>
  );
}

// AppFormItem.tsx：根据 name 自动生成 input testid
function AppFormItem({ name, children, ...rest }: FormItemProps) {
  const prefix = useContext(FormTestIdContext);
  const testId = `${prefix}-${name}-input`;
  return (
    <Form.Item name={name} {...rest}>
      {React.isValidElement(children)
        ? React.cloneElement(children, { 'data-testid': testId })
        : children}
    </Form.Item>
  );
}
```

调用方零样板：

```tsx
<AppForm data-testid="login-form" onFinish={...}>
  <AppFormItem name="email" label="邮箱"><Input /></AppFormItem>
  <AppFormItem name="password" label="密码"><Input.Password /></AppFormItem>
</AppForm>
```

testid 全自动：`login-form`、`login-form-email-input`、`login-form-password-input`。

## Select / TreeSelect / Cascader / AutoComplete

### 写法

```tsx
<Select
  data-testid="user-role-select"
  placeholder="选择角色"
  options={[
    { value: 'admin', label: '管理员' },
    { value: 'member', label: '成员' },
  ]}
/>
```

### Playwright

```ts
// 打开下拉
await page.getByRole('combobox', { name: /角色/ }).click();
// 或：await page.getByLabel('角色').click();

// 选项 —— AntD 给 option 加了 role="option"
await page.getByRole('option', { name: '管理员' }).click();

// 断言已选
await expect(page.getByRole('combobox', { name: /角色/ })).toHaveText(/管理员/);
```

### 关键点

- `data-testid` 落在 `.ant-select` 外层，不在真正的 combobox 上。但 AntD 给 combobox 自带 `role="combobox"`，**有 label 就够了**。
- 选项弹出层在 `body` 下，每个 option 有 `role="option"` + 可见文本 → 直接 `getByRole('option', { name: ... })`。
- 多选 + 选项很多时（虚拟滚动），要先在搜索框里搜，再点 option，不要尝试 scroll 到所有 option。
- `Select` 加载远程数据时用 `loading` prop，会渲染 `aria-busy`-ish 的 spinner，但**不投射到 combobox 元素**。要等加载完成，自研 wrapper 里手动加 `aria-busy={loading}` 到外层。
- AntD 5 的 `Select` 支持 `popupRender` 自定义弹层结构，业务上加 testid 时优先在 option 上加 `data-option-value`：
  ```tsx
  <Select
    optionRender={(option) => (
      <span data-option-value={option.value}>{option.label}</span>
    )}
  />
  ```

## Table

### 写法

```tsx
<Table
  data-testid="user-table"
  rowKey="id"
  columns={columns}
  dataSource={users}
  loading={loading}
  onRow={(record) => ({
    'data-testid': 'user-table-row',
    'data-user-id': record.id,
  })}
/>
```

### Playwright

```ts
// 整张表
const table = page.getByTestId('user-table');

// 某一行（按业务 id）
const row = table.locator('[data-user-id="42"]');

// 行内某操作按钮
await row.getByRole('button', { name: '删除' }).click();

// 列头排序
await page.getByRole('columnheader', { name: /创建时间/ }).click();

// 空态
await expect(page.getByRole('cell', { name: /暂无数据|no data/i })).toBeVisible();
```

### 关键点

- `onRow` 返回的 props 透传到 `<tr>`，是给行加锚点的标准位置。
- 不要给每个 cell 都加 testid，靠 `row.getByRole('cell', { name: '...' })` 或 `row.locator(':nth-child(N)')`（仅限列固定的场景）。
- 表头有 `role="columnheader"`，列名作 accessible name，直接 `getByRole` 命中。
- `loading` 时表格内有 spinner，但表格根没有 `aria-busy`。要等加载完成，自研 wrapper 里给外层 div 投射 `data-state="loading|success|error"`：
  ```tsx
  function AppTable<T>({ loading, error, dataSource, ...rest }: AppTableProps<T>) {
    const state = error ? 'error' : loading ? 'loading' : 'success';
    return (
      <div data-state={state} aria-busy={loading || undefined}>
        <Table {...rest} loading={loading} dataSource={dataSource} />
      </div>
    );
  }
  ```
- 虚拟滚动 + 大数据量时，行不在 DOM 里 Playwright 找不到。搜索 / 筛选 / 翻页定位，不要靠 scroll。

## Modal / Drawer / Popconfirm

### 写法

```tsx
<Modal
  open={open}
  title="删除确认"
  okText="确认删除"
  cancelText="取消"
  onOk={confirm}
  onCancel={() => setOpen(false)}
  okButtonProps={{ danger: true, 'data-testid': 'delete-confirm-button' }}
  cancelButtonProps={{ 'data-testid': 'delete-cancel-button' }}
>
  此操作不可撤销。
</Modal>
```

### Playwright

```ts
const dialog = page.getByRole('dialog', { name: '删除确认' });
await dialog.getByRole('button', { name: '确认删除' }).click();
```

### 关键点

- AntD 的 `Modal` 自带 `role="dialog"` + `aria-modal="true"` + focus trap ✅。`title` prop 自动作 `aria-labelledby`。
- 给确认/取消按钮加 testid 走 `okButtonProps` / `cancelButtonProps`，不是 Modal 根的 `data-testid`。
- `Drawer` 同理：`role="dialog"`、focus trap 都有。
- `Popconfirm` 弹出的确认浮层是 `role="tooltip"`，不是 `dialog`，按钮没有 accessible name 默认是 "确定" / "取消"——多个 Popconfirm 同时存在时考虑 `okText` 含上下文（"确认删除张三"）。
- 关闭后焦点会回到 trigger（AntD 自动管理）。

## Tabs

```tsx
<Tabs
  items={[
    { key: 'profile', label: '个人信息', children: <Profile /> },
    { key: 'security', label: '安全设置', children: <Security /> },
  ]}
/>
```

```ts
await page.getByRole('tab', { name: '个人信息' }).click();
await expect(page.getByRole('tabpanel', { name: '个人信息' })).toBeVisible();
```

AntD Tabs ARIA 完整，不需要加 testid。

## Notification / Message

```tsx
notification.success({ message: '保存成功', description: '...' });
message.error('网络错误');
```

```ts
await expect(page.getByRole('alert')).toContainText('网络错误');
// 或：
await expect(page.getByText('保存成功')).toBeVisible();
```

AntD `notification` / `message` 容器**有** `role="alert"`。注意：

- 自动消失（默认 4.5s）。测试要在消失前断言，或用 `duration={0}` 让它常驻直到手动关闭。
- 多个同时弹出，用 `getByText` 区分。
- 视觉回归要 mock 时间或屏蔽通知区域。

## Upload

```tsx
<Upload data-testid="avatar-upload" beforeUpload={...}>
  <Button>上传头像</Button>
</Upload>
```

```ts
const fileInput = page.locator('input[type="file"]');
await fileInput.setInputFiles('/path/to/avatar.png');
```

AntD `Upload` 的真实 `<input type="file">` 隐藏，Playwright 直接 `setInputFiles` 即可，无需点按钮。

## Date / Time / Range Picker

```ts
await page.getByLabel('开始日期').click();
await page.getByRole('button', { name: '2026-05-13' }).click();
```

或更稳的：用 `fill()` 直接输入字符串（AntD DatePicker 支持键盘输入）：

```ts
await page.getByLabel('开始日期').fill('2026-05-13');
await page.keyboard.press('Enter');
```

后者**强烈推荐**：日历面板的 ARIA 复杂、月份切换 locator 脆，文本输入稳定得多。

## 全局 a11y / 测试相关配置

`ConfigProvider` 注入到 root：

```tsx
<ConfigProvider
  theme={{ token: { motion: false } }}  // 全局关动画，视觉回归专用
  locale={zhCN}                          // 锁 locale 避免英文环境下默认 enUS
>
  <App />
</ConfigProvider>
```

测试 / Storybook 环境用 `motion: false` 关 AntD 动画；运行时正常打开。

## AntD 已知坑（必记）

| 坑 | 现象 | 对策 |
|----|------|------|
| `Form.Item` 不透传 testid 到 input | testid 落在 wrapper div | 在 `<Input>` 上直接写 testid，或自研 `AppFormItem` |
| `Select` option 弹层在 body | `within(form)` 找不到 option | 直接 `page.getByRole('option')` |
| `Button loading` 不加 `aria-busy` | 测试无法用 `aria-busy` 等 | 用 `toBeDisabled()` 等，或自研 wrapper 注入 |
| `Modal` 关闭后立刻 `expect.toBeHidden()` 可能失败 | 关闭动画 200ms | 用 `await expect(dialog).not.toBeVisible()` 内置自动等待 |
| AntD 5 `Spin` 在 Table 里只盖一层，根上无状态属性 | 测试看不到 loading | 自研 `AppTable` 外层 `data-state` |
| `Popconfirm` 确认/取消文本默认 "确定/取消" 重复 | 多个并存时 locator 命中多个 | 用 `okText` / `cancelText` 加上下文 |
| `notification` 自动消失 | 测试断言可能在它消失后跑 | 用 `findByRole('alert')` 自动等，或测试时 `duration={0}` |
| AntD 表格 sticky header 用 `position: fixed`，截图位置可能偏 | 视觉回归不稳 | 截图前 `scrollIntoView` 或屏蔽 sticky 区域 |

## 自研组件库的统一规则

写自研组件时（无论是否包装 AntD）**默认行为**应该是：

1. **优先包 AntD**，但不是强制。AntD 有同语义件就吃它的 a11y / 键盘；AntD 没有（自研甘特图、流程编排、Schema 表单）就基于原生 HTML 自己写，严格按 `semantics-aria.md`。**禁止**用 `<div onClick>` 重写已存在的语义控件。
2. **每个 wrapper 都接收并透传** `data-testid`、`className`、`style`、`ref`、`aria-*`。
3. **状态投射**（loading / error / empty / disabled）做成 `data-state` + `aria-busy`：包 AntD 时补 AntD 没加的；从零写时自己注入完整。
4. **暴露 sub-component 给 sub-anchor**：
   ```tsx
   <AppTable>
     <AppTable.Toolbar data-testid="user-toolbar" />
     <AppTable.Filters data-testid="user-filters" />
   </AppTable>
   ```
5. **强制 i18n key 作 testid 默认值**：调用方传业务名，wrapper 自动拼成 testid，避免开发者忘记。
6. **TS 类型里把 testid 列入必填**（高频使用的组件）或可选但**有 JSDoc 提示**。
7. **Storybook 故事覆盖每个状态**（loading / error / empty / success），视觉回归直接拍 Story。

详见 `component-contract.md`。
