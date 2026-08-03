---
name: frontend-a11y
description: 编写、修改或审查真正接入项目运行路径的前端 UI component 时使用。唯一目的：要求组件把 semantics 和实际支持的每个 state 标注成 accessibility tree / ARIA 可读的 public contract，使 test 能直接用 getByRole/getByLabel 和 ARIA state assert。不适用于临时 HTML、scratch、mockup、prototype、架构图、解释性 visualization、一次性 demo 或其他不进入 application runtime、route、entry、bundle、production import chain 的展示产物；这些产物无需通过本 skill 验收。不负责业务行为、端到端流程或业务结果测试。
---

# Frontend component a11y assertion contract

## 核心规则

真正生效的 UI component 必须把对外的 semantics 和 state 标注成 test 可直接 assert 的 a11y contract：

1. **Semantics**：用正确的 role、accessible name、label 和 semantic scope 标明元素是什么。
2. **State**：用 `aria-*`、status、alert、progressbar 或命名区域标明组件当前是什么状态。
3. **Assertion**：test 只读取这套 public DOM / accessibility contract，不读取 React/Vue 内部 state，不依赖 class、CSS、DOM 位置或实现细节。

完成标准：目标元素能由 `getByRole` / `getByLabel` strict 唯一定位，并且组件实际支持的每个 state 都能通过对应 ARIA state 或语义节点直接 assert。

本 skill 只保证组件对 test 暴露稳定的语义与状态，不代表完整 accessibility audit，也不代表业务行为正确。

## 适用范围

只对真正进入项目运行路径的前端实现应用本 skill。至少满足下列一项：

- 被 production source import，并最终进入 application bundle。
- 被实际 route、entry、layout、page 或 runtime component 引用。
- 修改后会影响用户实际运行的 UI，而不是只影响说明材料或临时产物。

不触发本 skill 的内容：

- 临时 HTML、scratch file、一次性 script 生成的页面。
- 仅用于讨论方案的 mockup、prototype、wireframe 或 UI 草图。
- 仅用于解释架构、流程或数据关系的 visualization、diagram、个人理解页面。
- 不被项目 import、route、entry、build 或 runtime 使用的一次性 demo 和 artifact。

不要根据 `.html`、`.tsx`、`.vue` 等扩展名直接判断。拿不准时先用 import、route、entry、build config 和实际 consumer 查证文件是否生效。临时产物后来接入项目运行路径时，从接入开始应用本 skill。

## 明确排除

触发本 skill 不得自行扩大到以下工作：

- 不新增或运行业务级 unit、integration、e2e 测试。
- 不为验证业务正确性构造完整 API mock、测试账号或业务数据。
- 不驱动提交、下单、删除、上传等业务流程来断言结果。
- 不要求 axe、WCAG、keyboard、focus trap、contrast、reduced motion 或 screen reader announcement 验证。
- 不把 `data-testid`、CSS selector、XPath 或 DOM 顺序当成 a11y locator 已完成的证据。

用户另行要求这些工作时，按对应任务或 skill 处理；不要归因于 `frontend-a11y`。

允许使用已有 Story、component props 或最小 state harness 直接渲染组件实际支持的状态；它们只用于检查 a11y representation，不得扩展为业务行为测试。

## Component annotation contract

### Role 必须正确

- 优先使用原生元素：`button`、`a[href]`、`input`、`select`、`textarea`、`table`、`dialog`。
- 自定义组件只在没有合适原生语义时声明 ARIA role。
- 不为让 locator 命中而添加虚假的 role；role 必须和元素实际用途一致。
- 组件封装必须把 `aria-*`、`id`、`htmlFor` 和必要 props 传到真正的交互元素，不能落在无关 wrapper。

### Accessible name 必须稳定且可消歧

- 优先使用可见文本或关联的 `<label>`。
- 纯图标控件使用 `aria-label`；已有可见文本时不要用不同的 `aria-label` 覆盖它。
- name 不随 loading、selected、expanded 等状态切换而改成另一套动作名称。
- 同 role 同 name 的目标通过业务上下文消歧，或先用有名字的 dialog、region、group、table、row 等语义容器收窄 scope。
- 不用 CSS class、位置或随机 ID 制造唯一性。

### 每个实际 UI state 都必须可定位

对组件实际支持的 idle、loading、success、error、empty 及其他状态逐态检查。不存在的状态不必创造；存在的状态不能只验证 happy path。

只验证该状态投射出的 role、accessible name、ARIA state 和可读信息，不验证状态为何发生、业务 transition 是否正确或后端结果是否真实。

需要时暴露对应 ARIA state，例如：

- `aria-selected`
- `aria-checked`
- `aria-expanded`
- `aria-pressed`
- `aria-current`
- `aria-disabled`
- `aria-busy`
- `aria-invalid`

各态的 locator contract：

| State | 必须能通过 a11y 找到的内容 |
|---|---|
| idle | 主要控件与区域，role/name 唯一且稳定 |
| loading | 原目标 name 不漂移；busy region、progressbar 或 status 可定位 |
| success | success status 或结果区域可定位 |
| error | alert、invalid field 或错误区域可定位 |
| empty | empty status 或命名区域可定位，不能只有无语义插图 |

### A11y tree 黑洞必须提供语义入口

canvas、WebGL、纯图形 SVG、closed shadow root、未渲染的虚拟列表项等不能直接提供目标节点时，为需要操作或读取的目标提供真实 DOM 语义入口。不要用坐标点击、CSS hash、`nth-child` 或 XPath 掩盖缺失的 a11y 节点。

## Locator 选择

只接受以下 a11y locator 作为主要完成证据：

1. `getByRole(role, { name })`
2. `getByLabel(name)`
3. 从有 role 和 name 的语义容器开始，再向下使用前两者

示例：

```ts
page.getByRole('button', { name: '保存' })
page.getByLabel('邮箱')
page
  .getByRole('dialog', { name: '确认删除' })
  .getByRole('button', { name: '确认' })
page
  .getByRole('row', { name: /订单 #42/ })
  .getByRole('button', { name: '删除订单 #42' })
```

`getByText`、`getByTestId`、CSS selector 和 XPath 不属于本 skill 的完成标准。若 a11y locator 无法唯一命中，先修 role、name 或 scope。

## 最小验证

声明完成前只验证 locator contract，不验证业务结果：

1. 查看目标所在 subtree 的 accessibility snapshot，确认目标 role 和 name 非空且正确。
2. 对目标 `getByRole` 或 `getByLabel` 执行 `toHaveCount(1)`，确认唯一命中。
3. 如果使用语义 scope，分别确认 scope 与 scope 内目标都唯一命中。
4. 用已有 Story、component props 或最小 state harness 渲染组件实际支持的每个状态，逐态重复前三项并检查相应 ARIA state。
5. 只断言 a11y representation，不触发完整业务流程，也不把业务结果作为通过条件。

使用项目已有的 browser automation 环境。不得仅为本 skill 安装测试框架、建立业务测试文件或补齐测试数据。

组件 locator 映射见 `references/patterns.md`；locator 空命中、重复命中或 detached node 时见 `references/traps.md`；最小验证片段见 `references/verification.md`。

## 红旗

- 需要 `data-testid`、CSS class、`nth-child` 或 XPath 才能找到目标
- role 正确但 accessible name 为空、漂移或在当前 scope 内重复
- `aria-*` 落在 wrapper，真正的交互元素没有语义
- 用虚假 role 只为满足 locator
- 组件实际支持多个状态，却只验证 idle 或 happy path
- loading 时 name 漂移，或 success/error/empty 没有可定位的 status、alert 或命名区域
- 为证明 a11y locator 可用而开始编写或执行业务流程测试
- 把 axe、keyboard、focus、contrast、motion 或业务 transition 检查宣称为本 skill 的必要验收

出现任一项时，只修 locator contract；不要扩大到业务测试。
