---
name: show-me
description: 需要核对 OpenUI 组件签名、选择组件或设置交互时查阅。需要查看网页实际布局时使用 HTML。
---

在 Pi 对话里用 OpenUI 展示分支流程、多方调用、复杂结构和数据对比。简单的单向关系或几行可直接使用的代码，直接写在正文里。

## OpenUI

按用户要看清的内容选择组件：

- **顺序与结构**：谁在什么时候调用谁，用 `SequenceDiagram`；步骤怎样分支、模块如何依赖，用 `Dag`。服务部署与连接用 `InfraMap`，数据实体与字段关系用 `Erd`。
- **比较与数据**：逐项比较方案或查具体值，用 `Table`；比较数量用 `BarChart`，观察随时间的变化用 `LineChart`。

需要核对语法或组件签名时，查阅 [OpenUI Lang 参考](references/openui.md)。

沿用 Pi 主题，用中文标注，只保留回答当前问题需要的节点、连线和数据。图旁用一两句话指出值得看的位置或结论。需要补充局部细节时用 `Tooltip`；需要用户指出具体节点或单元格时，提示点击 `[ notes ]` 批注。`Button` 可发起后续请求；选项需要结合图表判断时用 `Choice`，由用户点击 `Send answers` 提交，单独收集决定用 `ask_user_questions`。

## HTML

需要查看真实网页的颜色、间距、布局或视觉状态时，用实际名称和数据制作桌面端 dark theme HTML。macOS 用 `open`、Linux 用 `xdg-open` 打开；需要检查页面或继续交互时，先读取 `browser-use` skill。
