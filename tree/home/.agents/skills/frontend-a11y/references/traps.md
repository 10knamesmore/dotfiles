# A11y locator traps

本文件只处理 a11y locator 空命中、重复命中或 detached node，不处理业务正确性。

## Canvas、WebGL 与纯图形 SVG

绘制内容通常不产生可定位的 accessibility 子节点。为任务需要操作或读取的目标提供真实 DOM 语义入口；不要退化成坐标点击。

## Iframe 与 shadow DOM

- iframe 内目标从对应 frame 开始，再使用 `getByRole` 或 `getByLabel`。
- Playwright 可以穿透 open shadow root，但不能穿透 closed shadow root。
- 不要把必须由 agent 定位的目标只放在 closed shadow root 中。

## Portal 与 teleport

Dialog、popover、select menu 经常挂到 `document.body`，不再是 trigger 容器的 DOM 后代。从 document 根定位命名后的 dialog、menu 或 listbox，再向下查找目标。

## 虚拟列表

视口外项目没有 DOM 节点，因此也没有 accessibility node。先通过已有的搜索、筛选、分页或滚动让目标进入 DOM，再使用 a11y locator。不要用行索引代替目标语义。

## 重复 role 和 name

同一 scope 内多个元素拥有相同 role 和 name 时，strict locator 会失败。优先给 name 添加真实上下文；如果可见名称本来就相同，使用有名字的 region、group、table、row 或 dialog 收窄。

## Props 落到 wrapper

组件封装可能把 `aria-*`、`id`、`htmlFor` 或 visible label 留在 wrapper，真正的 button/input 没有 name。检查 accessibility snapshot，确保语义落在实际目标元素上。

## 不稳定 key 与节点重建

`key={index}`、随机 key 或 render 内定义组件可能导致目标节点被重建，已有 locator 指向 detached node。使用稳定业务 key，并保持组件类型稳定。

## `aria-hidden` 与隐藏节点

目标或其祖先带 `aria-hidden="true"` 时不会出现在 accessibility tree。需要定位的目标不能放在被隐藏的 subtree 中。

## Accessible name 漂移

按钮在 loading 等状态下替换主文案，会使 locator name 漂移。保持动作名称稳定；如果需要暴露 state，使用对应 ARIA state，而不是改成另一套 name。

## Selector 掩盖语义缺失

`data-testid`、CSS hash、组件库 class、`nth-child` 和 XPath 能命中 DOM，不代表 a11y locator 可用。出现这些 fallback 时回到 role、name 和 semantic scope 修复。
