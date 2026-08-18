# TypeScript 验证工具参考

本文件只提供命令选择依据。项目的 `package.json` scripts、当前安装版本和 runner config 优先；不要因为这里列出某个工具就安装它。

## 先发现真实配置

```bash
rg -n '"(typecheck|check|lint|format|test|build)[^"]*"\s*:' -g 'package.json' .
fd -u -t f -g 'tsconfig*.json' -g 'eslint.config.*' -g 'biome.json*' -g 'vitest.config.*' -g 'jest.config.*'
pnpm exec tsc -p path/to/tsconfig.json --showConfig
```

确认：

- script 是单命令还是复合 pipeline。
- test runner 是否 watch、run-once、transpile-only 或同时 typecheck。
- 目标 source 和测试文件是否在同一个 `tsconfig` 中。
- workspace package 的 name、依赖和被依赖项是否要求扩大验证范围。

## pnpm workspace

限制命令到单个 package：

```bash
pnpm --filter '<package-name>' typecheck
pnpm --filter '<package-name>' test
```

只在改动会影响依赖方时使用 dependent selector；只在目标命令需要先验证依赖时使用 dependency selector。selector 语义必须在当前 pnpm 官方文档中重新确认。

官方资料：<https://pnpm.io/filtering>

## TypeScript compiler

独立 project 且项目没有等价 script 时，常见检查形式是：

```bash
pnpm exec tsc -p path/to/tsconfig.json --noEmit
```

使用 project references 的 build graph 时，遵循项目已有 `tsc -b` script，不把单 project 的 `--noEmit` 命令套到整个引用图。用 `--showConfig` 确认最终选项和文件集合。

严格选项资料：

- <https://www.typescriptlang.org/tsconfig/strict>
- <https://www.typescriptlang.org/tsconfig/noUncheckedIndexedAccess.html>
- <https://www.typescriptlang.org/tsconfig/exactOptionalPropertyTypes.html>
- <https://www.typescriptlang.org/tsconfig/noImplicitOverride.html>
- <https://www.typescriptlang.org/tsconfig/useUnknownInCatchVariables.html>

## Vitest

Vitest 普通运行会转译 TypeScript，但默认不 typecheck。项目未把 typecheck 集成进 test script 时，runtime test 与 typecheck 需要分别取证。

运行单文件和单用例的典型形式：

```bash
pnpm exec vitest run path/to/file.test.ts -t 'test name'
```

优先调用项目 script 并按它支持的方式透传 file/name filter；不要绕开项目 config。显式使用 `run`，避免交互环境进入 watch mode。

官方资料：

- <https://vitest.dev/guide/learn/writing-tests>
- <https://vitest.dev/guide/filtering>
- <https://vitest.dev/guide/testing-types.html>

## Jest

Jest 经 Babel 处理 TypeScript 时只转译，不 typecheck。先检查 transformer 是 Babel、ts-jest、SWC 还是项目自定义链路，再决定是否需要独立 typecheck。

单文件和单用例应通过项目 script 的 path/name filter 运行，不默认执行全部 projects。命令语法从当前 Jest CLI 与项目 config 确认。

官方资料：<https://jestjs.io/docs/getting-started#using-typescript>

## Lint 与 format

- ESLint/typescript-eslint：区分普通 AST lint 与需要 type information 的 typed lint。typed lint 的 parser project/service 必须真实覆盖目标文件。
- Biome、Oxlint、Prettier、Oxfmt：只使用项目已经选定的工具和配置；对修改文件运行精确路径，不用全仓库 write 模式。
- 不直接运行带 `--fix` 或 `--write` 的全仓库命令。先确认目标范围，自动修复后重新检查 diff。

typescript-eslint 官方资料：

- <https://typescript-eslint.io/getting-started/typed-linting/>
- <https://typescript-eslint.io/users/configs/>

## 结果报告

交付时分别报告：

- typecheck 检查了哪个 `tsconfig` 或 package
- lint 使用了哪个配置与文件范围
- runtime test 使用了哪个 runner、文件和 name filter
- build 是否运行，覆盖哪个产物边界
- 哪些更大范围的 workspace、browser、发布或 E2E 验证没有运行

命令退出码、通过数量和失败诊断必须来自本次实际输出，不能根据 script 名称推断。
