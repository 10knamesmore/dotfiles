# TypeScript 项目配置

## 适用范围

创建 TypeScript 项目，或任务明确涉及 `tsconfig`、package scripts、module/build、lint、formatter、workspace boundary 时使用本规则。普通业务代码修改不得顺手重写项目级工具链。

## 配置前调查

1. 确认当前 TypeScript、Node 或浏览器目标、package manager、bundler、test runner 和发布格式的真实版本。
2. 用 `tsc --showConfig` 或配置文件继承关系确认最终生效选项，不只读取某个中间 `tsconfig`。
3. 检查 `include`、`exclude`、`files`、project references 和各 package 的 build config，确认 source、tests、scripts 与配置文件分别由哪个 project 检查。
4. 检查项目已经使用 ESLint/typescript-eslint、Biome、Oxlint、Prettier、Oxfmt 或框架工具中的哪一套。一个职责只保留一个 owner，不叠加功能重复的工具。
5. 查看成熟的同类 package 和仓库内相邻 package 如何设置；没有证据时不发明独有配置。

## 新项目或明确严格化任务

新建项目或用户明确要求严格化时，至少启用：

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitOverride": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "forceConsistentCasingInFileNames": true
  }
}
```

然后按当前 compiler 与实际 runtime 决定 module、resolution、lib、types、emit 和 side-effect import 相关选项。不要复制与目标 runtime 不同的整份 `tsconfig`。

严格配置的要求：

- 不用放宽 compiler option、扩大 `exclude`、添加 blanket ignore 或降低 lint severity 来让现有诊断消失。
- `skipLibCheck` 不能掩盖本项目自己生成或维护的声明错误。第三方声明问题必须先定位到具体 package、版本和声明路径，再决定任务是否包含依赖处置。
- library 的 runtime export、类型声明、source map、package exports 和实际构建产物必须由同一发布契约约束；不能只生成 `.d.ts` 就宣称 package 可用。
- app、library、test、script 需要不同环境类型时使用独立 project/config，并通过 references 或现有构建器表达关系，不把所有全局类型塞进一个 `types` 列表。

## Lint 与 formatter

- 已使用 typescript-eslint 且任务明确要求严格 typed lint 时，优先基于官方 `strictTypeChecked` 配置，再叠加少量有明确业务价值的项目规则；不要手工重造一套近似 preset。
- typed lint 会建立 TypeScript program。确认 parser project/service 覆盖目标文件，并把它的成本计入验证选择；不要误称普通 ESLint AST lint 已完成 typecheck。
- `strictTypeChecked` 的规则集合可能随 typescript-eslint 非 major 版本变化。依赖升级时查看当前版本的配置源码和 release notes，不凭旧规则列表修改 suppressions。
- formatter 只负责格式，lint 只负责确有价值的 correctness 和一致性检查。项目已有 formatter 时，不再让 ESLint 与它争夺同一批排版规则。
- 对生成代码、vendor 和 build output 使用精确目录边界；不要用过宽 glob 排除真实 source。

## Scripts 与 workspace

- 让 `typecheck`、`lint`、`format`、`test`、`build` 各自表达清楚的证据，不把名称相同的 script 包装成不可判断的复合命令。
- workspace 根命令通过 package graph 或项目已有 task runner 调度；局部验证使用精确 package filter，不默认执行整个 workspace。
- 使用 `pnpm` 和现有 lockfile。新增依赖前先证明项目已有依赖不能满足需求，并获得用户对依赖与架构变更的授权。
- package 的 source import 与 published import 分开验证。不要依赖只在 monorepo path alias 下成立、发布后不存在的路径。

## 完成条件

- 重新读取最终生效配置，确认目标文件确实被 typecheck、lint、format 和 build 中相应的 owner 覆盖。
- 对修改的配置运行最窄的真实命令并查看退出状态；只做静态文本审查不能宣称配置可用。
- 报告执行过的 package、project 和命令范围。未运行的 workspace、发布或 runtime 验证必须明确标为未验证。
