# Editor

管理 Pi 输入框、模型正文代码块和 `/copy` 选择器。

## 正文代码块

[正文渲染] 通过 `registerMarkdownTransformer` 处理 assistant 正文。普通代码块由独立的 Pi `Markdown` 组件完成语法高亮与折行，再把围栏换成圆角边框。顶层 `openui` 围栏闭合后显示 widget；顶层 `mermaid` 围栏逐行生成字符图。两者使用同一个[代码块外框]。

带颜色的边框行编码为 Markdown code span，保留代码中的空格、反引号与其他 Markdown 字符。列表和引用内的代码块使用所在容器的可用宽度。

普通代码框右上角的 `-[copy]-` 复制该块原文。OpenUI 和 Mermaid 标题栏的 `copy` 复制当前视图：渲染结果的纯文本或原始源码。`source` / `rendered` 切换视图；`hide` 折叠，`show` 回到折叠前的视图。悬停时加粗并加下划线；剪贴板写入成功后显示 success 色的 `-[✓ ok]-`，1.5 秒后恢复。

自定义渲染器为每个围栏创建一个 `CodeBlockFrame`，通过 `source()` 提供原文、`renderBody(width)` 提供正文。`width` 已扣除外框留白。正文可提供 `copyText` 以保留原始排版；有交互的正文还可提供点击区域。流式更新时复用同一外框并调用 `invalidate()`，源码视图与折叠状态就能跨帧保留。

## 复制

[复制选择器] 提供整条回复的 Markdown 原文、普通代码块原文以及交互块的渲染文本。

[复制高亮] 在选择器打开期间，选中整条回复会高亮所有正文行；选中单个块时，按正文分段、代码内容和出现次数定位。

[回复正文组件]按渲染后的行列处理鼠标点击。它从 Pi 的 `AssistantMessageComponent` 读取 `lastMessage` 和 `isStreaming`，取得原始正文、流式状态和 `/copy` 定位所用的时间戳。

[正文渲染]: reply-markdown.ts
[代码块外框]: ../../../ui/code-block.ts
[回复正文组件]: reply-body.ts
[复制选择器]: copy.ts
[复制高亮]: copy-highlight.ts
