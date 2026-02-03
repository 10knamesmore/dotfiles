# data-testid 设计规范（Agent 输入版）

## 1. 背景与目的

在现代前端工程中：

* `class` 通常由构建工具（CSS Modules / CSS-in-JS / hash）生成，不稳定
* DOM 结构随重构、样式调整频繁变化
* 通过 XPath / 复杂层级定位：

  * 可读性差
  * 可维护性低
  * 查找性能随层级指数上升

**data-testid 的唯一目的：为自动化测试提供稳定、语义化、低耦合的元素定位方式。**

---

## 3. testid 的核心设计目标（按重要性排序）

1. **有效性（Effectiveness）**

   * 能在页面中唯一或明确地定位目标元素
   * 支持列表、表格等多实例场景

2. **安全性（Safety）**

   * 不包含任何敏感信息：

     * 密码
     * 内部 ID
     * 数据库主键
   * 即使用户查看 DOM，也不会泄露风险信息

3. **可维护性（Maintainability）**

   * 人类可读
   * 从 testid 本身就能理解：

     * 在哪个页面
     * 属于哪个区域
     * 表示什么元素
     * 如何与之交互

---

## 4. 基础设计原则（必须遵守）

### 4.1 稳定优先（Stability First）

* ❌ 禁止依赖：

  * 样式
  * 布局
  * 顺序
  * index
* ❌ 禁止：

  * `div-1`
  * `row-3`
* ✅ 仅依赖业务语义

---

### 4.2 语义导向（Semantic Only）

* testid **只为测试而存在**
* 命名必须反映业务含义，而非技术实现

---

### 4.3 唯一明确（Uniqueness）

* 页面级：

  * testid 应唯一
* 多实例场景：

  * 必须明确这是“并列关系”
  * 能够被组合、区分

---

### 4.4 可组合（Composable）

* 支持：

  * 列表
  * 表格
  * 动态渲染
* 允许通过父级 + 子级 testid 组合定位

---

### 4.5 生产安全（Production Safe）

* testid 即使进入生产环境：

  * 不影响功能
  * 不泄露信息
* 是否在 prod 移除是 **优化项，不是前提条件**

### 4.6 低侵入性

增加的 testid 不应该影响现有代码结构、逻辑与样式。

最佳实践，比如原本是个 `<div></div>`，直接加上 `data-testid` 属性即可：

```html
<div data-testid="example.testid"></div>
```

**添加元素应当极其谨慎！！！**

- 如果在中间节点添加额外元素只是为了挂载层次性 testid，那就是违反低侵入性原则，会造成样式改变风险。

---

### 4.7 静态优先

尽可能使用代码中写死的字符串作为 testid，严禁复杂的字符串计算。

一旦你觉得需要添加一个 normalize、toLowerCase、replace 等字符串处理函数来生成 testid，就说明设计不合理，必须重新设计命名方案。

或者查找更合适的静态语义字段。

## 5. 命名规范（非可复用、末端可交互组件）

### 5.1 基本格式

```text
<page>.<component>.<element>.<interactWay>
```

#### 各字段定义

| 字段          | 说明         | 是否必须    |
| ----------- | ---------- | ------- |
| page        | 页面 / 路由名   | ✅ 必须且唯一 |
| component   | 页面内稳定区域或组件 | 可多个     |
| element     | 语义化元素标识    | 可多个     |
| interactWay | 交互方式       | ✅ 必须且唯一 |

---

### 5.2 命名规则

* 层级之间：`.` 分隔
* 层级内部：`-` 分隔
* 命名应 **尽量短，但必须唯一**

---

### 5.3 示例

```text
algotrade.monitorhead.total-profit.value.text
algotrade.table-cell.strategy-order.stock-code.text
algotrade.upload.basket-name.input
```

---

## 6. 纯容器组件（不可直接交互）

### 6.1 命名规则

```text
<page>.<component>
```

### 6.2 示例

```text
algotrade.uploadorder.strategy-order-header
```

---

## 7. 可复用组件的 testid 策略

### 7.1 核心原则

* **测试关心的是“最终呈现的 value / 可操作元素”**
* 组件内部不同 value **必须有可区分的 testid**

推荐方案（优先级从高到低）：

1. **组件 props 中提供唯一语义标识**
2. **组件 props 显式传入 testid**
3. **外层容器 testid + 树形结构定位**

---

## 8. 特殊场景规范

### 8.1 Dialog / Modal

特点：

* 渲染在 root 之外
* 无法通过 DOM 树定位

约定：

* 同一时间只存在一个同类型 dialog
* `title` 唯一

**规范：**

```text
<dialog-title>
```

直接使用 title 作为全局 testid。

---

### 8.2 Dropdown / Popover

特点：

* 渲染在 body / portal 下
* 需要全局搜索

**规范：**

```text
<component>.<pos>.<element>.<interactWay>
```

示例：

```text
dropdown.item.delete.button
dropdown.item.strategy-name.text
```

---

## 9. 何时必须加 testid

**必须加 testid 的元素：**

* 用户可直接操作的元素
* 页面上可见、用于断言的数据
* 一旦出错需要被测试捕获的 UI 节点

---

## 10. 质量门禁原则

* 新增页面 / 组件：

  * **未满足 testability → 不允许合入**
* testid 是：

  * 前端对测试负责的接口
  * 架构层面对可测试性的承诺

---

## 12. Agent 生成 testid 的指导总结（给模型用）

* 永远从 **业务语义** 出发
* 不关心 DOM、class、样式
* 先判断：

  * 页面
  * 是否可交互
  * 是否是 value
* 严格遵循命名层级
* 宁可多写一个语义字段，也不要使用 index

---

**End of Spec**
