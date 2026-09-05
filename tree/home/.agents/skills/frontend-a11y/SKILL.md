---
name: frontend-a11y
description: 为接入应用运行路径的前端组件提供可用 role、label 和 ARIA state 断言的语义。排除临时展示产物，不负责业务测试或完整无障碍审计。
---

# Frontend Component A11y Contract

组件的真实用途和实际支持的状态必须能通过 accessibility tree 读取，用 `getByRole` / `getByLabel` 唯一定位，并通过 ARIA state 或语义节点断言。测试不读取框架内部 state、class 或 DOM 位置。

## 范围

仅对进入 production import、route、entry 或 application bundle 的实现应用。临时 HTML、mockup、prototype、架构图和解释性 visualization 不适用；它们接入应用后再应用。判断不明时查实际 consumer，不按文件扩展名触发。

本 skill 只验证语义与状态的表达，不验证业务结果，也不等于完整 accessibility audit。不得因此新增或运行业务级 unit/integration/e2e 测试、搭建 API mock、账号或业务数据、驱动提交等业务流程，或要求 axe、WCAG、keyboard、focus、contrast、motion、screen reader announcement 验证。用户另行要求时按对应任务执行。

## 组件约定

- 优先原生元素；自定义 role 必须符合实际用途，不能为 locator 添加虚假语义。封装组件把 ARIA、id、label 关联和相关 props 传到真正的交互元素。
- 名称优先来自可见文本或 label；纯图标控件补 `aria-label`，不覆盖已有文本的真实含义。loading、selected、expanded 等状态不把原目标改名为另一套动作；同名控件通过有业务含义的语义容器消歧，不依赖随机 ID 或位置。
- 对实际存在的 idle、loading、success、error、empty 等状态暴露合适的 role、name、ARIA state 或可读信息。不存在的状态不创造，存在的状态不只检查 happy path。
- canvas、WebGL、纯图形 SVG、closed shadow root 或虚拟列表不能直接提供所需目标时，为需要操作或读取的目标提供真实 DOM 语义入口，不用坐标或 XPath 掩盖缺失。

## 验证与参考

使用项目已有浏览器验证环境。查看目标 subtree 的 accessibility snapshot，用 `getByRole` / `getByLabel` 验证目标及必要 scope 唯一命中；通过已有 Story、props 或最小 state harness 渲染实际支持的每个状态并检查其表达，不搭建业务流程。

- 选择元素语义或消歧方式时读 [patterns.md](references/patterns.md)。
- 需要状态表与具体验证命令时读 [verification.md](references/verification.md)。
- 空命中、重复命中或 detached node 时读 [traps.md](references/traps.md)。

`getByText`、`data-testid`、CSS 和 XPath 不作为本 skill 的完成证据。语义缺失时修 role、name 或 scope，不扩成业务测试，也不为本 skill 安装测试框架；验证环境缺失时如实报告未验证。
