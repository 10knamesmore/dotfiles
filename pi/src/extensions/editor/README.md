# Editor

管理 Pi 输入框、模型正文代码块和 `/copy` 选择器。

## 正文代码块

[正文渲染] 通过 `registerMarkdownTransformer` 处理 assistant 正文。每个代码块由独立的 Pi `Markdown` 组件完成语法高亮与折行，再把围栏换成圆角边框。普通正文、用户消息和 thinking 保持原有渲染。

带颜色的边框行编码为 Markdown code span，保留代码中的空格、反引号与其他 Markdown 字符。列表和引用内的代码块使用所在容器的可用宽度。

代码框右上角的 `-[copy]-` 复制该块原文。悬停时加粗并加下划线；剪贴板写入成功后显示 success 色的 `-[✓ ok]-`，1.5 秒后恢复，按钮宽度不变。[按钮组件] 在正文 Markdown 外处理点击，按渲染后的位置命中按钮；不替换 assistant 根组件，不改写 SDK 方法。

## 复制

[复制选择器] 从 session 原文提取整条回复和代码块。复制结果不包含显示边框，也不受视觉折行影响。

[复制高亮] 在选择器打开期间，临时用自有 Component 包装原回复的正文 Markdown。选中整条回复时，高亮所有正文行；选中代码块时，按正文分段、代码内容和出现次数定位。切换选项会清除相应 Markdown 缓存；关闭选择器时恢复原组件。

组件组合与渲染使用公开 API。查找原回复仍只读 `AssistantMessageComponent.lastMessage.timestamp`，因为 Pi 没有公开的消息到组件查询接口；不覆盖 SDK 的渲染方法。

[正文渲染]: reply-markdown.ts
[按钮组件]: reply-code-buttons.ts
[复制选择器]: copy.ts
[复制高亮]: copy-highlight.ts
