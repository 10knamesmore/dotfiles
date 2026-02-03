---
name: page-behavior-indexer
description: 从 React/类 React 前端源码中抽取页面级业务语义交互单元，并输出可丢弃的结构化页面行为说明（YAML/Markdown），包含触发方式、参数、可观察结果与置信度。用于分析指定页面/目录、整理业务动作、为 DSL/Playwright 测试生成提供输入或在改动后重生成页面行为说明。
---

# 页面行为索引器

## 概览

将前端源码转成一份结构化的业务语义交互清单，包含触发、参数、可观察结果与置信度。

## 工作流

### 1) 确定页面范围

- 定位页面入口组件与相关子组件（同目录 + 直接依赖）, 在这里你需要要求用户指出页面根组件文件目录, 这是流程的一份, 用以减少歧义。
- 关注数据来源（query/store）与变更点（mutation/dispatch/submit handler）。
- 范围越明确越好：一个页面或一个页面目录。

### 2) 识别交互单元

- 抽取业务语义交互，不要陷在底层 UI 手势。
- 用 `references/interaction-types.md` 选择类型。
- 包含：提交/创建/保存，状态变更（暂停/恢复/取消），导航/打开弹窗。
- 同一业务语义的多种 UI 入口合并为一个单元。

### 3) 提取触发方式

- 记录事件类型（click/submit/confirm/auto）与 UI 元素（按钮/表单/菜单）。
- 能看到 handler/function 名称就写上（如 `onSubmit`、`handleCreate`）。
- 若为自动触发，事件标 `auto` 并简要说明。

### 4) 提取参数

- 来自表单、输入、URL 参数、当前选择/上下文或 state 的业务参数。
- 尽量对齐 API payload 或领域对象字段。
- 推断出来的参数标为可选或未知，并降低置信度。

### 5) 提取可观察结果

- UI 结果：toast、弹窗关闭/打开、列表新增、状态文本变化、跳转。
- 业务结果：代码中明确可见的请求/状态设置/缓存刷新。
- 不要编造后端逻辑，未知写明。

### 6) 标注置信度

- 按 `references/confidence-rules.md` 标 high/medium/low。
- 基于命名推断时要保守降级。

### 7) 输出结构化结果

- 用 `references/output-schema.md` 的字段规范。
- 默认输出 YAML；若用户要阅读视图，再补 Markdown。
- 可丢弃、非事实源：不保证 100% 正确。
- 输出的文件放到 page_behavior_index目录, 这个目录可能是一个软链接, 写入前需要向用户确认

## 输出要求

- 聚焦业务语义交互，不做完整状态机。
- 只要 UI 可观察就可以输出，业务结果允许不完整。
- 不确定性写在 `notes`，并体现在置信度上。

## 非目标

- 不构建完整状态机。
- 不保证业务逻辑绝对正确。
- 不作为前端或测试的事实源。

## 参考

- 生成输出时加载 `references/output-schema.md`。
- 判断置信度时加载 `references/confidence-rules.md`。
- 分类交互类型时加载 `references/interaction-types.md`。
