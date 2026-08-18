# TypeScript 测试与验证

## 核心边界

未经用户明确要求，不新增或修改 unit、integration、E2E、type-level 测试或测试 fixture。验证 feature、bugfix 或重构时，可以运行项目已有测试，但不得以本 subskill 为由扩大成编写测试。

TypeScript 的 runtime test 与 typecheck 通常是两条证据链：Vitest/Vite 和 Jest/Babel 等常见路径可以只转译 TypeScript 而不做类型检查。先读取项目 script 和 runner config，确认实际命令是否调用 `tsc`、`vue-tsc`、`vitest --typecheck`、ts-jest 或其他 checker；不要从 `.ts` 测试能运行推断其已经通过 typecheck。

需要具体命令和 runner 差异时读取 `references/testing-tools.md`。

## compiler、lint 与测试的分工

- assignability、泛型约束、nullability、受检查的索引访问、override、返回路径和封闭联合穷尽性由当前严格配置下的 compiler 或 typed lint 提供证据，不写重复的 runtime 测试。
- 外部数据验证、业务规则、边界值、状态转换、序列化、错误路径、副作用、异步时序、取消、并发和真实模块加载属于 runtime behavior，由已有测试或任务明确要求的测试验证。
- formatter 只证明格式；lint 只证明其启用规则；typecheck 只证明被纳入 project 的静态契约；build 还可能证明 bundler、codegen、asset 和 module resolution。不要把其中一个的成功替代其他风险所需的证据。
- type-level test 只用于公共类型 API 的正负约束确实是产品契约的场景。不得为编译器本来就会在生产调用点报告的普通错误再造一层类型测试。

## 最窄验证流程

1. 确认受影响 package、最终 `tsconfig`、相关 scripts、runner 和目标文件是否包含在检查范围内。
2. 修改包含相关已有测试时，直接运行最窄 test file 和 test name。若该 runner 只转译，再运行覆盖 source 与测试文件的最窄 typecheck。
3. 改动完全由类型系统约束且没有 runtime behavior 风险时，只运行目标 package/project 的 typecheck。
4. 改动涉及 type-aware lint 才运行目标 package 或文件范围的 lint；它与 typecheck规则不同，不能机械地互相替代。
5. 改动涉及 bundling、package exports、code generation、CSS/asset import 或 runtime module resolution 时，追加目标 package 的真实 build。
6. 只对修改过的文件运行项目 formatter。不要顺手格式化整个仓库。
7. 命令失败后根据实际诊断修正，再运行提供同一证据的最窄命令。失败是诊断结果，不是需要刻意制造的流程阶段。

不要机械串行运行全 workspace 的 typecheck、lint、build 和 test。每条命令都必须对应本次改动中的具体风险。

## 用户明确要求编写测试时

- 测试 observable behavior 和公共契约，不读取私有字段、框架内部 state、CSS class、DOM 位置或实现调用次数来固定实现。
- 测试代码遵守与生产代码相同的类型标准：不使用 `any`、非空断言、双重 assertion、无类型 mock 或 blanket suppression。
- fixture 使用具名 builder 或按领域组织的对象，避免在每个测试复制巨大字面量。跨 package 共享时放进职责明确的 test-support package，不让生产 package 反向依赖测试代码。
- mock 只替换真实边界，如时钟、随机源、网络、文件系统或外部服务；优先使用项目已有 fake、in-memory adapter 或成熟 mock 工具，不深度 mock 被测模块内部实现。
- fake timer、环境变量、全局对象、module mock、临时 server 和事件监听器必须在每个用例后恢复。并发用例不得共享可变端口、目录或单例状态。
- Promise rejection、取消和超时使用 runner 的异步 assertion，不能靠未等待 Promise、固定 sleep 或吞错回调碰运气。
- 单值或少量字段直接断言；只有完整结构或渲染形状本身是契约时才使用 snapshot。

## Snapshot 纪律

- snapshot 必须小到能人工判断业务含义；动态时间、随机 ID 和机器路径先通过项目已有 serializer 或 matcher 精确归一化。
- 不运行自动接受全部 snapshot 的命令，不设置永久 update 模式。每个变化都必须人工检查差异。
- 不为隐藏不稳定输出而删除关键字段或把整段值替换成无意义占位符；先修正不确定性来源。

## 红旗

- runtime test 成功后直接宣称 TypeScript 类型检查通过
- 为消除诊断加入 `any`、`@ts-ignore`、双重 assertion 或非空断言
- 局部改动无依据地运行全 monorepo
- `pnpm test` 实际进入 watch mode，却被当作已完成的可退出验证
- 只运行 formatter 或 lint 就宣称 feature 已验证
- 自动接受 snapshot
- 未经用户要求创建测试、fixture 或 test-only dependency
- 用固定 sleep、真实公网、共享环境或未恢复全局状态制造 flaky test
