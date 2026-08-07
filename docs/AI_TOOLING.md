# AI 工具链配置

本仓库管理的 AI 编码工具（Claude Code / Codex / Kimi Code / opencode / pi）配置全貌：资产怎么落位、
共享守卫引擎与 `cc-usage` 用量统计怎么工作、改了东西怎么生效。

## 资产地图

```text
tree/home/.claude/                          →  ~/.claude/
├── CLAUDE.md                                  仅一行 `@~/.agents/AGENTS.md` import + Claude 专属补充
├── settings.json                              permissions / hooks 注册 / plugins
├── statusline-command.sh                      状态栏脚本（当日用量段调 cc-usage）
└── hooks/                                     granularity "children"：目录保持真实
    ├── pretool.toml                        →  ~/.claude/hooks/pretool.toml（守卫规则表）
    └── (cc-hook)                              二进制不入库，post_sync 编译后复制进来
                                               （住 hooks/ 是因为 settings.json 按此路径注册）

~/.local/bin/cc-usage                          用量统计二进制，同样不入库、post_sync 装进来
~/.local/bin/agent-hook                       Codex / Kimi Code hook adapter，同样由 post_sync 安装
~/.codex/pretool.toml                          ← Claude pretool.toml 的受管链接（共享规则）
~/.kimi-code/pretool.toml                      ← 同上，agent-hook kimi-pretool 的缺省规则路径

tree/home/.kimi-code/                       →  ~/.kimi-code/（逐文件链；sessions/oauth/credentials
├── config.toml                              主配置：permission 硬禁区 / hooks 注册（kimi-pretool）
│                                            / providers/models——/login 会原地回写 oauth 节
│                                            回流仓库（同 pi 的 settings.json）；运行期物留本机）
└── tui.toml                                 终端偏好（theme/editor/通知）

tree/home/.agents/                          →  ~/.agents/（整层镜像）
├── AGENTS.md                                  ★ 全局指令唯一真相源（跨 harness）
│                      ── distribute ──→       ~/.pi/agent/AGENTS.md
├── skills/            ── distribute ──→       ~/.claude/skills/ + ~/.codex/skills/（逐 skill 链）
├── codex/
│   └── hooks.json      ── distribute ──→       ~/.codex/hooks.json
└── claude/                                    Claude 专属格式，按工具命名空间隔离
    ├── agents/        ── distribute ──→       ~/.claude/agents/
    └── commands/      ── distribute ──→       ~/.claude/commands/

tree/home/.pi/agent/                        →  ~/.pi/agent/（granularity "children"：目录保持真实）
└── settings.json                              pi 主配置；auth.json/sessions/bin/ 等运行期物留本机

cli/crates/cc-hooks/                           共享引擎 + Claude/Codex/Kimi adapter（Rust）
cli/crates/cc-usage/                           cc-usage 用量统计源码（Rust）
scripts/common/cc-hook-test                 →  .gen/scripts/（进 PATH 的黑盒回归命令）
```

设计原则：**skills 和全局指令是公开标准、不专属某个工具**，所以源住中立目录 `.agents/`，
Claude 和 codex 一样只是 `distribute()` 的订阅者；落点保持真实目录，机器本地
（不入库）的 skill 可与受管链接共存。接入新工具 = `dots.lua` 的 `to` 列表加一行 + `dots sync`。
例外是 Kimi Code：它原生读 `~/.agents/AGENTS.md` 与 `~/.agents/skills/`，连订阅都不用加。
同一 domain 的可选细则收在一个 skill 内：顶层 `SKILL.md` 只做路由，按场景 reference
`subskills/*.md` 与 `references/`；subskill 本身是普通 Markdown，不再放嵌套 `SKILL.md`。

全局指令的两种接法按工具能力选：Claude Code 只认 `~/.claude/CLAUDE.md`，但支持
`@path` import，所以那份退化成一行 import；pi 只认自己 agent 目录下的 `AGENTS.md`，
够不着 `~/.agents/`，走 `distribute` 链过去。工具特定的协议 adapter
写在各自的文件里，别污染中立源。

## Spec-first engineering workflow

`tree/home/.agents/skills/` 中的工程 workflow 以 repo-local Markdown 为 source of truth，并允许 implicit invocation：

```text
wayfinder                         Spec 生命周期唯一入口：从模糊目标创建 / 固化当前对话 / 沿 frontier 推进
grill-with-docs                   逐问解决 decision Subspec，并同步 domain docs
implement                         一次实现并验证一个 implementation Subspec
handoff                           当前 Agent 基于 live state 生成可执行、用后即删的一次性交接文档
domain-modeling                   维护 CONTEXT.md glossary 与 docs/adr/
```

默认路径是 `specs/<yy-mm-dd>-<spec-slug>.md` 单文件 Spec；只有内容确实太大、单文件装不下时
才拆分到同名目录 `specs/<yy-mm-dd>-<spec-slug>/<n>-<subspec-slug>.md`。repo 已有约定时优先
沿用。spec 目录必须先被 ignore 排除（没有就先加入），Spec 与 Subspec 不提交入库。
Spec 是整个 effort 的 contract，Subspec 是单 session 工作单元，dependency 写在
Subspec frontmatter 的 `depends_on`。`blocked` 是 dependency 推导状态，不落盘。

这些 skills 不依赖 issue tracker，也不创建 label、assignee 或 resolution comment。用户明确
要求同步 tracker 时，tracker 只能作为 mirror，repo 内 Spec 与 Subspec 仍是 canonical artifact。

`handoff` 由当前 Agent 亲自采集 Spec、源码、git 与验证 evidence 后编写，不能把生成交接文档
再次委托出去。handoff 只是一次性 transport：接手 Agent 完整读取并把工作写入自己的 plan 后
立即删除；它不得成为 Spec、源码或长期项目文档之外的第二份 authority。

## settings.json 要点

源：`tree/home/.claude/settings.json`。

- **permissions.deny**：硬禁区，先于 hook 生效——敏感读取（`~/.ssh/**`、gh/docker/npm
  凭据、`.credentials.json`、codex `auth.json`）与毁灭性命令（`sudo` / `mkfs` / `dd`）。
- **hooks.PreToolUse**：matcher `*` 全量进 `~/.claude/hooks/cc-hook pretool`（见下文）。
- **env**：关遥测 / 错误上报 / 反馈问卷。
- **plugins**：rust-analyzer-lsp、superpowers、frontend-design。

permissions 与 cc-hook 的分工：permissions 是 Claude Code 内建的粗粒度白/黑名单；
cc-hook 负责需要**词法理解**的判定（旗标簇、链式命令、字段匹配）和**软引导**
（deny 的 reason 喂回模型让它自己改方案）。

## Kimi config.toml 要点

源：`tree/home/.kimi-code/config.toml`（字段全集见官方文档，这里只记人手维护的部分）。

- **`[[permission.rules]]` deny**：与 Claude `permissions.deny` 对齐的硬禁区，另补 Kimi 自身
  凭据（`~/.kimi-code/credentials`、`~/.kimi-code/oauth/**`）。pattern 形如 `Read(path)` /
  `Bash(cmd *)`，首条命中生效。
- **`[[hooks]]` PreToolUse**：省略 matcher = 全量工具（对齐 Claude 的 `matcher *`），进
  `~/.local/bin/agent-hook kimi-pretool`。
- **`telemetry = false`**、**`default_permission_mode = "auto"`**。
- providers/models/services 节由 `/login` 与模型目录刷新维护（oauth 节回写会经软链回流仓库，
  这是要的行为），别手改。

全局指令与 skills 不用配：Kimi 原生读 `~/.agents/AGENTS.md` 与 `~/.agents/skills/`。


## 共享守卫引擎

源码 `cli/crates/cc-hooks/`，保留两个入口：

- `cc-hook pretool`：Claude Code adapter，支持规则的 `deny` / `ask`。
- `agent-hook codex-pretool`：Codex adapter，复用同一判定结果；Codex PreToolUse
  暂不支持 `ask`，故将其降级成带原始理由的 hard deny，避免 hook failure 后意外放行。
- `agent-hook kimi-pretool`：Kimi Code adapter，语义同 Codex（ask 同样降级为 hard deny）；
  评估前把 Kimi 的 `FetchURL` 工具名改写为共享表使用的 `WebFetch`。

```text
src/
├── main.rs            cc-hook：Claude Code adapter
├── bin/agent-hook.rs  agent-hook：Codex / Kimi Code adapter
├── common/            跨 hook 事件共用
│   ├── outcome.rs       HookRun 统一返回值（业务函数不做 IO）
│   └── wire.rs          stdout / stderr / 审计日志统一落地
└── pretool/           PreToolUse 专属
    ├── argv.rs          命令词法：引号感知切段 / heredoc 剥除 / 短旗标簇
    ├── engine.rs        规则匹配：首条命中
    ├── evaluation.rs    harness 无关的规则判定与审计语义
    ├── envelope.rs      Claude Code 输出信封
    ├── codex.rs         Codex 输出信封与 ask → deny 降级
    ├── kimi.rs          Kimi Code 输出信封、ask → deny 降级、FetchURL → WebFetch 改写
    └── rules.rs         规则表 TOML schema
```

**fail-open 铁律**：任何失败（坏 stdin、规则文件缺失、TOML 解析失败、引号不闭合）
都表现为「无意见」静默放行（exit 0），绝不阻断正常命令。唯一例外是留痕：规则文件
存在但解析失败时 stderr 打一行告警（`claude --debug` 可见），防止守卫静默失效无人知。

### 决策流

```text
PreToolUse JSON (stdin)
  → tool_name == "Bash" ?
      是 → tool_input.command 过 [[bash]] 规则（argv 引擎），命中即返回
  → 所有工具过 [[tool]] 规则（字段匹配器）
  → 全不中 → 静默放行（走 harness 正常权限流程）

Claude：deny = 直接拦；ask = 弹确认框给用户
Codex： deny = 直接拦；ask = hard deny + 解释当前 PreToolUse 不支持 ask
```

Codex 全局 hook 源是 `tree/home/.agents/codex/hooks.json`，只匹配 `^Bash$`。当前
`[[tool]]` 规则仍按 Claude 的 canonical tool name 编写，且 Codex hosted tools
不经过本地 PreToolUse，因此首期只复用可验证等价的 Bash 规则。新增或修改
`~/.codex/hooks.json` 后，需要在 Codex `/hooks` 中审核并信任当前定义。

### 审计日志（可观测性）

守卫每次**命中决策**（deny/ask）或 **fail-open 留痕**（坏规则放行）时，wire 层追加一行到
审计日志，让「拦了/放了什么」可回溯——这是上线任何 block 类钩子前必备的可观测性，
否则守卫静默失效无人知。

- 路径：`CC_HOOK_AUDIT_LOG` 环境变量优先（指向 `/dev/null` 即关闭），缺省 `~/.claude/cc-hook.log`。
- Codex / Kimi Code adapter：`AGENT_HOOK_AUDIT_LOG` 优先，缺省
  `~/.local/state/agent-hook/audit.log`；降级行额外记录 `source_decision=ask`。
- 只记决策与留痕，**不记静默放行**（信噪比：放行是绝大多数，记了等于刷屏）。
- 行格式：`<epoch秒> decision=<deny|ask> tool=<Bash|工具名> rule=<规则名> cmd=<命令摘要≤200字>`。
- 落盘是 best-effort——任何 IO 失败都静默吞掉，**绝不违反 fail-open**（写不了日志也不阻断命令）。
- 测试隔离：`e2e_pretool.rs` / `e2e_production_rules.rs` / `cc-hook-test` 均把 `CC_HOOK_AUDIT_LOG`
  导向 `/dev/null`，不污染真实日志。

查最近被拦/被问的命令：`tail ~/.claude/cc-hook.log`。

### 规则表（pretool.toml）

源：`tree/home/.claude/hooks/pretool.toml`。Claude 直接读取该路径；`dots sync` 将同一源
分发到 `~/.codex/pretool.toml` 供 Codex 读取，避免复制两份规则。
同类规则自上而下首条命中。

**`[[bash]]`** —— 作用于 `tool_input.command` 的 argv 分词结果，条件 AND：

| 字段            | 语义                                                         |
| --------------- | ------------------------------------------------------------ |
| `cmd`           | argv[0] 全等（`command` 前缀自动剥除）                        |
| `subcmd`        | argv[1] 全等（如 git 子命令）                                 |
| `any`           | 词形列表，任一命中                                            |
| `all`           | AND-of-OR 词组：每组至少命中一个词形                          |
| `args_re`       | 位置参数正则（cmd/subcmd 之后），任一命中                     |
| `path_outside`  | 位置参数存在不落在任何白名单路径（绝对路径前缀）下的才命中——白名单豁免守卫（仅 `/tmp` 下放行，其余全拦） |

词形约定：`-x`（单杠单字母）查短旗标簇（`-rf` 含 `r`、`f`）；其余按字面词查 argv。

词法语义（`pretool/argv.rs`，低误伤设计）：

- 切段**引号感知**：单/双引号、反斜杠转义内的 `;` `|` `&` 换行不切——
  `git commit -m "fix; rm -rf temp"` 不误伤
- **heredoc 正文剥除**：`<<EOF … EOF` 之间不参与匹配（支持 `<<-`、引号定界符、
  一行多个）；`<<<` herestring 正确区分
- 短旗标收集遇字面 `--` 停止（POSIX 操作数约定）：`rm -- -rf` 不误伤
- 引号不闭合的段整体丢弃（fail-open）

**`[[tool]]`** —— 任意工具：`tool` 全等 `tool_name`，`where` 各字段匹配器 AND。
字段缺失或值非字符串 → 不命中（朝放行倾斜）。

匹配器词汇（同匹配器多种类 AND；每种类数组值内 OR）：

| 词汇       | 语义                                                       |
| ---------- | ---------------------------------------------------------- |
| `equals`   | 全等                                                       |
| `contains` | 含子串                                                     |
| `prefix`   | 前缀                                                       |
| `suffix`   | 后缀                                                       |
| `glob`     | git 风格路径 glob（`**/.env` 命中 `.env` 与 `a/b/.env`）   |
| `domain`   | URL 域名（含子域：`gist.github.com` 命中 `github.com`；防 `github.com.evil.com` 伪装） |
| `re`       | 正则兜底（仅在上述词汇表达不了时用）                       |
| `not`      | 反向：嵌套一个匹配器，内层命中则整体不中                   |

`not` 的典型用法——glob 命中但豁免模板文件：

```toml
[[tool]]
name     = "no-dotenv-read"
tool     = "Read"
where    = { file_path = { glob = ["**/.env", "**/.env.*"], not = { suffix = ".example" } } }
decision = "deny"
reason   = ".env 可能含密钥，不直接读。需要时让用户摘录非敏感字段，或读 .env.example。"
```

### 已知边界（有意为之）

守卫定位是**拦模型的无心之失，不防蓄意绕过**——false positive（误拦正常操作）比
false negative（漏拦）代价高，因为还有 permissions 和人工确认兜底。因此不拦：
`/bin/rm` 绝对路径、`env` / `exec` / `xargs` 前缀、`$(…)` 命令替换内的命令、
**`curl … | sh` 这类管道到 shell 的远程执行**（引擎在 `|` 处切段，下游 `sh` 与上游
`curl` 拆成两段独立匹配，单条规则无法识别「管道到 shell」整体模式——要拦只能各自拦，
误伤面大，故不拦）。这些边界由 `cc-hook-test` 的「已知绕过」分区固化，哪天行为变了测试会报。

链式命令**会**拦：`a && b && git push` 按 `&& || ; | &` 与换行切段，每段独立过规则，
`git push` 那段照样命中（测试已固化）。

rm 守卫是白名单豁免：`path_outside = ["/tmp"]` 只放行落在 `/tmp` 下的 `rm` **递归删除**
（`-r`/`-R`/`--recursive`，不要求 `-f`——非交互 shell 递归删本就不需要它），其余（`$HOME`、
`/usr`、`/` 等）全拦。`cd` 不跨段跟踪，相对路径按 hook 进程 cwd 判（不在白名单就拦）——删
`/tmp` 下相对路径请写绝对路径。路径先做词法规范化（剥 `.`、回退 `..`），`/tmp/../etc` 不会
被词法前缀误放行。

## cc-usage 用量统计

源码 `cli/crates/cc-usage/`，二进制 `cc-usage`，落 `~/.local/bin/`。状态栏那段
`📅today $x.xxxx N.NMtok +a/-r` 就是它算的：**跨 session 的当日** token / 成本 / 改动行。

```text
src/
├── main.rs            bin：clap 分发 + 一行 JSON 到 stdout
├── clock.rs           UTC 时间戳 → 本地日期（按天分桶的键）
├── metrics/
│   ├── mod.rs           Metrics：token / 改动行 / 工具次数 / 按模型拆分
│   ├── ledger.rs        ★ 去重账本：按稳定键攒条目，最后才折成 Metrics
│   ├── tokens.rs        四类 token（单价不同，必须分开存）
│   └── price.rs         model → 计价档 → USD
├── transcript/
│   ├── discover.rs      递归找 ~/.claude/projects/**/*.jsonl（含 subagents/）
│   ├── entry.rs         JSONL 单行 → 闭合事件（Message / Patch）
│   └── scan.rs          增量扫描：字节进度 → 按天账本
└── store.rs           状态落盘：扫描进度 + 按天账本目录
```

```bash
cc-usage today                     # 扫当天动过的 transcript 后打印今日汇总
cc-usage report --date 2026-07-01  # 只读已有状态打印某天（不扫描）
cc-usage backfill --days 30        # 回扫补历史，首次几十秒
```

statusline 里写死 `$HOME/.local/bin/cc-usage` 而不是靠 PATH——状态栏脚本跑在非交互
shell 里，别假设 rc 文件被 source 过。

### 为什么必须读 transcript

Claude Code 喂给 statusline 的 `context_window.current_usage` **不能当累加源**：状态栏
按 300ms 防抖刷新，窗口内的中间状态是**丢帧而非延迟**，官方没有补偿机制。实测把它
逐轮累加，token 比 transcript 真值少 5%~13%。transcript 是唯一的完整账本。

`cost.total_cost_usd` 倒是精确，但它只覆盖当前 session；要「今日跨 session」就还得
自己计价，于是干脆全部从 transcript 算，单一真相源。

### transcript 的四个坑（改这块前先读）

- **一条 API 响应按 content block 拆成多行**，各行共享 `message.id`、`usage` 重复。
  所以 usage 必须按 id 去重取末条（个别行是流式中途值，实测出现过 `output_tokens`
  先 1 后 309），而 `tool_use` 反过来要**逐行**数——每行只带一个块，按 id 去重会漏掉
  工具调用。
- **`/compact`、fork 出 background job 会把上游会话的行原样复制进新 transcript**：
  `uuid` 与 `message.id` 保持不变，只有 `sessionId` 被改写成新会话 id。实测一天里
  100 条消息同时存在于两个文件，**按文件各自累加指标再相加会多算 37%**。所以指标
  必须先进 `ledger.rs` 的去重账本（用量按 `message.id`、活动按行 `uuid`），跨文件
  合并完才折成 Metrics——指标只会加不会减，先折就没法再去重了。
  `sessionId` 不能当去重键，它在复制时会变。
- **subagent 的 transcript 在 `<session-id>/subagents/` 子目录**，token 常比主线还多，
  只扫一级目录会少算一大截 → discover 必须递归。
- **计价系数对齐 Claude Code**：cache write 记 `1.25 ×` input、cache read 记 `0.1 ×`
  input。按 1h TTL 的真实 `2 ×` 算会让「今日总额」比状态栏上同一 session 的成本还高，
  看着像坏了。`price.rs` 有一条拿真实会话数据对账的回归测试守着。

### 状态与并发

状态在 `$XDG_CACHE_HOME`/`~/.cache` 下 `cc-usage/`，两块：

```text
progress/<路径哈希>.json       每个 transcript 扫到第几字节
ledger/<YYYY-MM-DD>/<哈希>.json  它那天贡献的条目（按 message.id / uuid 索引）
```

汇总某天只读那天的目录，过期清理就是删整个日期目录，保留 30 天。账本存的是「从文件
开头累计」的**绝对值**而非增量——多个 session 的状态栏并发刷新同一个 transcript，最坏
是某次写回偏旧、下次自愈，**不会重复计数**，所以不需要加锁。

删掉状态目录只会损失历史（当天数据下次刷新即重建），`cc-usage backfill` 可补回。

## 双速部署链

| 改什么                      | 怎么生效                                                        |
| --------------------------- | --------------------------------------------------------------- |
| `pretool.toml` 规则         | Claude 保存即生效；Codex/Kimi 侧首次接入需 `dots sync` 建立共享链接 |
| 引擎代码（`cli/crates/cc-hooks/`） | `dots sync` → post_sync 编译；`cc-hook` 进 `~/.claude/hooks/`，`agent-hook` 进 `~/.local/bin/` |
| Codex `hooks.json`          | `dots sync` 后在 Codex `/hooks` 审核并信任；定义变更会要求重新审核 |
| Kimi `config.toml`/`tui.toml` | 受管软链，保存即生效；hooks 注册需 **Kimi 新会话**（官方口径：启动时读一次） |
| 用量统计（`cli/crates/cc-usage/`） | 同上，产物落 `~/.local/bin/cc-usage`                              |
| `statusline-command.sh`      | 受管软链，保存即生效                                            |
| `settings.json` 的 hooks 注册 | 需要**新会话**（Claude Code 启动时读一次）                      |

## 测试

三层，分工明确：

```bash
cargo nextest run -p cc-usage  # 单测：解析 / 增量扫描 / 计价 / 状态落盘
cargo nextest run -p cc-hooks # 共享引擎 + Claude/Codex adapter e2e
cc-hook-test                # 黑盒：~/.claude/hooks/ 部署二进制 × 生产 pretool.toml（部署最后一公里）
cc-hook-test <bin路径>      # 测任意二进制（如刚编译的 cli/target/release/cc-hook）
```

cargo test 内部又分两类规则来源，**已无手工同步负担**：

- `pretool/engine.rs` 与 `tests/e2e_pretool.rs` 的内嵌 `RULES` 是**合成的引擎语义 fixture**
  （含生产表没有的 `Probe` 等探针规则），只为覆盖匹配引擎，**无需镜像生产表**。
- `tests/e2e_production_rules.rs` 用 `include_str!` 直接内联**真实** `tree/home/.claude/hooks/pretool.toml`
  （改 toml 即触发本测试重编译），断言生产规则的决策——生产表正确性由它单源覆盖，
  不再靠手抄 fixture，「改生产表忘了同步测试」这类漂移由它机械兜住。
- `tests/e2e_codex_pretool.rs` 起真实 `agent-hook`，同时覆盖合成规则与生产规则，
  固化 deny 同形、ask → deny 降级、未命中静默三条 Codex 契约。
- `tests/e2e_kimi_pretool.rs` 同形覆盖 Kimi Code adapter，额外断言
  `FetchURL` 命中共享表的 `WebFetch` 规则（含生产表 `gh-not-webfetch-github`）。

`cc-hook-test`（源 `scripts/common/cc-hook-test`）按四区断言 deny/ask/silent 与
exit 0 契约：规则表预期 / 误伤回归 / 已知绕过 / fail-open。全绿 exit 0，有挂 exit 1。
能抓到 cargo test 抓不到的：忘了 `dots sync`（测旧二进制）、部署链路坏掉。
新增/修改生产规则时，同步在 `cc-hook-test` 的 A 区补一行黑盒断言即可（合成 fixture 不用动）。
