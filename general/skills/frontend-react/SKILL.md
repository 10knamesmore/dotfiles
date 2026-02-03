---
name: frontend-react
description: 使用 React + Vite + TypeScript + Ant Design + Sass + pnpm 编写或修改前端代码。适用于在该技术栈下创建新的 React 组件、页面、Hooks 或工具函数，或对现有前端代码进行重构以符合指定的风格规范（TS 风格注释、箭头函数组件、SCSS 样式导入方式）。
---

# Frontend React

## 概述

用于在 **React + Vite + TypeScript + Ant Design + Sass** 技术栈下进行一致、规范的前端开发，严格约束组件写法、类型定义和注释风格。

## 规则

* 使用 `pnpm` 作为依赖管理和脚本执行工具。
* 使用 React + Vite + TypeScript + Ant Design + Sass（SCSS）。
* 样式通过 `import { style } from "xx.scss"` 引入，并使用 `style.foo` 的类名形式。
* 禁止使用 `React.FC` / `React.FunctionComponent`。
* 使用 `type Props = { ... }` 明确定义组件属性类型，并在组件参数中解构 props。
* 组件统一使用 `const` + 箭头函数定义。
* 为**每一个组件、Hook 使用点、函数定义**添加 TypeScript 风格注释。

## 组件模式

```tsx
type Props = {
  title: string;
};

/** 渲染页面标题。 */
export const Title = ({ title }: Props) => {
  return <h1>{title}</h1>;
};
```

## Hook 使用模式

```tsx
/** 跟踪面板的展开状态。 */
const [expanded, setExpanded] = useState(false);
```

## 函数模式

```ts
/**
 * @param name - user name
 * @param id - user id
 * @returns 格式化后的用户标签
 */
const formatUserLabel = (name: string, id: string) => {
  return `${name} (${id})`;
};
```

## 样式引入模式

```tsx
import { style } from "./example.scss";

<div style={style.thisDiv}>内容</div>
```
