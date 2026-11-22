# React 组件与调参

包名 `@pi-design/react`。公开的运行时组件只有 `DesignProvider`、
`Editable.Text`、`Editable.Box`，另有 `useParameters()` hook。
布局、按钮、表单、图表和动画使用普通 React、CSS 或第三方库。

## DesignProvider

```tsx
import { DesignProvider, type DesignData } from "@pi-design/react";
import design from "../design.json";

<DesignProvider data={design as DesignData}>
  <App />
</DesignProvider>;
```

| Prop       | 类型         | 含义                            |
| ---------- | ------------ | ------------------------------- |
| `data`     | `DesignData` | 从 `design.json` 导入的初始数据 |
| `children` | `ReactNode`  | 整个作品应用                    |

放在作品根部。独立运行时使用传入的数据；在设计工作区里，使用当前设计状态。
用户直接修改或模型 patch 数据后，组件按最新值重新渲染。

## Editable.Text

用于允许用户直接改写的纯文本：标题、段落、按钮文案。

```tsx
<Editable.Text id="hero-title" as="h1" className="hero-title" />
```

```json
{
  "elements": {
    "hero-title": {
      "text": "Listen without distraction",
      "style": { "fontSize": 48, "color": "#f4f1e8" }
    }
  }
}
```

| Prop                         | 含义                                                     |
| ---------------------------- | -------------------------------------------------------- |
| `id`                         | 必填，对应 `design.json.elements` 中的键；同一页面内唯一 |
| `as`                         | 渲染的元素或组件，默认 `span`                            |
| `children`                   | 未设置 `elements[id].text` 时显示的内容                  |
| `className`、`style`、事件等 | 传给实际渲染的元素                                       |
| `ref`                        | 指向渲染的 DOM 元素                                      |

`elements[id].text` 优先于 `children`。
`elements[id].style` 优先于 JSX `style` 中的同名属性。
用户双击进入文字编辑，输入时自动保存；Enter 或失焦结束编辑，
Shift+Enter 换行，Escape 恢复进入编辑前的文字。

有链接、强调等嵌套结构的段落，拆成多个独立文本节点，不将整段富文本
放在一个可直接改写的 `Editable.Text` 中。

## Editable.Box

用于可选择、调整尺寸和样式的容器。保留正常 React 子组件与交互。

```tsx
<Editable.Box id="player-controls" as="section" className="controls">
  <button onClick={play}>
    <PlayIcon />
    <Editable.Text id="play-label" />
  </button>
</Editable.Box>
```

Props 与 `Editable.Text` 相同，默认渲染 `div`。
容器的 `elements[id]` 定义 `style`；容器内文字由单独的 `Editable.Text` 绑定。

```json
{
  "elements": {
    "player-controls": { "style": { "gap": 16, "padding": 24 } },
    "play-label": { "text": "Play" }
  }
}
```

样式名使用 React CSS 命名，例如 `backgroundColor`、`borderRadius`。
数值遵循 React CSS 语义；长度数值通常为 px，也可以用 `"2rem"`、
`"100%"` 等字符串。

`Editable.Box` 不是组件搭建器。页面结构和子组件组成仍在 TSX 中维护。
选择框和尺寸手柄由工作区提供，不在作品里自行实现。

## useParameters()

返回参数当前值组成的对象，不返回控件定义：

```tsx
const parameters = useParameters();
const coverSize = Number(parameters.coverSize);

<img src="/cover.jpg" style={{ width: coverSize }} />;
```

已知参数类型时可以指定返回类型：

```tsx
const { coverSize, accent, showLyrics } = useParameters<{
  coverSize: number;
  accent: string;
  showLyrics: boolean;
}>();
```

参数控件定义在 `design.json.parameters`。控件由工作区渲染，模型只定义
参数并将值连接到作品的 props、样式或行为。

| `control` | `value` 类型 | 其他字段                  |
| --------- | ------------ | ------------------------- |
| `range`   | number       | `min`、`max`、`step`      |
| `number`  | number       | 可选 `min`、`max`、`step` |
| `color`   | string       | 颜色值                    |
| `text`    | string       | —                         |
| `boolean` | boolean      | —                         |
| `select`  | string       | `options: string[]`       |

每个参数可用 `label` 提供显示名称。

```json
{
  "parameters": {
    "coverSize": {
      "label": "封面尺寸",
      "control": "range",
      "value": 240,
      "min": 120,
      "max": 400,
      "step": 4
    },
    "accent": { "label": "强调色", "control": "color", "value": "#b8d89c" },
    "showLyrics": { "label": "显示歌词", "control": "boolean", "value": true },
    "density": {
      "label": "列表密度",
      "control": "select",
      "value": "comfortable",
      "options": ["compact", "comfortable"]
    }
  }
}
```

为当前作品选择有意义的参数，例如封面尺寸、粒子密度、动画速度。
第三方组件通过正常 props 接收这些值，不需要专门的编辑器插件。

## 元素选择与评论

`Editable` 会给实际元素添加 `data-design-id`，供工作区定位。
可编辑元素能被选中、调整属性、附着评论；没有绑定的区域仍可附着坐标评论。
更换标签或重构组件时保留 ID，使用户已有的调整和评论继续对应同一对象。
