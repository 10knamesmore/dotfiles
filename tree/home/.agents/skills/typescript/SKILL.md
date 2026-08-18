---
name: typescript
description: TypeScript 代码编写、修改、审查、调试、项目配置与验证规范。处理 .ts、.tsx、.mts、.cts、.d.ts、tsconfig、TypeScript 类型错误、异步与模块边界、TypeScript 测试或 pnpm workspace 中的 TypeScript package 时使用。
---

# TypeScript

## 先确认项目事实

处理任何 TypeScript 代码前：

1. 读取目标文件所在目录向上的 `AGENTS.md` 或等价项目指令。
2. 读取最近的 `package.json`、`packageManager`、lockfile、scripts、生效的 `tsconfig` 继承链，以及现有 lint、format、test 和 build 配置。
3. 查看相关调用方、数据来源、运行时和当前依赖。拿不准第三方 API 时，读取项目当前安装版本的类型声明或源码，不根据记忆猜签名。
4. 使用项目已有依赖和 scripts。不要为了完成局部任务另装一套 schema、lint、format、test 或 build 工具。
5. 在 pnpm workspace 中先定位受影响 package 及其依赖关系，不默认把局部改动扩大成全 workspace 操作。

## 类型安全契约

- 不新增 `any`、`Record<string, any>`、`Function`、无类型容器或用宽泛 index signature 表示已知业务结构。
- 不用 `@ts-ignore`、`@ts-nocheck`、`as unknown as T`、非空断言 `!` 或无证明的 `as T` 消除诊断。语言或第三方声明确实无法表达已验证的不变量时，只保留最窄 assertion，并在同一边界先完成运行时检查或写清可由当前源码证明的理由。
- 把 HTTP、JSON、IPC、storage、环境变量、用户输入和第三方回调等外部数据视为 `unknown`。在进入领域逻辑前，使用项目已有的成熟 schema 或 validator 验证一次；能从 schema 推导类型时，不再手写第二份会漂移的结构。
- 禁止 `JSON.parse(text) as T`。type assertion 不会产生运行时校验。
- 用 discriminated union 表达互斥状态，用 exhaustive `switch` 或等价 `never` 检查封闭联合。不要用一组互相约束的 boolean 和 optional 字段拼出非法状态。
- 精确区分字段缺失、字段存在但值为 `undefined`、显式 `null` 三种事实；类型必须与序列化、patch、默认值和存在性判断的真实语义一致。
- 公共 API、跨模块边界、回调和配置入口显式表达参数与返回契约；局部变量优先让编译器推导，避免重复类型和类型漂移。
- 使用具名的嵌套业务类型组织相关字段。不要把多个子概念压成一个大平面对象，也不要用 `Record<string, unknown>` 逃避稳定结构建模。

## 模块与依赖

- 按业务能力组织目录；使用 `x/a.ts`、`x/b.ts`，不要平铺 `x_a.ts`、`x_b.ts`。一个模块只承担一个可描述的职责，避免无边界的 `utils.ts`、`types.ts` 和 barrel 聚合。
- 保持依赖方向单向。新增 import 前检查实际依赖图和 package boundary，不用 type-only import、动态 import 或 re-export 掩盖循环依赖。
- 按项目真实 runtime、bundler 和 package 输出选择 module 语义。`module`、`moduleResolution`、文件扩展名、package `type` 与发布产物必须一致，不能以编辑器能跳转或 `tsc` 能解析替代运行时验证。
- 类型依赖使用项目既有的 `import type` 约定；需要运行时副作用或值导入时显式保留值语义。
- 参数超过一个独立业务概念、存在互斥组合或包含多个 boolean 开关时，改成具名 options 或 discriminated union；不要用位置参数和 flag soup 隐藏调用契约。

## 错误与异步

- 把 `catch` 值按 `unknown` 收窄；不假定所有抛出值都是 `Error`，也不抛字符串或无稳定契约的普通对象。
- 每个 Promise 必须由当前调用方 `await`、返回给上层，或在明确的后台任务边界处理成功、失败、取消与生命周期。不要用裸 `void promise` 只为压掉 floating-promise 诊断。
- 并发任务必须明确顺序、资源上限、取消和部分失败语义。不要把串行循环机械改成 `Promise.all`，也不要把并发操作放进未等待的 `forEach` 回调。
- 只在业务确实允许时捕获错误；不要吞掉异常、返回伪成功值或无要求地增加 retry、fallback 和兼容分支。

## 注释与前端边界

- 编写或修改代码时同时使用 `doc-comment`，只为业务语义、数据来源、生命周期、不变量和异常行为补充有信息量的注释，不复述类型。
- 真正进入 production import、route、entry 或 bundle 的 UI component 同时使用 `frontend-a11y`，把语义和实际支持的状态暴露为可断言的 accessibility contract。
- 本 skill 不替代框架专用规则。React、Vue、Node、Electron、Vite、Next.js 等行为必须从目标项目的当前版本、配置和源码确认。

## 按任务加载细则

1. 创建或修改 `tsconfig`、package scripts、module/build 配置、lint 或 formatter 时，完整读取 `subskills/project-config.md`。
2. 编写或修改测试时，仅在用户明确要求测试后完整读取 `subskills/testing.md`。
3. 验证 TypeScript feature、bugfix 或重构时，完整读取 `subskills/testing.md`，但不得因此自行新增测试。
4. subskill 指向的 `references/` 只在需要对应工具命令时读取。
