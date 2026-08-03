# A11y locator patterns

本文件给出 component semantics/state 标注到 test assertion 的映射，不定义业务测试流程。

## 按钮

有可见文字时直接使用文字作为 accessible name：

```jsx
<button type="button">保存</button>
```

```ts
page.getByRole('button', { name: '保存' })
```

纯图标按钮提供 `aria-label`，图标本身不参与命名：

```jsx
<button type="button" aria-label="关闭">
  <CloseIcon aria-hidden="true" />
</button>
```

## UI state matrix

只验证各状态的 a11y representation，不验证产生状态的业务流程。

### Loading

保持主要动作的 name 不变，并让 loading 区域可定位：

```jsx
<section aria-label="订单列表" aria-busy={loading}>
  <button type="button" aria-disabled={loading}>刷新</button>
  {loading && <div role="status" aria-label="订单加载状态">加载中</div>}
</section>
```

```ts
const region = page.getByRole('region', { name: '订单列表' })
region.getByRole('button', { name: '刷新' })
region.getByRole('status', { name: '订单加载状态' })
```

### Success

```jsx
<div role="status" aria-label="保存结果">已保存</div>
```

```ts
page.getByRole('status', { name: '保存结果' })
```

### Error

```jsx
<div role="alert" aria-label="保存错误">保存失败</div>
```

```ts
page.getByRole('alert', { name: '保存错误' })
```

### Empty

```jsx
<section aria-label="搜索结果">
  <div role="status" aria-label="搜索结果状态">未找到结果</div>
</section>
```

```ts
page
  .getByRole('region', { name: '搜索结果' })
  .getByRole('status', { name: '搜索结果状态' })
```

组件没有某个状态时不必补造；组件支持该状态时必须能用 a11y locator 找到对应目标。

## 表单字段

用 `<label>` 建立 name，不依赖 placeholder：

```jsx
<label htmlFor="email">邮箱</label>
<input id="email" type="email" />
```

```ts
page.getByLabel('邮箱')
page.getByRole('textbox', { name: '邮箱' })
```

多个组内存在同名字段时，先用有名字的 group 收窄：

```jsx
<fieldset>
  <legend>账单地址</legend>
  <label htmlFor="billing-city">城市</label>
  <input id="billing-city" />
</fieldset>
```

```ts
page
  .getByRole('group', { name: '账单地址' })
  .getByRole('textbox', { name: '城市' })
```

## Dialog 与 region

容器本身必须有 accessible name，才能作为稳定 scope：

```jsx
<div role="dialog" aria-labelledby="delete-title">
  <h2 id="delete-title">确认删除</h2>
  <button type="button">确认</button>
</div>
```

```ts
page
  .getByRole('dialog', { name: '确认删除' })
  .getByRole('button', { name: '确认' })
```

## 重复行内操作

让 name 带上目标上下文，或先按有名字的 row 收窄：

```jsx
<tr>
  <th scope="row">订单 #42</th>
  <td><button aria-label="删除订单 #42">删除</button></td>
</tr>
```

```ts
page
  .getByRole('row', { name: /订单 #42/ })
  .getByRole('button', { name: '删除订单 #42' })
```

## Tabs、checkbox 与 switch

role 和 name 定位元素，ARIA state 只用于区分当前目标：

```ts
page.getByRole('tab', { name: '通用', selected: true })
page.getByRole('checkbox', { name: '接收通知', checked: true })
page.getByRole('switch', { name: '深色模式', checked: true })
```

不要为了验证这些 locator 而测试切换动作的业务结果。

## Table 与 list

真实数据使用原生 table/list 语义，先定位命名容器再定位内容：

```jsx
<table>
  <caption>订单</caption>
  <thead><tr><th scope="col">订单号</th><th scope="col">金额</th></tr></thead>
  <tbody><tr><th scope="row">#42</th><td>100</td></tr></tbody>
</table>
```

```ts
const table = page.getByRole('table', { name: '订单' })
table.getByRole('columnheader', { name: '金额' })
table.getByRole('row', { name: /#42/ })
```

## 文件输入

保留真实 input，并提供 label：

```jsx
<label htmlFor="avatar">上传头像</label>
<input id="avatar" type="file" />
```

```ts
page.getByLabel('上传头像')
```

## 图形内容

需要作为目标定位的信息型 SVG 使用 `img` role 和 name：

```jsx
<svg role="img" aria-labelledby="trend-title">
  <title id="trend-title">季度营收趋势</title>
</svg>
```

canvas 或 WebGL 内部目标无法进入 accessibility tree。为需要定位的目标提供真实 DOM button、link、table 或 list 入口。
