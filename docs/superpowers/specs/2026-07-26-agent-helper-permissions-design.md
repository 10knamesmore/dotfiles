# agent helper（cc-hook 重定位）+ pi 权限审批系统 · 设计稿 / Hand-off

日期：2026-07-26
范围：`cli/crates/cc-hooks/`（重命名 + 重构）、`tree/home/.pi/agent/extensions/`（新增 pi extension）

> **本文档为 hand-off 用途**：实现在另一台机器进行，该机器可访问公网、可自行 `git clone https://github.com/badlogic/pi-mono`。
> 第 3 节仍把 pi 侧技术事实连同 `file:line` 依据固化在此——它记录的是**已核实的结论**，省去重新查证；
> 但 pi 迭代很快（近 8 周周均约 91 个 commit），若行号对不上，以实际源码为准，本文档的结论需重新验证。
> 依据基于 pi-mono `@earendil-works/pi-coding-agent` **0.82.1**（commit `5bc1c2c`，2026-07-25）。

## 1. 背景与目标

pi 是 badlogic/pi-mono 的 coding agent，核心刻意做小。它**明确不内置权限系统**：

> It intentionally does not include built-in MCP, sub-agents, **permission popups**, plan mode,
> to-dos, or background bash. You can build or install those workflows as extensions or packages.
> —— `packages/coding-agent/docs/usage.md:296`

同时 `README.md:39` 说明 pi 默认以启动它的用户权限运行，官方给的隔离方案是容器化（Gondolin / Docker / OpenShell），没有细粒度审批。

本仓库已有一套成熟的守卫资产：`cc-hook`（`cli/crates/cc-hooks/`，1244 行 Rust）+ `pretool.toml` 规则表（140 行）。但它现在绑死 Claude Code 的 PreToolUse 协议。

**目标**：
1. 把 cc-hook 重定位为**工具无关的 agent helper** —— 承担所有 coding agent 都需要、且适合用 Rust 做的活（heavy CPU、强类型判定、词法分析）。
2. 在其上实现 **pi 的完整权限审批系统**（allow / deny / ask / rewrite 四态 + 三档记忆 + UI 交互 + ask-model 兜底）。
3. helper 顺带成为 **native tool 提供方**：用 Rust 类型系统定义 tool，经 IPC 把 JSON Schema 交给 TS 侧薄壳注册。

**非目标（YAGNI）**：
- 不做沙箱/容器隔离。那是 pi 官方推荐的正交手段，与审批层不冲突。

## 2. 关键约束

| 约束 | 影响 |
| --- | --- |
| `rs_agent` 是公司内部非开源项目 | **代码与 crate 依赖均不可引入本仓库**。只复用设计经验（JSONL 协议形状、`#[tool]` 宏的人机工程），那些是 LSP/MCP 早已通行的公开模式，不构成公司资产迁移 |
| dotfiles 仓库需保持自包含 | helper 作为本仓库的 crate，不依赖外部私有源 |
| 实现机器可访问公网 | 可自行 clone pi-mono 核对；§3 是已核实结论的快照，不是唯一来源 |

## 3. pi 侧技术事实（已验证，含依据）

> 全部经源码/文档核实，非记忆。路径相对 `pi-mono/packages/`。

### 3.1 Extension 加载与生命周期

| 事实 | 依据 |
| --- | --- |
| 全局 extension 位置 `~/.pi/agent/extensions/*.ts` 或 `*/index.ts` | `coding-agent/docs/extensions.md:117-120` |
| 项目级 `.pi/extensions/`，需项目被 trust 后才加载 | 同上 + `docs/settings.md:14` |
| 放上述位置才能 `/reload` 热重载；`pi -e ./x.ts` 仅适合临时测试 | `docs/extensions.md:7` |
| Extension 以**完整用户权限**运行，无沙箱 | `docs/extensions.md:111` |
| 可用 import：`@earendil-works/pi-coding-agent`、`typebox`、`@earendil-works/pi-ai`、`@earendil-works/pi-tui`、Node 内置模块；npm 依赖放同级/父级 `package.json` 即可 | `docs/extensions.md:141-152` |
| 长驻资源有 shutdown 钩子（用于管子进程生命周期） | `docs/extensions.md:220` |

### 3.2 registerTool 接受裸 JSON Schema（native tool 方案的地基）

TS 签名是 `parameters: TParams`，`TParams extends TSchema`（typebox），见 `coding-agent/src/core/extensions/types.ts:443-455`。**但那只是编译期约束**。运行时校验显式处理了「不是 typebox 而是裸 JSON Schema」的情况：

```ts
// ai/src/utils/validation.ts:278-295
export function validateToolArguments(tool: Tool, toolCall: ToolCall): any {
    const args = structuredClone(toolCall.arguments);
    Value.Convert(tool.parameters, args);
    const validator = getValidator(tool.parameters);
    if (!Object.getOwnPropertySymbols(tool.parameters).includes(TYPEBOX_KIND)) {
        const coerced = coerceWithJsonSchema(args, tool.parameters as JsonSchemaObject);
        // ...
    }
    if (validator.Check(args)) return args;
```

即：检测 `TYPEBOX_KIND` symbol 缺失 → 走通用 JSON Schema 强制转换路径。

**结论**：Rust 侧用 `schemars` 导出的 JSON Schema 经 IPC 传过来，可直接作为 `parameters` 传给 `pi.registerTool()`，无需转 typebox。TS 侧若被类型检查拦住，用 `as unknown as TSchema` 断言即可——运行时是安全的。

> 实现时建议在 extension 里加一条断言测试锁住这个行为，pi 升级若改了 `validation.ts` 能立刻发现。

### 3.3 拦截能力：tool_call

```ts
// docs/extensions.md:767-789
pi.on("tool_call", async (event, ctx) => {
  // event.toolName, event.toolCallId, event.input（可变）
  if (isToolCallEventType("bash", event)) {
    event.input.command = `source ~/.profile\n${event.input.command}`;   // 原地改写
    if (event.input.command.includes("rm -rf")) {
      return { block: true, reason: "Dangerous command" };
    }
  }
});
```

行为保证（`docs/extensions.md:761-765`）逐条抄录：
- 对 `event.input` 的修改**会影响实际执行**
- 后注册的 handler 能看到先前 handler 的修改
- **修改后不会重新做 schema 校验**
- 返回值**只能**控制阻断：`{ block: true, reason?: string }`

补充语义：
- `tool_call` 前 pi 会等 Agent 事件排空，故 `ctx.sessionManager` 是最新的（`docs/extensions.md:755`）
- 并行工具模式下，同一条 assistant 消息的兄弟工具调用**先顺序 preflight、再并发执行**，所以 `tool_call` 里看不到兄弟工具的结果（`docs/extensions.md:757`）

### 3.4 拦截能力：user_bash（`!` / `!!` 命令）

```ts
// docs/extensions.md:855-878
pi.on("user_bash", (event, ctx) => {
  // event.command / event.excludeFromContext（!! 为 true）/ event.cwd
  return { operations: remoteBashOps };          // 换后端
  // 或包装内置后端：createLocalBashOperations()
  // 或直接返回结果：{ result: { output, exitCode, cancelled, truncated } }
});
```

### 3.5 其余相关 API

| API | 用途 | 依据 |
| --- | --- | --- |
| `ctx.ui.select(msg, ["A","B","C"])` | 四档审批选单 | `docs/extensions.md:2459` |
| `ctx.ui.confirm(title, msg)` | 二元确认 | `docs/extensions.md:2462` |
| `ctx.ui.custom()` | 完整 TUI 组件（带键盘输入） | `docs/extensions.md:13` |
| `pi.exec(cmd, args, {signal, timeout})` → `{stdout,stderr,code,killed}` | 调外部进程 | `docs/extensions.md:1613-1620` |
| `pi.registerTool(def)` | 注册 native tool | `docs/extensions.md:1335` |
| `pi.registerCommand(name, opts)` | 注册 `/命令` | `docs/extensions.md:1491` |
| `pi.setActiveTools(names)` | 运行时增删活跃 tool（须为增量） | `docs/extensions.md:1622`、`2304-2318` |
| `ctx.signal` | 传给嵌套异步任务，使 Esc 可取消 | `docs/extensions.md:989` |
| `project_trust` 事件 | 自定义项目信任策略；仅全局/CLI extension 参与 | `docs/extensions.md:352-362` |

### 3.6 事件生命周期（节选自 `docs/extensions.md:277-348`）

```
user sends prompt
  ├─► input                      可拦截/变换/接管
  ├─► before_agent_start         可注入消息、改 system prompt
  │   ┌── turn（LLM 每轮调工具时重复）──┐
  │   ├─► context                       可改 messages
  │   ├─► before_provider_request       可检查/替换 payload
  │   ├─► tool_execution_start
  │   ├─► tool_call              ★ 可 block / 改写 input
  │   ├─► tool_result            ★ 可改结果（middleware 链）
  │   └─► tool_execution_end
  └─► agent_end
```

### 3.7 配置与目录

| 路径 | 性质 | 跨设备 |
| --- | --- | --- |
| `~/.pi/agent/settings.json` | 主配置 | ✅ 已纳管（软链回仓库） |
| `~/.pi/agent/extensions/` | extension | ✅ 应纳管 |
| `~/.pi/agent/auth.json` | 凭据 | ❌ |
| `~/.pi/agent/trust.json` | 项目信任决策（存本机绝对路径） | ❌ |
| `~/.pi/agent/sessions/` | 会话 JSONL | ❌ |
| `~/.pi/agent/bin/` | pi 自动下载的 fd/rg 二进制 | ❌ 平台相关 |

`tree/home/.pi/agent` 已配 `granularity(..., { mode = "children" })`（见 `dots.lua`），白名单语义：仓库里放什么才链什么，运行期文件天然不会被卷入。

> pi 用 `writeFileSync` **原地写** `settings.json`（`coding-agent/src/core/settings-manager.ts:247`，非 temp+rename），
> 故软链不会被替换，`/settings` 的改动会回流仓库。代价是 pi 自写的 `lastChangelogVersion` 等字段会进 diff。

## 4. 已决定的关键选型

| 维度 | 决定 | 理由 |
| --- | --- | --- |
| 系统范围 | **完整权限层**：allow/deny/ask/rewrite 四态 + 三档记忆 + 持久化 | pi 官方无任何权限系统，纯 prompt 约束已证明会被违反 |
| 判定引擎 | **复用现有 Rust 引擎**，不用 TS 重写 | `argv.rs` 307 行处理引号感知切段、短旗标簇（`-rf` 含 `r`/`f`）、heredoc、`--` 后操作数——最难写对的部分。重写会漂移，且新实现的 bug 会**静默放行**危险命令 |
| `rewrite` 决策 | **引入，但限等价替换** | `grep`→`rg`、`python3`→`uv run python` 这类机械翻译直接改写放行，零往返；语义有差异的（`rg -r`、`rm -rf`）仍 deny/ask。不限范围会让模型的世界模型与现实脱节，调试时极难发现 |
| 审批记忆 | **三档**（一次 / 本会话 / 永久）+ **机器本地**持久化 `~/.pi/agent/approvals.json` | 具体决策常带本机绝对路径，跨设备无意义；真正想跨设备的偏好应写进 `pretool.toml` 规则表（那才是跨设备真相源）。运行期回写人手维护的规则表会与 dotfiles「仓库即真相源」反向冲突 |
| ask-model | **由 TS 侧调模型**，Rust 只返回 `ask_model` 决策把判断委托回去 | helper 不该管 provider 认证和网络栈；TS 侧可直接复用 pi 已认证的 provider |
| tool 提供方式 | **native tool**（`pi.registerTool`），不走 MCP | pi 不内置 MCP client（`docs/usage.md:296`），走 MCP 要自己实现 client 和握手协商；native 路径已验证可行（§3.2） |
| tool 定义语言 | **TS 与 Rust 双路径并存**，见 §5.4 | 两边各有适用场景，不该二选一；对 pi 而言都是 native tool，注册路径统一在 extension 汇总 |
| 与 rs_agent 关系 | **完全独立实现**，只复用设计经验 | 公司非开源代码不可进个人仓库 |
| 新名字 | **待定**，见 §8 | |

## 5. 架构

### 5.1 分层

职责分界：**Rust 管「该不该拦」（无状态纯判定 + heavy CPU），TypeScript 管「拦了之后跟用户怎么谈」（UI、会话记忆、持久化、模型调用）。**

**推荐拆成 core lib + 两个 bin**，沿用 workspace 已有的 `dots-core`(lib) + `dots`(bin) 先例：

```
cli/crates/
├── <新名>-core/                 lib：harness 中立，无 IO、无 async
│   └── src/
│       ├── argv.rs                  词法：引号切段、短旗标簇、heredoc、`--` 边界
│       ├── rules.rs                 规则表 schema + 匹配器（equals/contains/prefix/
│       │                            suffix/glob/domain/re + not 反向）
│       ├── engine.rs                匹配 → 决策
│       └── tools/                   native tool 实现 + schemars 导出 schema
├── <新名>-hook/                 bin：一次性 exec，给 Claude Code
│   └── src/main.rs                  子命令 pretool（现有 envelope.rs 平移）
└── <新名>d/                     bin：JSONL 长驻 server，给 pi 及后续 harness
    └── src/main.rs                  stdio 双向 JSONL；guard / describe / invoke
```

**为什么拆 bin 而不是一个 bin 两个子命令**：CC 的 hook 是**每次工具调用都 fork 一个进程**，启动延迟直接叠加到每次交互上。现有 `cc-hooks` 是完全同步的、依赖极轻（`clap`/`serde`/`toml`/`shlex`/`regex-lite`/`globset`，无 tokio），启动几乎无感。而长驻 server 一旦引入 async runtime，单 bin 会把这些依赖也链进 hook 路径。拆开后 hook bin 保持精简，两边各自演进。

代价是多一个 crate。若最终 server 也用同步 IO（JSONL over stdio 本就不需要 async），则单 bin 双子命令同样成立——**这一条留到 P2 实测启动延迟后再定**，不必提前承诺。

分层依据：现有 `envelope.rs:23-41` 输出的是 CC 协议专属形状（`hookSpecificOutput` / `permissionDecision`），而 `engine`/`argv`/`rules` 完全不知道调用方是谁。`main.rs:3` 的注释也已写明「子命令 = 生命周期事件；将来 posttool/stop 同理」，扩展点是现成的。

顺带解掉一个命名债：`argv.rs`/`rules.rs` 现住在 `pretool/` 下，但它们与 PreToolUse 这个 CC 概念无关。

### 5.2 规则表共用

`pretool.toml` 保持**单一份、跨设备、两个 harness 共用**（现落点 `~/.claude/hooks/pretool.toml`，重命名后建议移到中立位置，见 §8）。

`Decision` 枚举（现 `rules.rs:15`，仅 `Deny`/`Ask`）需扩到四态。**跨 harness 降级规则**：Claude Code 不支持原地改写参数，故 `rewrite` 在 CC envelope 里降级为 `deny` + 把改写建议写进 `reason`（模型自己改，退回今天的行为）。这条降级必须有测试覆盖。

### 5.3 pi extension 结构

```
tree/home/.pi/agent/extensions/permissions/
├── index.ts              extension 入口：注册事件与命令
├── client.ts             JSONL IPC 客户端（spawn / 请求配对 / 崩溃重启）
├── approvals.ts          三档记忆：会话内 Set + ~/.pi/agent/approvals.json
├── ui.ts                 审批弹框（ctx.ui.select 四档选单）
└── ask-model.ts          ask_model 决策的模型调用
```

### 5.4 tool 的双注册路径

TS 和 Rust **都能定义 tool**，对 pi 而言两者都是 native tool，区别只在 schema 从哪来：

```
TS 侧定义                              Rust 侧定义
  typebox Type.Object({...})             #[derive(JsonSchema)] struct
  execute() 直接写 TS                     schemars 导出 JSON Schema
        │                                        │
        │                                  IPC describe → JSON Schema
        │                                        │
        └────────► pi.registerTool() ◄──────────┘
                   （§3.2：运行时接受裸 JSON Schema）
```

选哪条的判据：

| 场景 | 选 | 理由 |
| --- | --- | --- |
| 需要自定义 TUI 渲染（`renderCall`/`renderResult`） | TS | 渲染回调是 TS 闭包，跨 IPC 传不过去 |
| 需要访问 `ctx`（session、UI 交互、改 active tools） | TS | `ctx` 是 pi 进程内对象 |
| 轻量胶水、调外部命令 | TS | 不值得跨进程 |
| CPU 密集（解析、索引、大文件处理） | Rust | 正是 helper 的定位 |
| 需要强类型/穷尽匹配保证正确性 | Rust | 类型系统比手写 typebox schema 更难写错 |
| 已有 Rust 实现要复用 | Rust | |

extension 里做一个统一注册层：启动时先 `describe` 拿 Rust 侧 tool 列表注册一遍，再注册 TS 侧自己的 tool。Rust tool 的 `execute` 是薄壳——转成 IPC `invoke` 请求。

> 注意 `renderCall`/`renderResult` 只能在 TS 侧写。若某个 Rust tool 需要自定义渲染，在 extension 里为它单独补渲染回调即可（schema 和执行仍在 Rust）。

## 6. IPC 协议

### 6.1 传输

**长驻子进程 + stdio JSON Lines**（推荐），而非现在 cc-hook 的一次性 exec：

```
extension 加载 → spawn `<helper> serve --stdio` → 双向 JSONL
                                                → extension shutdown 时关闭（docs/extensions.md:220）
```

理由：一次性 exec 每次调用都要 fork + 读 TOML + 解析规则；长驻进程规则表只解析一次，可持有索引与缓存，这正是「heavy CPU 交给 Rust」诉求的前提。

**Claude Code 侧保持一次性 exec 不变**（CC 的 hook 协议就是一次性调用，不支持长驻）。故两种模式长期并存，由独立 bin 承载（§5.1）。

必须处理：子进程崩溃 → 重启并重放；helper 不可用 → **fail-open 还是 fail-close？**（见 §8 开放问题）

### 6.2 消息形状

请求/响应用 `id` 配对。

```jsonc
// TS → Rust
{"id":1, "op":"describe"}
{"id":2, "op":"guard", "harness":"pi", "tool":"bash",
 "input":{"command":"grep foo src/"}, "cwd":"/home/wanger/x"}
{"id":3, "op":"invoke", "tool":"<name>", "input":{}}

// Rust → TS
{"id":1, "tools":[{"name":"...", "description":"...", "parameters":{/* JSON Schema */}}]}

{"id":2, "decision":"allow"}
{"id":2, "decision":"deny",    "reason":"工具偏好：用 rg 替代 grep"}
{"id":2, "decision":"rewrite", "input":{"command":"rg foo src/"}, "reason":"grep→rg 等价替换"}
{"id":2, "decision":"ask",     "rule":"git-push", "signature":"git push",
                               "reason":"git 推送需要用户确认"}
{"id":2, "decision":"ask_model", "reason":"规则未覆盖，建议模型评估风险"}

{"id":3, "content":[{"type":"text","text":"..."}], "details":{}}
```

`guard` 响应中的 `signature` 是审批记忆的 key，见 §7.2。

## 7. 权限系统设计

### 7.1 判定流程

```
tool_call 事件
   │
   ├─► extension 查会话内记忆 / approvals.json ──命中 allow──► 放行（不走 IPC）
   │
   ├─► IPC guard 请求 ──► Rust 引擎按 pretool.toml 首条命中
   │
   ├─ allow    ──► 放行
   ├─ deny     ──► return { block: true, reason }
   ├─ rewrite  ──► 原地改 event.input，放行（不返回 block）
   ├─ ask      ──► ctx.ui.select 四档选单 ──┬─ 允许一次    ──► 放行
   │                                        ├─ 本会话都允许 ──► 记内存 + 放行
   │                                        ├─ 永久允许    ──► 写 approvals.json + 放行
   │                                        └─ 拒绝        ──► block
   └─ ask_model ──► TS 调模型评估风险 ──┬─ 低危 ──► 放行
                                        └─ 高危 ──► 降级为 ask，走上面的选单
```

### 7.2 审批记忆的 key 设计

**不能用整条命令做 key** —— `git push origin main` 和 `git push --force` 会被当成不同条目，而后者危险得多却可能因为前者被批准过而混淆。

建议 key = `规则名 + 规范化签名`，签名由 Rust 侧生成（它已经做完词法分析，知道命中了什么）：

```
git-push        + "git push"                → 永久允许后，git push origin main 放行
git-push-force  + "git push --force"        → 仍然单独问
rm-recursive-force + "rm -r -f"             → 仍然单独问
```

签名**只含 cmd + subcmd + 命中的旗标**，不含具体路径和参数值。这样「永久允许 git push」不会顺带允许 `git push --force`（它命中的是不同规则/不同旗标集）。

> 这是本设计中**最需要在实现时验证**的一条：签名粒度太粗会放行危险变体，太细则永久允许形同虚设。建议先按上述实现，用真实 session 跑一周再调。

### 7.3 always-ask 清单

除规则表命中外，某些工具应无条件走审批（对应「always ask」诉求）。建议做成 `pretool.toml` 的一个新段而非硬编码：

```toml
[always_ask]
tools = ["write"]                    # 例：任何写文件都问
paths = ["~/.ssh/**", "**/.env"]     # 例：碰这些路径必问
```

### 7.4 fail-open / fail-close

现有 cc-hook 是 **fail-open**（`main.rs:104` 注释：配置缺失/stdin 坏掉 → 静默放行；规则解析失败 → 留痕后放行）。这对「工具偏好重定向」是合理的，但对「权限审批」语义相反 —— 见 §8。

## 8. 开放问题（实现前需决定）

1. **新名字**。`cc-hook` 已名不副实。命名需满足：不绑定任何 harness、体现「给 agent 干重活的原生副手」、短好打。
   候选：`deputy`（副手，语义准、不撞常见命令）、`anvil`（重活承载体）、`agentd`（Unix 惯例，但 `d` 后缀暗示纯 daemon，而 CC 侧仍是一次性 exec，语义不符）。
   倾向 **`deputy`**，但这是个人品味，待定。
   连带决定：规则表落点从 `~/.claude/hooks/pretool.toml` 移到中立位置（如 `~/.config/<新名>/rules.toml`），并在 dotfiles 里改 `distribute`。

2. **fail-open 还是 fail-close**。helper 进程崩溃/不可用时：放行（可用性优先，同现状）还是拦截（安全优先）？
   建议**分级**：工具偏好类规则 fail-open，`always_ask` 与高危规则 fail-close。但这需要规则表能标注等级。

3. **ask-model 用哪个模型**。复用当前会话模型（贵、但上下文足）还是固定一个便宜模型（快、省，但缺上下文）？建议后者 + 只传命令和规则上下文。

4. **签名粒度**（§7.2）需真实使用验证。

5. **`user_bash` 要不要也走审批**。用户手打的 `!` 命令是用户自己的意图，拦截可能招人烦；但 `!rm -rf /` 同样危险。倾向只对高危规则生效，工具偏好类跳过。

## 9. 实施顺序

按「用真实消费者把协议逼出来」的原则，不要先设计完整协议再找用途。

| 阶段 | 内容 | 验收 |
| --- | --- | --- |
| **P0** | 重命名 + 拆 `<新名>-core`(lib) / `<新名>-hook`(bin)，CC 侧行为**完全不变** | 现有测试全绿；`cc-hook-test` 黑盒回归通过 |
| **P1** | `Decision` 扩到四态；CC envelope 对 `rewrite` 降级为 `deny` | 新增降级测试 |
| **P2** | `<新名>d` bin：stdio JSONL server + `guard` op | 喂 JSONL 断言决策的集成测试；实测启动延迟以定 §5.1 的单/双 bin |
| **P3** | pi extension：IPC 客户端 + `tool_call` 拦截 + deny/rewrite | 手动验证 `grep foo` 被改写成 `rg foo` |
| **P4** | 审批 UI + 三档记忆 + `approvals.json` | 手动验证四档选单与持久化 |
| **P5** | `ask_model` + `always_ask` 段 | |
| **P6** | `describe`/`invoke` op + 首个 Rust tool；同时在 extension 里放一个 TS tool 验证双路径 | 验证 §3.2 的裸 JSON Schema 路径 |

P0 必须独立成一个 commit 且行为零变化 —— 重构与功能混在一起会让回归难以定位。

P6 之后若 Rust 侧已有 2-3 个 tool，再回头做 derive 宏抽象（本期 YAGNI）。

## 10. 参考

- pi 文档（本机 clone：`~/Documents/repos/pi-mono`，commit `5bc1c2c`）
  - `packages/coding-agent/docs/extensions.md`（2961 行，extension API 全集）
  - `packages/coding-agent/docs/settings.md`、`skills.md`、`usage.md`
- 现有实现：`cli/crates/cc-hooks/`、`tree/home/.claude/hooks/pretool.toml`
- 现有文档：`docs/AI_TOOLING.md`（资产地图，重命名后需同步更新）
