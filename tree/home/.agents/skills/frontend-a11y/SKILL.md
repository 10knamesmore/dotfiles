---
name: frontend-a11y
description: 编写、修改或审查任何前端/UI 代码时必须触发——HTML/CSS/React/Vue/Svelte/组件/页面/表单/交互/弹窗/图表/上传/列表。产出的代码必须可访问(a11y),其最终判据是:浏览器自动化 agent(Playwright / CDP / accessibility snapshot)能通过语义 role + 唯一稳定的 accessible name 定位到每一个可交互或有信息的元素,并能读出状态、感知动态变化。关键词 aria、role、getByRole、getByLabel、accessibility tree、locator、data-testid、landmark、live region。
---

# Frontend a11y —— 让 UI 对 agent 可定位

## 核心原则

**agent 读 UI 只有两条通道,内部 state / React state / class 都不在其中:**

1. **accessibility tree** —— role + accessible name + `aria-*` 状态。agent 用 `ariaSnapshot()` / CDP `Accessibility.getFullAXTree` / `getByRole` / `getByLabel` 读。
2. **DOM 属性** —— `data-state`、`data-testid`、`data-<biz>-id`。agent 用 `toHaveAttribute` / `[data-state="…"]` 属性 locator 读(**这些不进 a11y 树**)。

一个元素若两条通道都读不到,或没有可解析的 **role + accessible name**,自动化 agent 就"看不见"它,点不到、读不到、断言不了。**可访问性不是给屏幕阅读器的附加品,它就是机器可操作性本身。**

判据(自查):**假想一个只拿到 accessibility snapshot + DOM 属性的 agent——它能唯一且稳定地找到这个元素、读出它的状态、感知它的变化吗?不能就没写完。**

各组件的正确 markup(图标钮/loading/表单/dialog/tabs/table/list/上传/图表/进度条…)见 `references/patterns.md`;验证命令见 `references/verification.md`。

## 契约:每次写前端都要满足

### 1. 结构语义 —— 先把骨架和角色立对

- **landmark 骨架**:`<header>`(banner)/`<nav>`/`<main>`/`<aside>`(complementary)/`<footer>`(contentinfo)。**全页只能一个 `<main>`**;多个 `<nav>` 时每个必须 `aria-label` 区分。agent 用 `getByRole('main')`、`getByRole('navigation',{name:'主导航'})` 收窄查询、消解重名。
- **标题层级**:每页只有一个 `<h1>`;`h1→h6` 顺序不跳级;视觉大小用 CSS,别为了"字小一点"降级 heading。agent 用 `getByRole('heading',{level,name})` 按大纲导航。
- **真数据表格**:`<table><caption><thead><th scope="col"><th scope="row"><td>` → 生成 table/columnheader/rowheader/cell/row 角色。**别用 div grid 铺表格**(退化成 generic,读不出行列)。agent:`getByRole('row',{name})`、`getByRole('columnheader',{name:'金额'})`。
- **列表**:同类条目集合用 `<ul>/<ol>/<li>` → list/listitem,agent 才能 `getByRole('listitem')` 计数遍历。(`list-style:none` 只在 Safari/VoiceOver 剥离 role,给 SR 补 `role="list"`;Playwright/Chromium 不受影响。)
- **分区**:要作为查询起点的块用 `<section aria-labelledby="标题id">` 或 `<div role="region" aria-label="…">`;**无名 `<section>` 不进树,等于没分区**。agent:`getByRole('region',{name}).getBy…` 收窄。
- **交互元素**:原生 `<button>`/`<a href>`/`<input>`/`<select>`/`<label>`;**绝不用 `<div>`/`<span>`+`onClick` 冒充**。
- 需作查询起点或重复出现的容器:加 `data-testid`。

### 2. accessible name —— 唯一、稳定、与视觉一致

- 图标/纯图形控件**必须**有 `aria-label`。
- 表单控件用 `<label for>` 或 `aria-labelledby`,**不靠 `placeholder`**。
- **同页同 role 不重名**(否则 `getByRole` strict 报错);重名加上下文:`"Delete"` → `"Delete invoice #12"`。
- **name 不随状态漂移**:loading 别把 `"Subscribe"` 改成 `"Sending"`,固定 name,状态用 `aria-busy`。
- **`aria-label` 不要覆盖可见文本**(WCAG 2.5.3 Label in Name):控件已有可见文字时别再加不同的 `aria-label`——accessible name 会变成 aria-label 值、与屏幕文字不符,`getByRole({name:可见字})` 命中 0。三者取一,优先可见 `<label>` > `aria-labelledby` > `aria-label`。
- `fieldset`/`legend` 提供的是**组名**,**不会**拼进组内字段的 name。两组同名字段(账单城市 / 收货城市)仍要各自 label 带上下文,否则 `getByLabel('城市')` 命中两个。

### 3. 状态可读 —— 两条通道分工

- **aria-\*(进 AX 树)**:`aria-expanded`/`aria-selected`/`aria-checked`/`aria-current`/`aria-pressed`/`aria-disabled`/`aria-busy`/`aria-invalid`/`aria-required`。
- **`data-state`(进 DOM 属性,不进 AX 树)**:统一状态机 `idle|loading|success|error|empty`,**跨状态转换保持在同一元素**,让 agent 用 `toHaveAttribute('data-state','success')` 确定性等终态,而非 `sleep`。`aria-busy` 是布尔(只表达忙/不忙),区分 success/error/empty 必须靠 `data-state`。
- `aria-busy` 放哪:控件级操作(表单提交、单按钮)放该控件;区域加载(列表/表格/面板)放区域根。区域同时是 live region 时,加载全程 `true`、内容落定才 `false`(让 AT 批量播报一次)。
- 必填用 `required` / `aria-required`(别只靠红星号)。**`disabled`**(移出 Tab 序、不参与提交、`toBeDisabled()` 真)vs **`readonly`**(可聚焦可读、照常提交)语义不同,别混用。
- `aria-current` token:`page`/`step`/`location`/`date`/`time`/`true`。Playwright `getByRole` **无 current 选项**,用属性断言 `[aria-current="page"]`。别靠 `.active` class。
- **挂了 role 就得补配套键盘**:tablist/listbox/menu/radiogroup 的通用契约是**方向键切换 + roving tabindex**(仅当前项 `tabindex=0`,余 `-1`);Home/End 仅 slider 强制(跳 min/max),长 listbox 推荐、其余可选;radiogroup 无 Home/End。只挂 role 不补键盘 = 残缺控件。

### 4. 动态内容可感知

- **live region 容器先空存在,再插内容才播报**;别把 `<div role="status">{msg}</div>` 整块条件渲染。idle 时容器常驻且为空,只替换文本子节点——既让 SR 播报,又给 agent 一个永远在场可 poll 的锚点。
- 阻断性错误用 `role="alert"`(assertive);非阻断提示/计数/成功用 `role="status"`(polite)。内联校验在 **blur/提交时**触发,别每次击键就写 `alert`(会刷屏、节点抖动难断言)。
- **表单提交失败**:渲染 error summary(`role="alert"` + `tabindex="-1"`,列出全部错误、每条链到字段)并把焦点移过去。agent 一次 `getByRole('alert')` 读全部错误。
- **toast 自动消失是瞬时信号**:绝不让会消失的 toast 成为某结果的唯一记录——同时落持久 `data-state="success"` / 常驻文案供事后读;toast 本身用自动等待断言,`duration` 可配(自动化下设常驻)。
- **空态**:`data-state="empty"` + `role="status"` 文本播报("未找到匹配结果"),与 loading/error 三态可辨,别只画一张空插图。
- **进度**:`<div role="progressbar" aria-valuenow aria-valuemin aria-valuemax aria-valuetext aria-label>`;不确定进度省略 `aria-valuenow` 并配 `aria-busy`。

### 5. 可达性基础(人 + agent 都受益)

- **焦点可见:永不 `outline:none`/`0` 而不给 `:focus-visible` 替代。**
- 键盘可达:Tab 到达、Esc 关 dialog、打开移焦点进内部、关闭归还、dialog `focus trap` + 背景 `inert`。
- 对比度 AA:正文 ≥ 4.5:1,大字/UI 边界 ≥ 3:1。**颜色不作唯一信息通道**(WCAG 1.4.1):错误别只用红边(配 `aria-invalid` + 文字),状态点/图例配文字或形状——颜色不进 AX 树。
- 尊重 `prefers-reduced-motion`。
- 触控目标 ≥ 24×24 CSS px(computer-use 视觉点击建议 ≥ 40)。
- **`aria-hidden` 别加在可聚焦元素**(能 Tab 进却读不到,红线);隐藏交互用 `disabled` / `display:none`。

### 6. 定位与锚点

- 优先级:`getByRole(role,{name})` > `getByLabel` > `getByTestId` > `getByText`。
- **i18n 例外**:文案随语言/产品微调变动时 **`getByTestId` 优先于 `getByText`**(`data-testid` 可用翻译 key,或 name 用正则匹配多语言)。text 是巧合,testid 是契约。
- **禁止作 locator**:CSS class、库前缀类(`.ant-*`)、`:nth-child`、xpath 索引、CSS-in-JS 哈希类。出现这些 = 语义/name/testid 没做够,回去补,别在选择器侧硬抓。
- testid 命名 `<feature>-<element>-<role>` 全小写 kebab,**不含索引/数据**。列表项:固定 testid + 业务 id 走独立 `data-<biz>-id`(`<li data-testid="todo-item" data-todo-id="42">`)。

### 7. agent 黑洞与框架陷阱(会静默失效,完整清单见 `references/traps.md`)

页面"看起来正常",但下列形态会让 agent 定位/断言莫名空命中或拿错节点。命中其一 → 读 `references/traps.md` 的对应修法:

- **黑洞(整块不进 a11y 树)**:canvas/WebGL、纯 SVG、iframe/shadow DOM、虚拟滚动(视口外行不在 DOM)、拖拽、被样式化按钮盖住的 file input、portal/teleport(脱离触发子树)、`dangerouslySetInnerHTML`。共性修法:提供 DOM 层可定位替代(覆盖层/sr-only 数据/键盘等价通路/从 document 根定位)。
- **框架陷阱(reconciliation/时机)**:`key={index}`(丢焦+拿错节点)、render 内定义子组件(重挂丢焦)、组件库 `loading` 不投射 `aria-busy`(自补)、testid/aria/ref 透传断链、SSR/RSC 首屏就要带状态属性。

### 8. 媒体

- `<img>`:有意义给描述性 `alt`(agent `getByAltText`/`getByRole('img',{name})`);纯装饰用 `alt=""`(空串,**非省略**——省略会读出文件名)且可 `aria-hidden`;带说明用 `<figure><figcaption>`。
- `<video>`/`<audio>`:`<track kind="captions">` + 可见/`<details>` transcript 区。自建播放器每个控件补 role/`aria-label`,进度 `role="slider"`。(原生 `controls` 内部在 UA shadow DOM,Playwright 难可靠穿透,信息靠 captions/transcript。)

## 验证 —— 声明"完成"前必须实际做

跑下列检查并贴输出。完整可粘贴片段见 `references/verification.md`(自包含,`@playwright/test` / `playwright` CLI / CDP)。

1. **accessibility snapshot**:每个交互/信息元素带 role+name;**信息型非交互文本**(计数/状态行/图表 label)也要在树里,不是只塞进 `title`/`aria-hidden`/canvas/`::before`。
2. **locator 唯一性**:关键元素 `getByRole`/`getByLabel` strict 唯一(含表单字段重名);退化成 css/xpath = 缺锚点。
3. **逐态验**:把组件分别驱动进 idle/loading/error/empty 各态验;empty/error 必须有可读文本不能只插图。
4. **状态变化**:动作前断言 live region 空存在,触发后断言出现目标文本(before/after diff,不是一次性静态存在)。
5. **焦点**:Tab 序匹配阅读序、无正 tabindex、dialog 焦点陷内部、焦点可见。
6. **axe**:`@axe-core/playwright` 0 个 critical/serious。
7. **motion**:验证前 `emulateMedia({reducedMotion:'reduce'})` 断言动画停;测试/快照构建全局关动画避免命中时机不稳。
8. 对比度达标。

## 反模式

| 反模式 | agent 后果 | 修 |
|---|---|---|
| `<div onClick>` 当按钮 | 不在交互层,`getByRole('button')` 找不到 | 原生 `<button>` |
| div grid 当表格 | 无 row/cell 角色,读不出行列 | `<table>` + `th[scope]` |
| 图标按钮无 `aria-label` | name 空,无法定位 | 加 `aria-label` |
| 有可见文字又加不同 `aria-label` | name≠视觉字,`getByRole({name})` 命中 0 | 去掉 aria-label 或与可见字一致 |
| loading 改按钮文字 | name 漂移,脚本失配 | 固定 name + `aria-busy` |
| `placeholder` 当 label | name 不稳/读不到 | `<label for>` |
| `outline:none` 无替代 | 焦点丢失 | `:focus-visible` |
| 只用 `data-state`/`aria-busy` 不落 live region | 结果变化 SR/snapshot 感知不到 | 错误/结果进 `role=alert/status` |
| live region 整块条件渲染 | 首次插入不播报、节点是移动靶 | 容器先空存在再填文本 |
| toast 自动消失作唯一结果 | agent 晚一步读不到 | 同时落持久 `data-state` |
| 状态只改 class(`.active`/`.open`) | agent 读不到状态 | 补 `aria-*` / `data-state` |
| 同页多个同名控件 | strict 命中多个 | name 带上下文 / landmark·region 收窄 |
| CSS 哈希 / nth-child / xpath 作 locator | 构建/重排即断 | 回去补 role/name/testid |
| 虚拟列表靠盲滚定位 | 视口外行不在 DOM | 搜索/筛选/翻页 + `scrollIntoViewIfNeeded` |
| file input `display:none` | agent 无法 `setInputFiles` | sr-only 保留 + `aria-label` |
| canvas 画交互 UI | AX 树黑洞 | DOM 覆盖层承接 + sr-only 数据 |
| `key={index}` | 丢焦/locator 拿错节点 | 稳定业务 id |

## 红旗 —— 出现即停,回去改

- `<div>`/`<span>` + `onClick`,或 div 铺表格/列表
- 交互元素无可见文本也无 `aria-label`;或有可见字又被 `aria-label` 覆盖
- `outline:none` / `outline:0`
- 状态只靠 class,没有 `aria-*` / `data-state`
- 异步/toast/错误/空态直接改 DOM,没进 live region(或 live region 整块条件渲染)
- 会消失的 toast 是某结果的唯一记录
- 按钮/链接可读名字随状态改变
- 挂了 role 却没实现方向键/Home/End/roving tabindex
- 仅用颜色表达状态/错误/分类
- canvas/纯 SVG/iframe/虚拟列表/拖拽/file input 无 DOM 层可定位替代
- `key={index}`、render 内定义组件、组件库 loading 未补 `aria-busy`
- 声明"做完"却没跑 accessibility snapshot / 逐态验 / `getByRole` 验证

**任意一条都意味着:agent 无法可靠定位或断言这段 UI,回去改。**
