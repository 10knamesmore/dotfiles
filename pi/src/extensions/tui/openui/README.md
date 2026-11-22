# OpenUI

在 Pi 的 assistant 正文中，将顶层 `openui` fenced code block 渲染为可交互的终端图表、表格或关系图。渲染能力启用后，Pi 的 system prompt 说明围栏格式。组件签名见 [OpenUI Lang 参考]，交互用法见 [show-me]。

每个 widget 由四角圆弧与上下虚线界定，上边标有 `OpenUI`，内容与 `[ notes ]` 均位于两线之间。边界跟随终端宽度和 Pi 主题，由插件统一绘制，不加左右竖线；激活时使用 accent 色，其余时候保持 dim。

标题栏的 `source` / `widget` 切换源码与渲染结果。`copy` 复制当前视图：渲染视图复制组件的 Unicode 纯文本，源码视图复制原始 OpenUI 源码。`hide` / `show` 收起或展开内容；收起时不显示 `copy`。折叠和源码预览保留当前排序、选项与批注。

复制按钮与正文代码框共享[反馈样式]：悬停或键盘聚焦时使用 accent 色、加粗和下划线；剪贴板写入成功后显示绿色 `-[✓ ok]-`，1.5 秒后恢复。发送按钮显示发送中、排队、已发送或失败；已发送表示 Pi 已接收消息。失败可重试。

Table 支持列头升序／降序排序、点击单元格查看并复制完整原文，以及按当前显示顺序复制含列头的 CSV。CSV 保留单元格空白和换行，使用逗号分隔、双引号转义及 CRLF 行分隔。排序不改变 tooltip 与批注使用的原始行编号。代码和单元格原文在查看时折行，复制不包含视觉折行。

## 使用

通过仓库的 `dots sync` 分发。OpenUI 由 [入口] 加载，向 [editor] 注册代码块渲染。渲染启用时，扩展向 Pi 注册 [show-me] skill。语法或图片校验失败时，正文保留源码并在块下方显示诊断。

| 命令                  | 用途                                                         |
| --------------------- | ------------------------------------------------------------ |
| `/openui-gallery`     | 展示全部组件示例                                             |
| `/openui-gallery dag` | 按前缀选择示例，如 `dag`、`table`、`tabs`、`choice`、`image` |

正文 widget 的按钮发送后续用户消息；Gallery 中的按钮只在本地回显。点击 `[ notes ]` 添加批注，批注先汇入消息框，由用户审阅、编辑并发送。点击交互元素后可用 Tab/方向键移动、Enter 激活、Esc 退出。图片需要支持 Kitty graphics protocol 的终端。

组件源保存在 Pi 会话中；恢复会话时重新绘制，交互状态重置，批注不持久化。

## 维护

源码来自 [pi-openui] 0.2.0，导入基准为 [67aa779]，保留 [MIT 许可证]。本目录直接维护本地修改；更新上游时按需移植。

修改语言或组件签名后，在仓库根目录运行以下命令更新 skill 的参考：

```bash
node --input-type=module -e 'import { reference } from "./pi/src/extensions/tui/openui/src/lang/library.ts"; process.stdout.write(reference())' \
  > pi/src/extensions/tui/openui/skills/show-me/references/openui.md
```

在 `pi/` 下运行：

```bash
pnpm run typecheck:openui
```

[类型配置] 继承 workspace 的严格检查，仅关闭 `noUncheckedIndexedAccess`，以匹配导入代码的数组索引写法。workspace 的 `pnpm run typecheck` 包含这项检查。

[show-me]: skills/show-me/SKILL.md
[OpenUI Lang 参考]: skills/show-me/references/openui.md
[editor]: ../editor/README.md
[入口]: index.ts
[类型配置]: tsconfig.json
[反馈样式]: ../../ui/action-feedback.ts
[MIT 许可证]: LICENSE
[pi-openui]: https://github.com/newtonapple/pi-openui
[67aa779]: https://github.com/newtonapple/pi-openui/tree/67aa779b0574a9e4c35684e69effa1394c317070
