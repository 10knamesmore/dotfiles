# 语义化 HTML 与 ARIA

ARIA 第一守则：**能用原生 HTML 就别用 ARIA**。原生标签自带 role、键盘行为、焦点管理；用 div + ARIA 等于人肉重写浏览器，几乎一定漏。

## 反模式 → 正解对照

| 反模式 | 正解 | 原因 |
|--------|------|------|
| `<div onClick={...}>提交</div>` | `<button type="button" onClick={...}>提交</button>` | div 不可聚焦、不响应 Enter/Space、屏幕阅读器不识别 |
| `<span class="link" onClick={...}>` | `<a href="...">` 或 `<button>` | 无 `href` 的 `<a>` 也不是链接；导航用 a，非导航用 button |
| `<div class="checkbox-active">` | `<input type="checkbox" checked>` 或 `<button role="checkbox" aria-checked="true">` | role 必须配套键盘行为 |
| `<img>` 无 `alt` | `<img alt="...">` 或装饰图 `alt=""` | 没有 alt 屏幕阅读器会读 src 文件名 |
| `<i class="icon-trash">` 作为按钮 | `<button aria-label="删除"><svg aria-hidden="true">...</svg></button>` | 图标按钮必须有可访问名 |
| `<table>` 用于布局 | `<div>` + CSS Grid/Flex | table 仅用于真正的表格数据 |
| 表单字段无 label | `<label htmlFor="email">邮箱</label><input id="email">` | 没 label 的字段不可被 `getByLabelText` 命中，屏幕阅读器无法朗读用途 |

## Landmark：页面骨架

每个页面只能有一个 `<main>`，其它 landmark 按需出现：

```html
<header>     <!-- role="banner"，站点级头部，只能一个 -->
<nav aria-label="主导航"> <!-- role="navigation"，多个 nav 必须用 aria-label 区分 -->
<main>       <!-- role="main"，主要内容，只能一个 -->
<aside>      <!-- role="complementary"，补充内容 -->
<footer>     <!-- role="contentinfo"，站点级页脚 -->
```

特殊：分组但没有专门标签时用 `<section aria-labelledby="...">` 或 `<div role="region" aria-label="...">`，必须带 label 才生效。

## ARIA 标注三件套

| 属性 | 用途 | 何时用 |
|------|------|--------|
| `aria-label="字符串"` | 直接给元素一个可访问名 | 图标按钮、没有可见文本的控件 |
| `aria-labelledby="id"` | 用页面已有元素的文本作为名 | 已有可见标题且不想重复字符串时 |
| `aria-describedby="id"` | 附加描述（不是名字） | 表单错误消息、辅助说明 |

**优先级**：可见 `<label>` > `aria-labelledby` > `aria-label`。可见文本可被翻译、可被搜索、AI agent 也能看到，纯 `aria-label` 不行。

## Live Region：动态内容播报

| 标记 | 语义 | 用例 |
|------|------|------|
| `role="status"` 或 `aria-live="polite"` | 非紧急更新，等空闲再播报 | Toast 成功提示、加载完成、搜索结果数量 |
| `role="alert"` 或 `aria-live="assertive"` | 紧急，立即打断 | 表单提交失败、网络错误、会话过期 |
| `aria-busy="true"` | 区域正在加载，内容暂时不可信 | Skeleton 容器、表单提交中 |

Live region 容器必须**先存在于 DOM**，然后再插内容；初始就插好内容反而不会播报。

## 表单：label 绑定三法

```html
<!-- 1. htmlFor（最推荐） -->
<label htmlFor="email">邮箱</label>
<input id="email" type="email" />

<!-- 2. 包裹（label 内嵌输入） -->
<label>
  邮箱
  <input type="email" />
</label>

<!-- 3. aria-labelledby（label 在视觉上分离时） -->
<h3 id="section-title">联系方式</h3>
<input type="email" aria-labelledby="section-title email-label" />
<span id="email-label">邮箱</span>
```

错误态完整模板：

```html
<label htmlFor="email">邮箱</label>
<input
  id="email"
  type="email"
  aria-invalid="true"
  aria-describedby="email-error"
/>
<span id="email-error" role="alert">请输入合法邮箱</span>
```

## 隐藏元素的三种方式

| 方式 | 视觉 | 屏幕阅读器 | 焦点 | 用途 |
|------|------|-----------|------|------|
| `display: none` / `hidden` 属性 | 隐藏 | 不读 | 不可聚焦 | 完全不存在的内容 |
| `visibility: hidden` | 隐藏 | 不读 | 不可聚焦 | 占位但隐藏 |
| `aria-hidden="true"` | 可见 | 不读 | 仍可聚焦 ⚠️ | 装饰性图标、视觉冗余的文本 |
| sr-only CSS（`position:absolute; clip; w/h=1px`） | 隐藏 | 可读 | 可聚焦 | 仅给屏幕阅读器的提示文本 |

**陷阱**：`aria-hidden="true"` 加在可聚焦元素上是 a11y 红线——用户能 Tab 进去但屏幕阅读器读不到。隐藏交互元素请用 `disabled` 或 `display:none`。

## 模态与对话框

```html
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="dlg-title"
  aria-describedby="dlg-desc"
>
  <h2 id="dlg-title">删除确认</h2>
  <p id="dlg-desc">此操作不可撤销。</p>
  <button>取消</button>
  <button>确认删除</button>
</div>
```

打开时：
- 焦点移入对话框（首选第一个交互元素或关闭按钮）。
- Tab 焦点被陷在内部（focus trap）。
- 背景内容 `inert` 或 `aria-hidden="true"`。
- Esc 关闭，关闭后焦点回到触发元素。

## 标题层级

`h1` → `h6` 不能跳级（不能从 h2 直接到 h4），且页面应只有一个 `h1`。视觉大小用 CSS 调，不要为了"小一点"就用 `h3` 当 `h2`。

## 键盘可达性

- 所有交互必须能用键盘完成。Tab 顺序遵循 DOM 顺序，不要用 `tabindex` 大于 0。
- 自定义控件用 `tabindex="0"`（可聚焦，跟随 DOM 顺序）；明确不参与 Tab 的用 `tabindex="-1"`。
- 焦点环（focus ring）不要全局 `outline: none`。要么保留浏览器默认，要么用 `:focus-visible` 自定义可见样式。

## 与 AI Agent 协作的额外要求

- 关键操作的可点击区域 ≥ 40×40 CSS px（Computer-Use 视觉点击精度有限）。
- 不要把唯一交互入口藏在 hover 之后（"鼠标移上去才出现的删除按钮"）。AI agent 看不到 hover 态；视觉回归也拍不到。
- 同一图标的多个按钮（一行有 5 个"删除"图标）必须用 `aria-label` 区分（`aria-label="删除张三"`）。
