---
name: show-me
description: 用 Neovim tour、ASCII/Unicode 字符图和原 buffer 内的只读 diff 预览讲清当前问题；需要展示真实 UI 布局时制作聚焦的 HTML。
---

用**中文**, 用dark theme

Help the user understand the current topic of conversation visually. Skip the preamble and keep prose brief. Pick the smallest view that makes the key point clear.

### 展示方式

- 代码、伪代码、调用树、组件树、文件树或 ASCII/Unicode 字符图能讲清楚时，使用 Neovim tour。先读取 `nvim-use` skill，按 showcast 流程发布站点；代码窗口展示相关代码，站点 comment 用 fenced code block 放图或结构草图，配简短中文说明。
- 流程图、时序图也用字符展示，不转图片。Mermaid 可作为制图输入，先转换为纯字符，再把结果放进 tour 的 `text` 代码块；不要依赖 Neovim 渲染 `mermaid` 代码块。可用已安装的 `mmdflux --format text --color off` 生成 Unicode 图，或 `--format ascii --color off` 生成纯 ASCII 图。按窗口宽度精简或拆分图，避免折行破坏结构。
- 展示具体代码的拟议修改时，使用下文的原 buffer diff 预览。
- 只有需要真实 UI、颜色、间距或布局展示时，才使用 HTML。

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

- Show component interaction, control flow, or data flow as character diagrams in the tour:

```text
User            UI                 Daemon
 │               │                    │
 ├─选择命令──────►│                    │
 │               ├─发送展开后的提示──►│
 │               │◄────流式结果───────┤
```

- Use `diff` sketches in tour comments when comparing conceptual structures. For concrete source edits, use the inline preview below. Match the diff shape to the topic.

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

### 原 buffer 内的 diff 预览

先读取 `nvim-use` skill，把拟修改后的完整文本放入 tour 站点的 `proposed_text`。以当前 buffer（包括未保存内容）为基准；进入站点显示对应 diff，离站清除，结束 tour 一并清理。删除行高亮，新增行以虚拟行展示；告知用户 `<leader>up` 切换 diff 显示，`q` 结束 tour。

Diff 只改变显示，不修改真实 buffer、undo 历史或磁盘文件，也不额外创建 tab、window 或 buffer。不要先修改代码再用 undo 撤销来模拟预览。只有任务明确授权实际修改时，才另外用 `apply_patch` 或其他写入工具执行；执行前重新读取当前内容。

### HTML 展示

For a visual UI, layout, or visual state comparison, write one focused HTML file in dark theme — a diagram, an infographic, or a short slide deck, whichever fits the point. Match the product's colors, type, spacing, and components; use real labels and data; only support desktop. Then open it for the user:

```
Bash(open path/to/show-me-{description}.html)
```

### guidance

Place each visual next to the short text it supports. Keep only the calls, files, props, states, and boundaries needed to answer the user's current question or the options to resolve the current discussion point.

You may use one of these, you may use several, it is unlikely you will use all of them. Use your judgement and don't overwhelm the user.
