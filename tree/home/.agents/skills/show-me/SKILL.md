---
name: show-me
description: 用图、表格和代码预览讲清当前问题；按内容选择对话内字符图、Pi OpenUI 交互组件、Neovim tour 与只读 diff，或展示更细致gui的 HTML。
---

用中文说明，使用 dark theme；终端组件沿用当前 Pi 主题。围绕当前问题选择足以讲清关键点的最小视图，直接展示，说明简短。

### 展示方式

- 几行文字、小表格、伪代码或字符图就能讲清楚时，直接在对话中展示。
- 独立的关系图、时序图、数据表或图表，尤其需要 tooltip、点击或定点批注时，使用 Pi 的 `render_widget`。工具可用时按下文 OpenUI 流程调用。
- 需要对照真实源码、跨文件导航或结合编辑现场讲解时，使用 Neovim tour。先读取 `nvim-use` skill，按 showcast 流程发布站点；代码窗口展示相关代码，站点 comment 用 fenced code block 放图或结构草图，配简短中文说明。
- 展示具体代码的拟议修改时，使用下文的原 buffer diff 预览；概念上的结构对比可以直接展示 diff 草图。
- 需要真实网页的颜色、间距、布局或视觉状态对比时，使用 HTML。

### OpenUI 交互展示

`render_widget` 把 OpenUI Lang 绘制在 Pi 对话中，不需要浏览器。它提供表格、指标、图表、DAG、ER 图、基础设施图、时序图，以及 Tabs、按钮、选择题、图片和批注。

1. 首次绘制前调用 `render_widget` 读取组件签名：

   ```json
   {"reference": true}
   ```

2. 按返回的签名编写源码，再调用 `render_widget`：

   ```json
   {"source": "root = Stack([summary])\nsummary = Table([\"服务\", \"状态\"], [[\"API\", \"运行中\"], [\"Worker\", \"等待任务\"]])"}
   ```

   使用插件返回的语言子集和位置参数；不要套用官方 React 组件库或假定支持 `$` 状态、`Query`、`Mutation`。按当前问题组织内容；相关视图可以放进 Card 或 Tabs，不为填满组件而添加内容。

3. 工具报告错误时，按行号和错误信息修改，并提交完整源码。绘制后补充必要结论，不重复抄写图表数据。

交互按用途选择：

- **Tooltip**：放节点说明、数据来源或局部细节，让主视图保持简洁。
- **Button**：点击后发送一条后续用户消息；收到后执行该请求，需要更新展示时绘制新组件。
- **Choice**：仅当选项需要结合图表理解时使用。点击先暂存，同一组件内的答案通过 `Send answers` 一次提交；单纯收集决定用 `ask_user_questions`。
- **Notes**：需要用户指出具体节点、单元格或图表位置时，提示点击 `[ notes ]`。批注会带上目标标识；汇入消息框后由用户审阅并发送。

点击 widget 内的交互元素后，可用 Tab/方向键移动、Enter 激活、Esc 退出。`/openui-gallery` 查看示例，示例按钮只本地回显。恢复会话会重画组件，但交互状态与批注不持久化。

### 选择内容形状

- Show logic or an algorithm as pseudocode:

```text
on(save)
  if content is unchanged
    return cached result
  write new content
  return fresh result
```

- Show runtime control flow as a call tree:

```text
submitForm
  createSession
    persistPrompt
    launchAgent
  navigateToSession
```

- Show UI structure as a component tree, including state and module boundaries that matter:

```tsx
<SessionPage> (apps/example/src/routes/session.tsx)
  useSessionEvents()
  <SessionToolbar>
    <RunSkillButton> (packages/ui)
```

- Show file responsibility or a broad refactor as a shallow file tree:

```text
src/
├── commands/       # parses user actions
├── sessions/       # owns session state
└── transport/      # sends API requests
```

- Show component interaction, control flow, or data flow with OpenUI diagrams, or use character diagrams directly in chat or alongside source in a tour:

```text
User            UI                 Daemon
 │               │                    │
 ├─选择命令──────►│                    │
 │               ├─发送展开后的提示──►│
 │               │◄────流式结果───────┤
```

- Use `diff` sketches in chat or tour comments when comparing conceptual structures. For concrete source edits, use the inline preview below. Match the diff shape to the topic.

For a component change:

```diff
 <SessionPage>
   useSessionEvents()
   <SessionToolbar>
+    <RunSkillButton />
   <SessionTimeline>
+    <SkillResultCard />
```

For a file-layout change:

```diff
 src/
 ├── commands/
+│   └── show-me.ts       # expands the slash command
 ├── sessions/
-└── transport.ts
+└── transport/
+    ├── client.ts
+    └── stream.ts
```

For a call-tree or call-stack change:

```diff
 submitForm
   createSession
     persistPrompt
+    expandSkillMention
     launchAgent
-  navigateToSession
+  navigateToSession
+    subscribeToEvents
```

For a state or control-flow change:

```diff
 on(save)
-  write content
+  if content is unchanged
+    return cached result
+  write new content
+  invalidate cache
```

- Show the whole block when most of it is new, when omitted context would hide ownership or order, or when the user needs a copyable target shape:

```ts
function expandSkill(command: string): string {
  const skillName = command.slice(1)
  return `use the ${skillName} skill`
}
```

### Neovim 中的字符图与 diff 预览

流程图、时序图放进 tour 时使用字符图，不转图片。Mermaid 可作为制图输入，先用 `mmdflux --format text --color off` 转成 Unicode 图，或用 `--format ascii --color off` 转成纯 ASCII 图，再放进站点的 `text` 代码块；不要依赖 Neovim 渲染 `mermaid` 代码块。按窗口宽度精简或拆分，避免折行破坏结构。

先读取 `nvim-use` skill，把拟修改后的完整文本放入 tour 站点的 `proposed_text`。以当前 buffer（包括未保存内容）为基准；进入站点显示对应 diff，离站清除，结束 tour 一并清理。删除行高亮，新增行以虚拟行展示；告知用户 `<leader>up` 切换 diff 显示，`q` 结束 tour。

Diff 只改变显示，不修改真实 buffer、undo 历史或磁盘文件，也不额外创建 tab、window 或 buffer。不要先修改代码再用 undo 撤销来模拟预览。只有任务明确授权实际修改时，才另外用 `apply_patch` 或其他写入工具执行；执行前重新读取当前内容。

### HTML 展示

需要真实 UI 布局或视觉状态对比时，写一个聚焦当前问题的 dark theme HTML 文件。匹配产品的颜色、字体、间距和组件，使用真实名称和数据，只考虑桌面端。macOS 用 `open`、Linux 用 `xdg-open` 打开；需要检查页面或继续交互时，先读取 `browser-use` skill。

### guidance

Place each visual next to the short text it supports. Keep only the calls, files, props, states, and boundaries needed to answer the user's current question or the options to resolve the current discussion point.

You may use one of these, you may use several, it is unlikely you will use all of them. Use your judgement and don't overwhelm the user.
