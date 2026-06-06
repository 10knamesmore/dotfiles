# dots —— dotfiles 管理 CLI 设计文档

日期：2026-06-06
状态：已与用户逐节确认定稿

## 0. 背景与目标

现状 `install.py`（717 行 Python）的核心痛点：

1. **每加一个非 XDG 规范的链接目标（如 `~/.claude/hooks/xxx`）都要改安装器代码**。各类 coding agent 配置目录均不守 XDG，这类需求高频出现。
2. 模板渲染（`*_TEMPLATE` 占位符 + str.replace）人体工学差：改配置要"改模板→跑渲染"两步，模板变量表硬编码在 Python 里。
3. 无 per-host 支持：显示器坐标、背光设备名、校园网账号等单机参数硬编码。
4. oh-my-zsh + `static/omz_custom` 维护负担：更新唠叨、vendor 结构丑。

新机制目标（用户拍板）：

- 实现载体：**Rust 自写 CLI（名 `dots`）**，源码作为 workspace 放仓内，新机用极薄 bootstrap.sh 引导 cargo 编译。
- 核心机制：软链接管理，仓库即真相源。
- 保留能力：路径注入（重新设计）、`~/.zshrc` stub 保护、scripts 聚合进 PATH、bootstrap 装机。
- 本期实现 per-host 支持与 secrets 不入库安置。
- 目录结构可大改、一步到位。
- 顺带完成 OMZ 退役（shell 栈自包含化）。

## 1. 心智模型（一句话）

> **文件放进 `tree/` 的对应位置 = 声明了链接；约定盖不住的例外写进 `dots.toml`；日常操作用 CLI 动词（adopt/sync/status/undo），永远不改安装器代码。**

## 2. 仓库结构

```
dotfiles/
├── bootstrap.sh              # 极薄（~15 行）：装 rustup → cargo build → exec dots bootstrap
├── dots.toml                 # 例外清单（只记约定盖不住的东西，人可读可手编，预期 < 50 行）
├── cli/                      # Rust workspace
│   └── crates/
│       ├── dots/             #   bin：命令行、彩色输出
│       └── dots-core/        #   lib：纯逻辑（清单解析、Plan 计算），无 IO 副作用，可单测
├── tree/                     # ★ 映射根：目录结构即链接声明，纯 $HOME 镜像
│   ├── home/                 #   → $HOME（跨平台）
│   ├── home.linux/           #   → $HOME（仅 Linux，条目级覆盖通用层）
│   └── home.macos/           #   → $HOME（仅 macOS）
├── scripts/                  # 脚本源
│   ├── common/  linux/  macos/
├── hosts/                    # per-host 层
│   ├── <hostname>.toml       #   本机变量 + 本机专属链接
│   ├── files/<hostname>/     #   本机专属配置文件（如 monitors.conf）
│   └── secrets.age           #   age 加密 secrets（密文入库，明文永不入库）
├── packages/                 # bootstrap 清单（纯文本/TOML，不硬编码在代码里）
│   ├── pacman.txt  aur.txt  brew.txt  apt.txt  toolchains.toml
├── docs/
├── .gen/        (gitignore)  # 派生区：scripts/（聚合）、injected/（渲染产物）
├── .dots/       (gitignore)  # state.json 链接台账（谁建/何时/什么模式）→ undo/unlink/漂移检测
└── backup/      (gitignore)  # 时间戳备份（沿用现机制）
```

注意：`static/` 目录随 OMZ 退役一并消失（见 §7）；`generated/` 被 `.gen/` 替代。

## 3. 映射规则（替代 install.py 全部分支逻辑）

**规则 1 —— 纯 $HOME 镜像**：
- `tree/home` → `$HOME`，所有平台。
- `tree/home.<os>` → `$HOME`，仅该平台；同一目标路径**平台层条目级覆盖通用层**（同现状行为），被遮蔽项 `dots status` 可见。
- 没有任何工具专属概念。`~/.claude/...` 就住在 `tree/home/.claude/...`。
- 真正非 `$HOME` 的目标（如 macOS `~/Library/Application Support/...`）用 `dots.toml [roots]` 声明额外层，按需出现。

**规则 2 —— 链接粒度启发式**：
- 文件 → 直接链。
- **层根第一级目录是"容器"**：不链自身，下钻一层、逐子项链接。
- 第二级及更深的目录 → 整目录链。

| tree 路径 | 行为 | 说明 |
|---|---|---|
| `tree/home/.vimrc` | `~/.vimrc` → 链文件 | |
| `tree/home/.config/` | 容器，下钻 | `~/.config/nvim` 等各自成链（=现状） |
| `tree/home/.config/nvim/` | 整目录链 | 深度 2（=现状） |
| `tree/home/.claude/` | 容器，下钻 | skills/hooks/agents/settings.json 各自成链 |
| `tree/home/.claude/hooks/` | 整目录链 | `~/.claude` 本体保持真实目录，Claude Code 运行时数据（projects/、todos/）不进仓库 |

**规则 3 —— 清单覆盖**：启发式不对时在 `dots.toml` 写 `[granularity]`（`mode = "dir"|"children"|"file"` + `ignore`）。

## 4. dots.toml（例外清单）

```toml
[roots]
# 仅当目标不在 $HOME 镜像内时声明额外层：
# vscode = { path = "~/Library/Application Support/Code/User", os = "macos" }

[granularity."home/.config/opencode"]
mode   = "file"                # 逐文件链
ignore = ["node_modules", "package.json", "bun.lock", ".gitignore"]
# opencode 在配置目录生成运行时垃圾，整目录链会拖进仓库视图

[granularity."home.linux/.config/systemd/user"]
mode = "file"                  # 只链 *.service/*.timer 单文件；.wants/ 由 systemctl 管（§10）

[distribute.skills]
src  = "tree/home/.claude/skills"   # 唯一真相源（主落点走镜像 → ~/.claude/skills）
to   = ["~/.codex/skills", "~/.copilot/skills", "~/.config/opencode/skill"]
mode = "children"              # 逐 skill 链，防 codex 的 skills/.system 运行垃圾污染

[distribute.agents]
src = "tree/home/.claude/agents"
to  = []                       # 将来加目标：dots dist add agents <path>

[systemd]
user-units = ["napcat.service", "mihomo.service", "bsu-login.service", "bsu-login.timer"]
# sync 时执行 systemctl --user enable（幂等）；.wants/ 软链不再入库

[scripts]
keep-tree = ["hypr"]           # hypr/ 子目录保持树形（键位用 $scripts_dir/hypr/xxx.sh 引用）
```

## 5. CLI 命令面

```
dots sync [--dry-run]      幂等收敛：建链/渲染注入/聚合脚本/systemctl enable/护 stub
dots adopt <path>...       ★ 收编：搬进 tree 正确位置 + 原地反链 + 记台账（+必要时写清单）
dots status                三态巡检：✔ 已链 / ~ 漂移 / ✘ 缺失 + 孤儿链接；非零退出码可挂 CI
dots doctor                深检：hostname 未命中、注入变量未解析、distribute 目标父目录缺失、
                           脚本重名、被遮蔽条目、台账与磁盘漂移
dots dist add <name> <to>  给分发组加一个落点（一条命令，写清单+建链）
dots undo                  撤销上一次 mutating 操作（靠 .dots/state.json 操作日志）
dots unlink <path> [--keep-in-repo]   解除纳管，文件搬回 $HOME
dots secret set|edit <key> age 加密读写 hosts/secrets.age
dots host init             生成本机 hosts/<hostname>.toml 模板
dots bootstrap             装机：packages/* + toolchains + 末尾自动 sync
dots completions zsh       补全
```

`adopt` 的智能（全部可被 `--layer/--mode` 参数覆盖）：
- 按路径自动推断层：`~/.claude/hooks/x` → `tree/home/.claude/hooks/`；平台歧义时询问。
- grep 到文件内含仓库绝对路径 → 提醒转 `.inject`（防 systemd unit 类文件换机失效）。
- 检测到目录含 node_modules/lock 文件 → 建议 `mode="file"+ignore`。
- 输出附 `dots undo` 提示。

## 6. 路径注入：渲染降级为最后手段

**原则：配置文件只引用"安装后路径"（$HOME 侧）或自身相对路径，永不引用仓库内部路径。`$DOTFILES_DIR` 只用于真正指向仓库本身的东西（`dot` 别名、`.gen/scripts`）。**

**A. 环境变量吃掉 90%**：`dots sync` 写固定路径小文件 `~/.config/dots/env.zsh`：

```zsh
export DOTFILES_DIR="/home/wanger/dotfiles"
export DOTS_SCRIPTS="$DOTFILES_DIR/.gen/scripts"
path=($DOTS_SCRIPTS $path)
```

`.zshrc_dotfiles` 变回普通文件直接软链（不再 .template），首行 source 该 env 文件。现有 4 个模板变量全部消灭，编辑配置即生效。

**B. 读不到 shell 环境的消费者才渲染**：典型是 systemd unit 的 `ExecStart=`。约定：源文件带 `.inject` 后缀（如 `bsu-login.service.inject`），sync 时替换 `@DOTFILES@ @SCRIPTS@ @VAR:x@ @SECRET:k@`，产物落 `.gen/injected/` 再链到目标。变量优先级：host vars > 内置 > secrets。`doctor` 报未解析的 `@…@`。预计全仓只剩 1-2 个这类文件。

**C. Hyprland 兜底**：`hyprland.lua` 改 `os.getenv("DOTFILES_DIR")`，取不到时读固定兜底文件 `~/.config/dots/root`（sync 写入，一行仓库路径）——裸 TTY 启动 Hyprland 拿不到 zsh 环境也不断。

## 7. shell 栈：OMZ 退役与模块化

### 7.1 退役决策

- **oh-my-zsh 本体退役**（提示符早已是 starship，主题层从未使用）。
- **you-should-use 删除**（用户判定无用）。
- **correction（setopt correct_all）删除**。
- **zsh-syntax-highlighting：整库冻结 vendor**（要完整功能、不再跟上游更新；它本是独立插件不依赖 OMZ）。
- **z → zoxide**（进 packages/ 清单；一次性 `zoxide import --from z ~/.z` 迁移跳转历史）。
- copypath / copyfile / cd-ls / zsh-allclear / waiting-dots → 内联微函数（waiting-dots 保留，约 3 行）。
- `static/omz_custom` 删除，`static/` 目录消失；install.py 的 bootstrap_omz 步骤随之消亡。

### 7.2 模块化结构（conf.d 数字前缀）

```zsh
# .zshrc_dotfiles 里的加载逻辑
for _f in "$HOME"/.config/zsh/*.zsh(nN); do source "$_f"; done
```

```
tree/home/.config/zsh/
├── 10-options.zsh                  # OMZ 后果补齐表全在这（见 7.3）
├── 20-functions.zsh                # cd-ls/allclear/copypath/copyfile/waiting-dots
├── 90-syntax-highlighting.zsh      # 2 行自相对 shim：source "${0:A:h}/vendor/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh"
└── vendor/zsh-syntax-highlighting/ # 冻结整库（目录不匹配 *.zsh，不被循环加载）
```

`(nN)` = 数字排序 + nullglob；编号保证顺序，z-sy-h 的 90 号天然最后加载（其硬性要求）。新增 shell 功能 = 扔一个编号 .zsh 文件，不碰入口。

### 7.3 OMZ 隐性依赖补齐表（10-options.zsh 的验收清单）

| # | 丢失项 | 补救 |
|---|---|---|
| 1 | `HISTFILE`（OMZ 设的；不补则历史不落盘） | `HISTFILE=~/.zsh_history` |
| 2 | share_history / hist_ignore_dups / extended_history | ~4 行 setopt |
| 3 | ↑↓ 前缀历史搜索、Home/End/Delete 键 | ~10 行 bindkey（up-line-or-beginning-search 等） |
| 4 | compinit + 大小写不敏感匹配 + 菜单高亮 + 补全缓存 | ~8 行；`uv generate-shell-completion` 依赖 compinit 先跑，顺序排好 |
| 5 | `..` `...` `-` `1`-`9` 目录别名、auto_cd、auto_pushd | ~6 行 |
| 6 | interactive_comments（粘贴带 # 的命令） | 1 行 setopt |
| 7 | 终端标题自动更新 | kitty shell integration 已兜底，不补 |
| 8 | z 跳转数据库 | `zoxide import --from z ~/.z` 一次性迁移 |
| 9 | take 等零散函数 | 用到啥补啥 |

迁移验收按此表逐项手测（双终端验历史共享、前缀↑、Tab 大小写匹配等），不以"能跑"为准。

## 8. ~/.zshrc stub（沿用并加固）

```
# DOTS_MANAGED: 由 dots 维护。下面这行勿删；其余内容（conda/nvm 等追加）安全保留。
source "$HOME/.zshrc_dotfiles"
```

- 首行 marker 命中 → 只确保 source 行存在（缺则幂等补回），绝不动其他行。机器本地工具初始化放 `~/.zshrc` 不入库的习惯被保护。
- 普通文件无 marker → 备份后接管；软链 → 重建 stub（同现状）。

## 9. scripts 聚合

- `scripts/common/ + scripts/<os>/` → 聚合软链到 `.gen/scripts/`：顶层文件拍平；`[scripts] keep-tree` 列表里的子目录（hypr/）保持树形，不破坏键位 `$scripts_dir/hypr/xxx.sh` 引用。
- 重名冲突 → doctor 报错（修正现状的静默覆盖）。
- 新脚本丢进目录，sync 即进 PATH。

## 10. per-host 与 secrets

**hosts/<hostname>.toml**（按 `hostname()` 命中；**未命中且本机有 host 引用时 sync 硬报错**，不静默回落——堵死"渲错显示器坐标不报错"）：

```toml
[vars]                          # 供 .inject 文件引用
backlight = "amdgpu_bl1"
ddc_index = "1"

[links]                         # 本机专属链接（纯链接，不渲染）
"hosts/files/xz07/monitors.conf" = "~/.config/hypr/monitors.conf"
```

显示器等整段配置**优先走"per-host 文件 + 主配置 source 引用"纯链接方案**（hyprland.lua 加 source monitors.conf），不走变量渲染——hostname 失配最坏是文件缺失报错，绝不静默渲出错误值。

**secrets**：`hosts/secrets.age`（age 加密，公钥入库私钥不入库），`dots secret set <key>` 写入，`.inject` 文件里 `@SECRET:key@` 引用。诚实声明：age 只保护 git 同步面；渲染产物（如 bsu-login.service）在本机 `.gen/injected/` 仍是明文（systemd 必须读明文）。迁移时把现仓库明文校园网密码挪出（git 历史是否 filter-repo 清洗由用户后定）。

**systemd enable 状态**：`.wants/` 软链是 enable 状态载体（绝对路径、机器相关），**不再当文件纳管入库**。改为 `[systemd] user-units` 声明，sync 时 `systemctl --user enable`（幂等）。现暂存的 napcat `.wants` 链接迁移时删除。

## 11. bootstrap（新机一条命令）

```bash
git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh
```

bootstrap.sh（~15 行 sh）：有 cargo 跳过 rustup → `cargo build --release` → `exec dots bootstrap`。
`dots bootstrap`（Rust 内，可测）：探测 backend（pacman/brew/apt）→ 装 `packages/*.txt`（加包改文本不改代码）→ 装 toolchains（uv/starship/zoxide/nvm…，逐项探测幂等跳过）→ 自动 `dots sync` → 提示建 host 文件。omz 安装步骤删除。

## 12. 备份与错误处理

- 覆盖普通文件/目录前 → `backup/<ISO时间戳>/`（保留 .config 二级结构，同现状）；目标是软链 → 直接重建不备份（同现状）。
- **plan/execute 两段式**：dots-core 先算完整 Plan（建链/删除/备份/渲染清单），executor 再落盘。`--dry-run` = 只打印 Plan。Plan 可直接单测断言。
- 致命错误（建链失败）非零退出；可选目标缺失（AI 工具未装 → distribute 父目录不存在）warn 跳过（同现状容错哲学）。

## 13. 测试策略

- `dots-core`：纯逻辑单测（清单解析、粒度启发式、层覆盖、Plan 计算），proptest 跑路径边界。
- `dots` bin：assert_cmd + 临时 $HOME 跑 e2e（adopt→status→undo 全链路），insta 快照锁 status 输出格式。
- **迁移对拍闸**：`dots sync --dry-run` 链接集合与 `install.py --dry-run` 全等（target→source 逐对比对），通过才允许切换。

## 14. 迁移路径（分阶段，每步可回退，install.py 暂存共存）

1. `cli/` 落地，先实现 sync/status 核心引擎（TDD）。
2. `git mv` 重排目录：`general/.config/*`→`tree/home/.config/*`、`linux/.config/*`→`tree/home.linux/.config/*`、`general/skills`→`tree/home/.claude/skills`、`general/agents/claude`→`tree/home/.claude/agents`、`*/scripts`→`scripts/<scope>/`…（保历史）。
3. **对拍**：两边 dry-run 链接集合全等 → 验收闸。
4. 模板消灭：`.zshrc_dotfiles.template` → 普通文件 + `$DOTFILES_DIR`；`hyprland.lua` 改 getenv+兜底文件。
5. shell 栈改造：OMZ 退役、conf.d 模块化、§7.3 补齐表逐项落地与手测、zoxide 数据迁移。
6. systemd：`.wants` 链接删除（含暂存的 napcat 那条），改 `[systemd]` 声明；bsu-login 密码 → `dots secret`。
7. per-host：显示器/背光/ddc 抽进 `hosts/xz07.toml` + `monitors.conf`。
8. 全绿后删 `install.py`、`generated/`、`static/`，更新 CLAUDE.md/README 对应章节。

## 15. 三场景演练

**A. 给 Claude Code 全局加 hook**：`~/.claude/hooks/` 里调通 → `dots adopt ~/.claude/hooks/on-stop.sh` → 完（1 条命令，0 清单，0 代码）。

**B. 新装 Linux 机器**：`git clone … && ./bootstrap.sh` → 装包+工具链+建链自动完成 → `dots host init` 填显示器/背光 → `dots secret set bsu_pass` → `exec zsh`。

**C. opencode 接入 skills**：`dots dist add skills ~/.config/opencode/skill` → 完（1 条命令；对比现状要改 link_skills() 的 Python 元组）。

## 16. 已知弱点（诚实清单）

- 三种链接来源（镜像约定/distribute/inject）并存，CLI 实现复杂度最高，doctor 必须同时理解三者——用强测试基建对冲。
- 改 CLI 要重编译，迭代环比 Python 长。
- adopt 推断可能猜错（有 --layer/--mode 覆盖 + undo 兜底）。
- age 私钥安全到达新机无银弹（手动拷贝/密码管理器）。
- 清单极少数情况仍需手编（granularity），不是 100% 命令化。
- z-sy-h 冻结后不获上游修复；兼容问题出现时退路是 packages/ 加官方包一行。

## 17. 范围外（明确不做）

- 包管理（除 bootstrap 装机清单外的日常包同步）——历史上做过又删除的决策维持。
- git 历史中已有明文密码的清洗（filter-repo 需 force push，单独决策）。
- 多机 git 协作冲突的结构化 merge 工具。
- 本机静止数据加密（.gen/injected 明文是机制下限）。
