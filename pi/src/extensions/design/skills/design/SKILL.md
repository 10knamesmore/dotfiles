---
name: design
description: 创建和迭代产品 UI、可交互原型与多方案设计；读取用户的画布编辑、调参和位置评论，修改 React 项目并检查效果。不用于临时图示或一次性可视化。
---

# Design

## 新建设计

1. 从用户的自由文本确定设计目标。按请求选择项目目录；未指定时放在
   `~/Documents/` 下。
2. 阅读 [React 组件与调参](references/react.md) 和
   [项目文件](references/project.md)。使用普通 React/Vite 工程，
   将可直接编辑的内容绑定到运行时组件，为作品提供有意义的参数。
3. 用现有文件工具编写项目，使用 pnpm 安装第三方依赖。
4. 安装完成后注册并打开：

   ```python
   import pi_design
   project = pi_design.open("/absolute/project/path", name="项目名称")
   ```

5. 用 browser-use 检查实际页面和交互。多个方案放在不同画板上比较。

## 继续设计

先读取当前作品，不凭上一轮截图或上下文快照还原用户的调整：

```python
import pi_design
project = pi_design.current()
state = project.read_state()
print(state)
```

关注：

- `design.elements`：用户已经改过的文字和样式。
- `design.parameters`：当前参数值。
- `workspace.comments`：评论及其附着的元素或画板坐标。
- `selection`：用户当前看的画板和选中的元素。

按任务读取 [Python SDK](references/python.md) 中的局部查询方法。
需要新增或改变可编辑组件时，读 [React 组件与调参](references/react.md)。

## 修改与检查

- 直接用 `read` / `apply_patch` 修改 React、CSS 和设计 JSON。文件变化会
  更新工作区，不需要再调用提交或 reload。
- 手动编辑和调参已经生效。保留这些值及稳定的元素 ID，在此基础上继续设计。
- 元素评论按元素定位；坐标评论按对应画板中的 CSS 像素定位。
  用 browser-use 查看评论附近的画面，确认后再改。
- 查看 `project.url` 对应的工作区，或 `state["preview_url"]` 对应的应用。
  在运行模式验证按钮、导航和状态切换。
- 完成并检查一条评论要求后，调用 `project.resolve_comment(comment_id)`。

交付时给出作品位置和可打开的地址，说明完成的改动以及尚未处理的意见。
