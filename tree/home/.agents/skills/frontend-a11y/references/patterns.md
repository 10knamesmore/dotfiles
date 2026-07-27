# 组件 ARIA 模式(agent 可定位版)

每个模式给出:必须暴露的 role/name/state,以及 agent 对应的 locator。目标是「snapshot 里能唯一找到、能读状态、状态变化能感知」。基于 WAI-ARIA Authoring Practices,只保留 agent 定位相关的关键点。

## 图标按钮 / 纯图形控件

无文字的按钮必须有 accessible name;装饰性图标 `aria-hidden`。

```jsx
<button type="button" aria-label="关闭" onClick={onClose}>
  <XIcon aria-hidden="true" />
</button>
```
agent:`getByRole('button', { name: '关闭' })`

## 提交按钮的 loading(name 不漂移)

反例:`{loading ? 'Sending' : 'Subscribe'}` —— name 变了,脚本失配。
正例:name 固定,状态用 `aria-busy`,spinner `aria-hidden`。

```jsx
<button type="submit" aria-busy={loading} disabled={loading}>
  <span className="spinner" aria-hidden="true" hidden={!loading} />
  Subscribe
</button>
```
agent:全程 `getByRole('button', { name: 'Subscribe' })`;判 loading 读 `aria-busy` 或 `expect(btn).toBeDisabled()`。

## 表单字段 + 校验错误

label 关联提供 name;错误进 live region 且 `aria-describedby` 关联回字段。

```jsx
<label htmlFor="email">邮箱</label>
<input id="email" type="email" aria-invalid={!!error}
       aria-describedby={error ? 'email-err' : undefined} />
<span id="email-err" role="alert">{error}</span>
```
agent:`getByRole('textbox', { name: '邮箱' })`;错误出现后 snapshot 里 `alert` 节点可读。

## 模态 Dialog

`role="dialog"` + `aria-modal="true"` + 用 `aria-labelledby` 指向标题;打开移焦点进去、Esc 关闭、关闭归还焦点、焦点陷在内部。

```jsx
<div role="dialog" aria-modal="true" aria-labelledby="dlg-title">
  <h2 id="dlg-title">确认删除</h2>
  …
  <button aria-label="关闭对话框">×</button>
</div>
```
agent:`getByRole('dialog', { name: '确认删除' })`。

## Disclosure / 折叠面板

触发器用 `aria-expanded` 表达开合;`aria-controls` 指向被控区域。

```jsx
<button aria-expanded={open} aria-controls="panel-1" onClick={toggle}>详情</button>
<div id="panel-1" hidden={!open}>…</div>
```
agent:`getByRole('button', { name: '详情' })`,读 `aria-expanded` 断言状态。

## Tabs(挂了 role 必须补键盘)

`role="tablist"` > `role="tab"`(`aria-selected`、`aria-controls`);面板 `role="tabpanel"`(`aria-labelledby`)。选中态走 `aria-selected` 不是 class。**键盘契约不可省**:方向键切换 + roving tabindex(仅选中 tab `tabindex=0`,其余 `-1`)。listbox/menu/radiogroup 同理——挂 role = 承诺这套键盘模型。Home/End(跳首末)仅 slider 强制,长 listbox 推荐、tab/menu 可选,radiogroup 无。

```jsx
<div role="tablist" aria-label="设置">
  <button role="tab" aria-selected={i===0} aria-controls="p0" id="t0"
          tabIndex={i===0 ? 0 : -1} onKeyDown={handleArrowKeys}>通用</button>
  …
</div>
<div role="tabpanel" id="p0" aria-labelledby="t0">…</div>
```
agent:`getByRole('tab', { name: '通用' })`,`aria-selected` 判当前页;键盘走查须实测方向键能切、只有一个 tabstop。

## Toast / 通知

短暂信息进 live region 才会被 snapshot 捕获。非阻断用 `role="status"`(polite),紧急/错误用 `role="alert"`(assertive)。

```jsx
<div role="status" aria-live="polite">{message}</div>
```

## 列表中的重复操作(消歧)

每行都有 `Delete` 会让 `getByRole` 命中多个。给每个 name 独有上下文:

```jsx
<button aria-label={`删除发票 #${invoice.id}`}>删除</button>
```
agent:`getByRole('button', { name: '删除发票 #12' })` 唯一命中。

## 复选 / 开关 / 单选

原生优先(`<input type="checkbox">` 自带 role+checked)。自建开关补 `role="switch"` + `aria-checked`。选中态永远走 `aria-checked`,不靠 class。

```jsx
<button role="switch" aria-checked={on} aria-label="深色模式" onClick={toggle} />
```
agent:`getByRole('switch', { name: '深色模式' })`,读 `aria-checked`。

## 单选/复选组(组级名字)

一组选项的问题是 **group 级**语义。用 `fieldset`+`legend`(legend 成为组的 accessible name),或 `role="radiogroup"`+`aria-labelledby`。注意:**组名不会拼进组内字段的 name**——跨组同名字段仍要各自 label 带上下文。

```jsx
<fieldset>
  <legend>配送方式</legend>
  <label><input type="radio" name="ship" value="express" /> 次日达</label>
  <label><input type="radio" name="ship" value="normal" /> 普通</label>
</fieldset>
```
agent:`getByRole('radiogroup', { name: '配送方式' })` 或 `getByRole('group',{name:'账单地址'}).getByRole('textbox',{name:'城市'})` 先按组名收窄再定位。组级必填/错误用 `aria-required`/`aria-invalid` 挂在 fieldset/radiogroup 上。

## 表单字段的完整属性

label 关联 + `required` + `autocomplete` + 错误关联,缺一 agent 就少读一维状态。

```jsx
<label htmlFor="email">邮箱</label>
<input id="email" type="email" name="email" autoComplete="email" required
       aria-required="true" aria-invalid={!!error}
       aria-describedby={[error && 'email-err', 'email-hint'].filter(Boolean).join(' ') || undefined} />
<span id="email-hint" aria-live="polite">用于登录,不公开</span>
<span id="email-err" role="alert">{error}</span>
```
要点:`required`/`aria-required` 让 agent `toBeRequired()`;`autoComplete` token(`email`/`tel`/`current-password`/`one-time-code`…)是字段用途的机器可读标识(WCAG 1.3.5);提示与错误可用空格分隔的多 id 一起挂进 `aria-describedby`;计数/剩余名额用 `aria-live="polite"`(别用 assertive 逐字刷屏)。值可读但不可改用 `readOnly` 不是 `disabled`(disabled 值不提交)。

## 提交失败:error summary + 焦点

长表单单靠 per-field 错误,agent 不知道"总共错几处"。提交失败渲染汇总区,链到各字段,并移焦点过去。

```jsx
{errors.length > 0 && (
  <div role="alert" tabIndex={-1} ref={summaryRef} id="err-summary">
    <h2>有 {errors.length} 处需修正</h2>
    <ul>{errors.map(e => <li key={e.field}><a href={`#${e.field}`}>{e.message}</a></li>)}</ul>
  </div>
)}
// 提交失败后:summaryRef.current?.focus()
```
agent:一次 `getByRole('alert')` 读全部错误,`getByRole('link',{name})` 跳字段。

## 进度条

确定性进度必须暴露数值,别用 `<div style="width:45%">`。

```jsx
<div role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}
     aria-valuetext={`${pct}%`} aria-label="上传进度" />
```
不确定进度:省略 `aria-valuenow`(role=progressbar 无值即表示 indeterminate),区域配 `aria-busy`。agent:`getByRole('progressbar')` 读 `aria-valuenow` 按值等待完成。

## 真数据表格

```jsx
<table>
  <caption>2024 订单</caption>
  <thead><tr><th scope="col">订单号</th><th scope="col">金额</th></tr></thead>
  <tbody>
    <tr data-testid="order-row" data-order-id={o.id}>
      <th scope="row">{o.no}</th><td>{o.amount}</td>
    </tr>
  </tbody>
</table>
```
`th scope` 关联表头与数据。agent:`getByRole('table',{name:'2024 订单'})`、`getByRole('columnheader',{name:'金额'})` 点排序、`getByRole('row',{name:/#12/}).getByRole('cell')`。`<table>` 只用于真数据,别做布局;布局用 flex/grid。

## 列表

```jsx
<ul data-testid="todo-list">
  <li data-testid="todo-item" data-todo-id="42">
    <input type="checkbox" aria-label="完成「买菜」" /><span>买菜</span>
    <button aria-label="删除「买菜」">×</button>
  </li>
</ul>
```
agent:`within(getByRole('list')).getByRole('listitem')` 计数遍历;具体项用 `[data-todo-id="42"]`。设 `list-style:none` 时补 `role="list"` 保 SR 语义。

## 文件上传(别把真 input 藏死)

真实 `<input type="file">` 始终留在 DOM(用 sr-only/`opacity` 而非 `display:none`),给 `aria-label`,agent 直接注入文件,无需点按钮触发原生选择器。

```jsx
<label htmlFor="avatar">上传头像</label>
<input id="avatar" type="file" accept="image/*"
       style={{ position:'absolute', width:1, height:1, opacity:0 }} />
<button type="button" onClick={() => document.getElementById('avatar').click()}>选择文件</button>
```
agent:`page.getByLabel('上传头像').setInputFiles('a.png')` 或 `locator('input[type=file]').setInputFiles(...)`。上传中/结果投射 `aria-busy` + `role="status"`。

## 信息型 SVG / canvas 替代

SVG 当图标见「图标按钮」;当**信息载体**(logo/趋势箭头/状态):

```jsx
<svg role="img" aria-labelledby="t"><title id="t">下降趋势</title>…</svg>
```

canvas/WebGL 画的图表/画布在 AX 树是黑洞,提供 DOM 层替代:

```jsx
<figure>
  <canvas aria-hidden="true" />                    {/* 只做绘制 */}
  <table className="sr-only">                       {/* 同一份数据,agent 可读 */}
    <caption>季度营收</caption>
    <thead><tr><th scope="col">季度</th><th scope="col">营收</th></tr></thead>
    <tbody>{rows.map(r => <tr key={r.q}><th scope="row">{r.q}</th><td>{r.v}</td></tr>)}</tbody>
  </table>
</figure>
```
交互点(可点的 bar/point/图例)用真实 DOM 覆盖层承接点击 + name,别指望 agent 点 canvas 内部坐标。

## 虚拟滚动列表

视口外行不在 DOM。首选给列表**搜索/筛选/翻页**入口(带 role+name)让 agent 用过滤而非盲滚定位目标;行进入 DOM 后带 `role="row"` + 稳定 `data-<biz>-id`。需要滚动时 `await row.scrollIntoViewIfNeeded()`。文档注明该列表虚拟化、总数从别处(如 `aria-rowcount` 或计数文本)读。
