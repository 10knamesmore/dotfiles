# 项目文件

作品是普通 React + TypeScript + Vite 项目。自己的 `package.json`、
lockfile、源码和第三方依赖都留在项目内。

```text
project/
├─ package.json
├─ vite.config.ts
├─ index.html
├─ src/
│  ├─ main.tsx
│  ├─ App.tsx
│  └─ styles.css
├─ public/
├─ design.json
└─ .design/workspace.json
```

## 依赖

运行时包位于本参考文件的 `../../../react`。根据实际 skill 路径解析
成绝对目录，用 pnpm 的 `file:` 依赖安装：

```sh
pnpm add react react-dom "@pi-design/react@file:<运行时绝对目录>"
pnpm add -D vite @vitejs/plugin-react typescript @types/react @types/react-dom
```

图标、动画、图表等第三方库按正常方式 `pnpm add` 并 import。
无需向 SDK 注册依赖。

## Vite 配置

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import piDesign from "@pi-design/react/vite";

export default defineConfig({
  plugins: [react(), piDesign()],
});
```

`piDesign()` 将设计 JSON 的修改作为数据更新传给页面，避免用户打字或
拖动时被 Vite 整页重载打断。源码仍按正常 Vite 方式热更新。
该插件只在开发服务器中运行，不进入生产构建。

pnpm 11 安装 Vite 时，若要求声明 esbuild 的构建脚本策略，在项目的
`pnpm-workspace.yaml` 中加入 `allowBuilds: { esbuild: false }`。
这里使用 esbuild 随平台依赖提供的预编译 binary。

## 入口

`index.html` 挂载 `src/main.tsx`。根组件放在 `DesignProvider` 内：

```tsx
import { createRoot } from "react-dom/client";
import { DesignProvider, type DesignData } from "@pi-design/react";
import design from "../design.json";
import { App } from "./App";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <DesignProvider data={design as DesignData}>
    <App />
  </DesignProvider>,
);
```

编辑器运行时通过 URL 查询参数连接工作区。作品路由切换时保留 URL 中
`__pi_design` 和 `__pi_artboard` 参数，使页面刷新后仍能连接。

## design.json

```json
{
  "name": "Player",
  "elements": {
    "title": {
      "text": "Now playing",
      "style": { "fontSize": 32 }
    }
  },
  "parameters": {}
}
```

元素绑定和参数定义详见 [React 组件与调参](react.md)。
这是作品当前数据，不是待应用的修改清单。

## .design/workspace.json

```json
{
  "artboards": [
    {
      "id": "desktop",
      "name": "Desktop",
      "route": "/",
      "x": 80,
      "y": 80,
      "width": 1200,
      "height": 800
    },
    {
      "id": "mobile",
      "name": "Mobile",
      "route": "/",
      "x": 1360,
      "y": 80,
      "width": 390,
      "height": 844
    }
  ],
  "comments": []
}
```

`x/y` 是画板在工作区中的位置，`width/height` 是预览视口的 CSS 像素尺寸。
`route` 是 Vite 应用内的路径，也可包含查询参数。多个方案可以使用不同
route；同一页面的响应式效果可以使用相同 route、不同尺寸。

保留已有评论。评论的完整对象和定位方式见 [Python SDK](python.md)。

## 运行与检查

依赖安装完成后 `pi_design.open(path)` 会运行 Vite，不要重复启动同一预览。
构建仍使用项目自己的 `pnpm build`。直接修改以上文件会被监听并反映到
页面；无需专用写入 SDK 或 reload。
