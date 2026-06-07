# AI 工具链配置

本仓库管理的 AI 编码工具（Claude Code / Codex / opencode）配置全貌：资产怎么落位、
`cc-hook` 守卫引擎怎么工作、改了东西怎么生效。

## 资产地图

```text
tree/home/.claude/                          →  ~/.claude/
├── CLAUDE.md                                  全局指令（所有项目生效）
├── settings.json                              permissions / hooks 注册 / plugins
├── statusline-command.sh                      状态栏脚本
└── hooks/                                     granularity "children"：目录保持真实
    ├── pretool.toml                        →  ~/.claude/hooks/pretool.toml（守卫规则表）
    └── (cc-hook)                              二进制不入库，post_sync 编译后复制进来

tree/home/.agents/                          →  ~/.agents/（整层镜像）
├── skills/            ── distribute ──→       ~/.claude/skills/ + ~/.codex/skills/（逐 skill 链）
└── claude/                                    Claude 专属格式，按工具命名空间隔离
    ├── agents/        ── distribute ──→       ~/.claude/agents/
    └── commands/      ── distribute ──→       ~/.claude/commands/

cli/crates/cc-hooks/                           cc-hook 引擎源码（Rust）
scripts/common/cc-hook-test                 →  .gen/scripts/（进 PATH 的黑盒回归命令）
```

设计原则：**skills 是公开标准、不专属某个工具**，所以源住中立目录 `.agents/`，
Claude 和 codex 一样只是 `distribute()` 的订阅者；落点保持真实目录，机器本地
（不入库）的 skill 可与受管链接共存。接入新工具 = `dots.lua` 的 `to` 列表加一行 + `dots sync`。

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

## cc-hook 守卫引擎

源码 `cli/crates/cc-hooks/`，二进制 `cc-hook`。**子命令 = hook 事件**
（`pretool` → PreToolUse；将来 posttool/stop 同理），事件内的工具差异全部下沉到规则 TOML。

```text
src/
├── main.rs            bin：clap 分发 + wire 落地（stdout/stderr/exit code）
├── common/            跨 hook 事件共用
│   └── outcome.rs       HookRun 统一返回值（业务函数不做 IO）
└── pretool/           PreToolUse 专属
    ├── argv.rs          命令词法：引号感知切段 / heredoc 剥除 / 短旗标簇
    ├── engine.rs        规则匹配：首条命中
    ├── envelope.rs      stdin JSON 解析 + hookSpecificOutput 输出信封
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
  → 全不中 → 静默放行（走 Claude Code 正常权限流程）

命中输出：{"hookSpecificOutput": {"permissionDecision": "deny"|"ask", "permissionDecisionReason": "…"}}
  deny = 直接拦，reason 喂回模型让它换方案；ask = 弹确认框给用户
```

### 规则表（pretool.toml）

源：`tree/home/.claude/hooks/pretool.toml`，**改完即生效**（每次 hook 调用现读）。
同类规则自上而下首条命中。

**`[[bash]]`** —— 作用于 `tool_input.command` 的 argv 分词结果，条件 AND：

| 字段      | 语义                                                         |
| --------- | ------------------------------------------------------------ |
| `cmd`     | argv[0] 全等（`command` 前缀自动剥除）                        |
| `subcmd`  | argv[1] 全等（如 git 子命令）                                 |
| `any`     | 词形列表，任一命中                                            |
| `all`     | AND-of-OR 词组：每组至少命中一个词形                          |
| `args_re` | 位置参数正则（cmd/subcmd 之后），任一命中                     |

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
`/bin/rm` 绝对路径、`env` / `exec` / `xargs` 前缀、`$(…)` 命令替换内的命令。
这些边界由 `cc-hook-test` 的「已知绕过」分区固化，哪天行为变了测试会报。

## 双速部署链

| 改什么                      | 怎么生效                                                        |
| --------------------------- | --------------------------------------------------------------- |
| `pretool.toml` 规则         | 保存即生效（hook 每次调用现读 TOML）                            |
| 引擎代码（`cli/crates/cc-hooks/`） | `dots sync` → post_sync 钩子 `cargo build` + 复制进 `~/.claude/hooks/`（见 `dots.lua`） |
| `settings.json` 的 hooks 注册 | 需要**新会话**（Claude Code 启动时读一次）                      |

## 测试

两层，分工明确：

```bash
cargo test -p cc-hooks      # 单测 + e2e：cargo 产物 × fixture 规则（语义正确性）
cc-hook-test                # 黑盒：~/.claude/hooks/ 部署二进制 × 生产 pretool.toml（部署最后一公里）
cc-hook-test <bin路径>      # 测任意二进制（如刚编译的 cli/target/release/cc-hook）
```

`cc-hook-test`（源 `scripts/common/cc-hook-test`）按四区断言 deny/ask/silent 与
exit 0 契约：规则表预期 / 误伤回归 / 已知绕过 / fail-open。全绿 exit 0，有挂 exit 1。
能抓到 cargo test 抓不到的：忘了 `dots sync`、生产规则表改坏。

引擎与规则表协同演进时，三处 fixture 需同步：`pretool/engine.rs` 内嵌 RULES、
`tests/e2e_pretool.rs` 的 RULES、`cc-hook-test` 的用例（各文件头注释有标注）。
