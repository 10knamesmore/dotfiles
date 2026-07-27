# 陷阱清单:会让 agent 静默失效的 UI 形态

两类:**A. agent 黑洞**——整块 UI 不进 a11y 树、agent 看不到;**B. 框架陷阱**——reconciliation / 渲染时机让 locator 或焦点悄悄失效。共同点:页面"看起来正常",但 agent 定位/断言会莫名其妙地空命中或拿错节点。

## A. agent 黑洞(需提供 DOM 层替代)

### canvas / WebGL / 纯图形渲染
`<canvas>`/WebGL 画的图表、编辑器画布、地图、白板,在 AX 树里是**无子节点的黑洞**。修:
- 交互点(可点的 bar/point/图例/热区)用真实 DOM 覆盖层(`<button>`/`<a>`)承接点击 + name,canvas 只做绘制;别指望 agent 点 canvas 内部坐标。
- 纯展示画布给 `role="img" aria-label="…"` 概述,或旁挂 `sr-only` 的 `<table>`/`<ul>` 承载同一份数据供 agent 读(见 `patterns.md` 的 canvas 替代)。

### 纯 SVG
- 信息型:`<svg role="img" aria-labelledby="t"><title id="t">下降趋势</title>…</svg>`(`aria-labelledby` 指 title 比裸 `<title>` 跨浏览器稳)。
- 装饰型:`aria-hidden="true"` + `focusable="false"`(旧 Edge/IE 下 SVG 默认可聚焦,会把不可读节点带进 Tab 序)。

### iframe / shadow DOM
- iframe 内容需专门穿透:Playwright `page.frameLocator('iframe[name=…]').getByRole(…)`。
- open shadow DOM:Playwright locator 默认穿透,但要清楚组件边界(closed shadow root 无法穿透——别用 closed 承载可交互内容)。

### 虚拟滚动 / 窗口化列表
react-window / TanStack Virtual / AntD `Table virtual` 只渲染视口内行,**视口外行不在 DOM**,`getByRole` 命中 0、滚动位置不同命中不同、快照不可复现。修:
- 首选给列表**搜索/筛选/翻页**入口(带 role+name),让 agent 用过滤而非盲滚定位目标行。
- 需要滚动时 `await row.scrollIntoViewIfNeeded()` 触发渲染再操作。
- 行进入 DOM 后带 `role="row"` + 稳定 `data-<biz>-id`;总数从 `aria-rowcount` 或计数文本读,别数 DOM 行。
- 文档注明该列表是虚拟化的。

### 拖拽(drag-and-drop)
原生 `draggable` 或 pointer 拖拽序列对 snapshot agent 不可见、对 computer-use 极不稳(需精确坐标轨迹)。修:
- **必给非拖拽等价通路**:每个可拖项旁给「上移/下移/移到…」按钮(带 role+name),或选择目标的下拉。
- 排序/进度/数值用原生 `<input type="range">`(role=slider,自带 `aria-valuenow/min/max`,键盘可调),别用纯拖拽 div。
- 放置结果投射到 DOM(`data-state` + `aria-live` 播报「已移动到 X」)。
- `aria-grabbed`/`aria-dropeffect` 已在 ARIA 1.1 deprecated、1.2 移除,**别用**。

### file input 被样式化按钮盖住
普遍把真 `<input type="file">` 用 `opacity:0`/`display:none` 藏起来只露好看的按钮。agent 点按钮会触发原生文件选择器(无法自动化)。修:
- 真 input **始终留在 DOM**(用 sr-only/`opacity` 而非 `display:none` 移除)+ `aria-label`,agent 直接 `getByLabel('上传头像').setInputFiles(…)` 注入,无需点按钮。
- 触发按钮与 input 用 `<label>` 关联;上传中/结果投射 `aria-busy` + `role="status"`。

### portal / teleport
Modal/Select/Tooltip/Popover/Toast 常经 `createPortal`(AntD/Radix/MUI)或 `<Teleport>` 挂到 `document.body` 末尾,脱离触发它的 DOM 子树。后果:按父容器 scope 的定位(`within(container)`、从 trigger 父节点往下找)全落空。修:
- 定位从 document 根走:`page.getByRole('dialog',{name})` 命中后再下钻;Testing Library 用 `screen.*` 不用 `container.querySelector`。
- 写组件时 portal 根必带 role + accessible name(`aria-labelledby` 指标题),跨子树关联用稳定 id。

### dangerouslySetInnerHTML / v-html
注入的原始 HTML 绕过语义化:里面可能是 `<div onclick>` 假控件、无 name、无正确 role,不在交互层。修:
- 尽量不用;必须用则 sanitize(DOMPurify)+ 容器标 `data-content-html="true"`,别在其中放命名锚点或依赖 role/testid;交互控件用真实组件渲染,不塞 innerHTML。

## B. 框架陷阱(reconciliation / 时机)

### 列表 key 用不稳定值
`key={index}` / `key={Math.random()}`:列表增删/排序/过滤后框架按位置复用同一 DOM 节点承载不同数据——受控 input 的 value 与焦点串行、行内关联 id 错乱;Playwright 已解析的 locator 可能指向被复用的旧 detached 节点。修:`key` 一律用稳定业务 id(`key={item.id}`、AntD `rowKey="id"`、Vue `:key="item.id"`);业务 id 缺失让后端补,别用位置。

### 在 render 内定义子组件
把子组件定义写在父组件函数体内 → React 视为新组件类型,每次 render 卸载旧子树重挂 → 受控 input 每次输入后失焦、内部 state 重置。agent 连续 `type()` 落空或 node detached 报错。修:组件定义提到模块级;别在 render 里现造组件或现造对象 props。同理别把会变的值(时间戳)当受控 input 的 `key`。

### 组件库 loading prop 不投射 aria-busy
AntD `<Button loading>`、MUI `<LoadingButton>`、AntD `<Select loading>`/`<Table loading>`/`<Spin>` 通常只加视觉 spinner + `disabled`,**不投射 `aria-busy` 到 DOM**——Playwright `toHaveAttribute('aria-busy','true')` 与屏幕阅读器都等不到忙态。修:消费侧自补 `aria-busy={loading || undefined}`(`undefined` 避免渲染 `aria-busy="false"` 噪音);区域级用外层 `<div data-state aria-busy>` 包;库只给 `disabled` 时 agent 退用 `toBeDisabled()`。

### 透传断链
`data-testid`/`aria-*`/`ref` 落在 wrapper 而非真正的交互 DOM,导致锚点空命中。高发:AntD `Form.Item` 的 testid 落外层 div 到不了 `<input>`(写在子 `<Input data-testid>` 上);Radix/shadcn `asChild`/Slot 把 `aria-*`/`onClick`/`ref` clone 到 child,child 若不 `forwardRef` + `{...props}` 透传则静默丢失。修:testid 写在真正交互子节点;自封装组件根元素一律 `forwardRef + {...rest}` 透传;改完用 snapshot / DevTools 确认属性真到了交互节点。

### SSR / RSC 首屏状态
Server Components / SSR 首屏是纯 HTML,hydration 前没有 JS。状态若只在客户端 effect 里补 `aria-*`,读首屏 HTML 或 hydration 前截图的 agent 读不到。修:服务端渲染出的 HTML 直接带 `data-state`/`aria-*`;Suspense `fallback` 用带 name/testid 的骨架(`<div role="status" aria-busy="true" data-testid="x-loading">加载中</div>`),别用无名 skeleton;错误走 `error.tsx`/ErrorBoundary 渲染 `role="alert"`。Nuxt/Vue SSR 同理。
