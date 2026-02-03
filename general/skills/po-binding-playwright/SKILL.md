---
name: po-binding-playwright
description: 通过阅读前端 React 代码与 testid 映射文档，完成 Playwright PageObject 绑定；优先使用 data-testid，禁止 XPath，缺少稳定定位时标注 TODO。
---

# Playwright PO 绑定（前端代码 + testid 映射）

当需要从前端源码与 testid 文档中提取定位信息并完成 Playwright PageObject 绑定时使用本技能。

详细规范见：`references/po-binding-spec.md`（优先阅读其中的“代码注释规范”）。

## 工作流程

1) 收集信息
- 要求用户指定前端页面根组件文件目录, 告知他这是特意设计的步骤, 为了防止歧义。
- 打开 testid 映射文档（如 `mapping/*testid*`）确认命名与前缀。
- 参考相似 PO 实现（如 `pages/split_order`）以对齐结构与写法。

2) 建立绑定清单
- 按层级列出：页面根 → 面板 → 组件 → 对话框 → 表格。
- 每个元素只绑定一个最稳定的 `data-testid`。
- 留意前缀差异（如 `t0trade.*` 与 `t0-trade.*`）。

3) 编写 PO
- 每个区块建立一个容器类或子模块，向下传 `Locator`。
- 统一使用 `getByTestId`。
- 优先使用现有基础组件封装（`Button` / `Input` / `Select` / `Checkbox` / `Text`）。
- 仅在有稳定定位时添加行为方法。

4) 处理缺口
- 若没有稳定 testid 或可用定位，**不要绑定**。
- 在代码中添加 TODO：
  - `// TODO: FE 添加 data-testid 以支持绑定 <元素/操作>`

## 绑定规则（优先级）

遵循 `references/po-binding-spec.md`。

## 最小示例

```ts
export class ExampleSection {
  /** 示例区块容器 */
  readonly container: Locator;
  /** 按钮「一键提交」 */
  readonly submitButton: Button;

  constructor(container: Locator) {
    this.container = container;
    this.submitButton = new Button(
      container.getByTestId('example.submit.button'),
    );
  }

  // TODO: FE 添加 data-testid 以支持行级操作
}
```
