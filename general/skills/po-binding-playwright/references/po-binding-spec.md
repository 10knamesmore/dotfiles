# PO 绑定规范（详细版）

## 绑定优先级

1) **data-testid**（必须首选）
2) **其他稳定属性**（仅在前端代码明确存在时使用）
3) **文本/角色定位**仅在 UI 固定且无 testid 时允许

**禁止 XPath。** 若只能使用 XPath，则跳过绑定并标注 TODO。

## 代码注释规范（新增）

- PO 的**每个组件/locator**必须添加文档注释（JSDoc/TSDoc）。
- 注释需说明该元素在页面上的位置或用途。
- 说明文字优先使用**页面上已有的用户可见标识**（例如：标题文本、按钮文案、面板名称）。

示例：
```ts
/** 下单交易面板标题「下单交易」 */
readonly title: Locator;

/** 按钮「一键提交」 */
readonly submitButton: Button;
```

## po模板

所有 PO 对象类遵循以下模板
```ts
export class Xxx {
  readonly page: Page // 页面本身(如果这个组件需要)
  /** 根容器 */
  readonly container: Locator;

  /** 子元素 / 子组件 */
  readonly xxx: Locator;
  readonly yyy: Locator;
  readonly zzz: Button; // 一种自定义的组件

  constructor(container: Locator) {
    this.page = container.page();
    this.root = container;
    this.container = container.getByTestId('...');
    this.yyy = this.xxx.locator('...');
    this.zzz = new Button(this.container.getByTestId('...'));
  }

  /** 其他业务方法 */
  async doSomething() {}
}
```

## 对话框绑定规则

- `CustomDialog` 的根容器为 `data-testid={title}`。
- 统一绑定以下通用按钮：
  - 确认：`dialog-confirm-button`
  - 取消：`dialog-cancel-button`
  - 关闭：`dialog-close-button`（如需要）

## 表格绑定规则

- 仅绑定 `*.table-cell.*` 级别的 testid。
- 如果没有行级 testid，不要通过 XPath 推导行操作, 可以通过css选择tr, 每一行作为一个Row PO对象, 在根据每一行的唯一key(比如id)筛选操作。

## TODO 规范

当绑定缺失或不稳定时：
- 不做绑定
- 添加 TODO：
  - `// TODO: FE 添加 data-testid 以支持绑定 <元素/操作>`

## 页面目录结构规范

对于一个 FOO 页面 po 对象, 应该目录结构如下:

|- index.ts // 只做导出
|- FOO.ts // 页面对象定义
|- types.ts // 页面对象相关类型定义, 要求是整个页面所有子元素都共享的类型放这里
|- parts/ // 页面子组件目录
   |- BAR/  // FOO 页面中的 BAR 组件定义
        |- BAR.ts // BAR 组件定义
        |- types.ts // BAR 组件相关类型定义
   |- BAZ/ // FOO 页面中的 BAZ 组件定义
