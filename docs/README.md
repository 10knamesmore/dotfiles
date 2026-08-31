# Dotfiles

跨平台个人 dotfiles 仓库，管理入口是自写的 Rust CLI **`dots`**（源码在 `cli/`）。

核心机制是**软链接镜像**：`tree/` 下的目录结构镜像 `$HOME`，仓库即配置的单一真相源。`dots sync` 把它们链接到位——编辑仓库即生效，无需复制、无需重新安装。

仓库不管理系统包、rustup、Node、pnpm 或 AI CLI。`bootstrap.sh` 要求本机已有 `cc` 与 `cargo`；受管 Pi Distribution 还要求 Node 22.19+ 与 pnpm 11.18。`dots sync` 会运行声明的 lifecycle hook、构建 crates.io binary，并把 Cargo 实际生成的 bin 收敛到 `~/.cargo/bin`。

## 快速开始

本机预先装好 C 编译器和 Rust/Cargo 后：

```bash
git clone <your-repo-url> ~/dotfiles
~/dotfiles/bootstrap.sh
```

`bootstrap.sh` 检查 `cc` 与 `cargo`，执行 `cargo build --release`，再直接运行 `dots sync`。bootstrap 不自举系统包、Rust、Node、pnpm 或 AI CLI；sync 会执行声明的 hook 并应用显式 Resource。

已有环境只想链配置：

```bash
cd ~/dotfiles
./dots.sh sync --dry-run   # 先看会做什么
./dots.sh sync             # 实际执行
```

`dots.sh` 是开发期透传脚本（`cargo run --release`，自动编译）。编译产物在 `cli/target/release/dots`——想直接敲 `dots <子命令>` 需自行把它加进 PATH 或建链（bootstrap 不代劳）。

日常跳转有两个 alias（由 `.zshrc_dotfiles` 提供）：`dot` cd 到仓库根，`skill` cd 到 skills 目录。

## 命令一览

| 命令                       | 作用                                                                  |
| -------------------------- | --------------------------------------------------------------------- |
| `dots sync [--dry-run]`    | 把完整 Desired Set 收敛到本机                                          |
| `dots status`              | 只读展示与 sync 相同的创建、更新、删除、Collision 和 Drift Plan       |
| `dots forget <resource>`   | 不改真实对象，只从 Applied Inventory 放弃 ownership                   |

## 仓库结构

```text
dotfiles/
├── dots.lua          # 例外清单（人手编辑，CLI 不写入）
├── dots.sh           # 开发期透传（cargo run --release）
├── bootstrap.sh      # 检查 cc/cargo → 编译 → dots sync
├── cli/              # Rust workspace：crates/（dots-core 纯逻辑 + dots bin）、lua-api/（类型标注）
├── pi/               # vanilla Pi 的 TypeScript extension source 与 pnpm workspace
├── tree/             # ★ 映射根：目录结构即链接声明
│   ├── home/         #   → $HOME（跨平台）
│   ├── home.linux/   #   → $HOME（仅 Linux，条目级覆盖通用层）
│   └── home.macos/   #   → $HOME（仅 macOS）
├── scripts/          # 脚本源（common/ linux/ macos/），聚合到 .gen/scripts/ 进 PATH
├── docs/             # 文档（本文件实际位置）
├── .gen/   (不入库)  # 派生区：scripts/ 聚合软链、injected/ 模板渲染产物
├── .dots/  (不入库)  # state.json Applied Inventory（删除与 Drift 判断依据）
├── README.md         # → docs/README.md 的符号链接
└── AGENTS.md         # → CLAUDE.md 的符号链接
```

## 映射规则

`dots sync` 按四条规则工作：

1. **纯 `$HOME` 镜像**：`tree/home/X` → `$HOME/X`；`tree/home.<os>/X` 仅在该平台生效，同名条目时平台层覆盖通用层。
2. **链接粒度启发式**：文件直接链；层根的一级目录（如 `.config/`）是「容器」，下钻逐子项链；二级及更深目录（如 `.config/nvim/`）整目录链。
3. **粒度覆盖**：启发式不对时在 `dots.lua` 写一行 `granularity(path, {mode=…, ignore=…})`。
4. **链接判定**：未拥有且与声明完全一致的链接自动接管；未拥有且不同的目标报告 Collision，sync 不覆盖。

## dots.lua（例外清单）

约定盖不住的才写这里。可用 API：

- `granularity(path, spec)` — 覆盖某路径的链接粒度（`mode = "dir" | "children" | "file"` + `ignore`）
- `distribute(name, spec)` — 一源多落点（如 skills 同时分发到 codex / copilot）
- `root(name, spec)` — 声明 `$HOME` 之外的映射层（罕用，如 macOS App Support）
- `scripts { ignore_tree = … }` — 脚本聚合时递归拍平的子目录（子目录默认保持树形）
- `dots.hook.before_sync(spec)` — 在真实 sync planning 前运行具名程序
- `dots.resource.symlink/copied_file/cargo_binary/managed_block/systemd_user_unit` — 显式持续 Resource
- `dots.path.exists(path)` — 声明阶段只读条件，用于 Resource 的 `enabled`

完整参考见 [LUA_API.md](LUA_API.md)。Lua API 不提供任意 shell Action、run-once 状态、`dots.file.*`、`dots.cargo.build` 或 `dots.json.*`。

`dots.lua` 维护全局 crates.io package inventory。字符串 `cargo_binary.source` 不需要 version、binary 或 target；Cargo 生成的全部 bin 分别成为 `~/.cargo/bin/<文件名>` Resource。该 inventory 不包含系统包、rustup、Claude、Node 或 pnpm。

CLI 不编辑 `dots.lua`。编辑器类型补全由 `.luarc.json` 挂载 `cli/lua-api/dots.meta.lua` 提供。

## 路径注入

配置只引用「安装后路径」或自身相对路径，不使用模板路径占位符：

- `dots sync` 写 `~/.config/dots/env.zsh`（export `DOTFILES_DIR` / `DOTS_SCRIPTS` + PATH），`.zshrc_dotfiles` 首行 source 它。
- 读不到 shell 环境的消费者（systemd unit）才用模板：`.inject` 后缀 + minijinja `{{ }}`，可引用 `{{ DOTFILES }}`（仓库根）和 `{{ SCRIPTS }}`（聚合脚本目录），产物落 `.gen/injected/` 再链过去。缺变量直接报错（strict 模式）。

## Zsh 结构

Zsh 不使用框架，配置分为两层：

- `~/.zshrc` — `dots-env` managed block 只负责 source `~/.zshrc_dotfiles`；block 外的软件内容（conda / nvm 等）始终保留。**不要把它当主配置维护。**
- `~/.zshrc_dotfiles` — 主配置（源在 `tree/home/.zshrc_dotfiles`），按序加载 `~/.config/zsh/*.zsh` 模块：
  - `10-options.zsh` — 历史 / 目录 / 补全 / 键绑定
  - `20-functions.zsh` — cd-ls、copypath、copyfile、allclear 等内联微函数
  - `90-syntax-highlighting.zsh` — fast-syntax-highlighting（仓库内固定版本）
- 平台差异在 `tree/home.linux/.zshrc_linux` / `tree/home.macos/.zshrc_macos`。

提示符是 starship + 自写 transient prompt；`z` 由 zoxide 提供。

## 修改配置的正确方式

- **改已有配置**：直接编辑 `tree/` 下对应文件。已链接的文件改完即生效，无需重跑任何命令。
- **加新配置**：把源文件放进 `tree/` 对应位置，再运行 `dots sync`。

## Collision 与 Drift

未拥有的目标位置已有内容时：

- 与 Declaration 完全一致 → 自动接管，不重写
- 与 Declaration 不同 → Collision，sync 不动；手工移走或删除陌生目标后再 sync

已拥有的 Resource 偏离 Applied Inventory 时报告 Drift，不自动修复。Declaration 删除后，未漂移 Resource 自动删除；漂移 Resource 保留实际内容与 inventory，可以恢复实际状态后再 sync，或用 `dots forget <resource>` 放弃 ownership。

`forget` 只删除 Applied Inventory 记录，不构建 Plan，也不读取或修改真实对象。Declaration 仍存在时，下次 sync 会把该对象视为未拥有：实际状态与声明一致则重新接管，否则报告 Collision。

预览受管 target、inventory 动作和 sync hook：`dots sync --dry-run`。dry-run 不执行 lifecycle hook；为计算 Cargo binary 内容，它仍可能更新 Cargo target 或 `.gen` derivation cache。

## AI 工具链（skills / agents / hooks）

`tree/home/.agents/` 是手写、跨 agent AI 资产的唯一真相源：`skills/` 分发到 `~/.claude/skills`、`~/.codex/skills` 与 `~/.kimi/skills`（逐 skill 链接），`claude/agents|commands/` 分发到 `~/.claude/` 对应目录，agent hook 定义与规则再分发到各工具配置目录。接入新工具时在 `dots.lua` 增加对应分发目标并运行 `dots sync`。

Claude Code、Codex、Kimi Code 与 Pi 共用 `cli/crates/agent-hooks/` 的 Rust PreToolUse 判定引擎。各 adapter 通过同一个 `agent-hook` binary 保留 harness 协议差异；Pi 的 `ask` 由 extension 在有 UI 时显式确认，无 UI 时拒绝。`tree/home/.agents/hooks/pretool.toml` 负责高风险操作提示和工具偏好重定向。它是引导模型的启发式守卫，不替代 sandbox、permission 或系统权限。

`pi/` 是 vanilla Pi 的本地 Distribution，不包含或 fork Pi CLI。Pi 通过 jiti 直接加载仓库内的 TypeScript extension；`dots sync` 先按 frozen lockfile 安装 Acorn，再把两个 source tree 和 workflow-authoring skill 链到 `~/.pi/agent/`。Workflow runtime 由本仓库直接维护并保留原作者 MIT license。依赖和本地 state 边界见 [pi/README.md](../pi/README.md)。

## 当前管理的主要配置

- Shell：Zsh（自管 conf.d，无框架）+ Starship
- 编辑器：Neovim（LazyVim）/ Vim
- 终端：Kitty；文件管理器：Yazi；多路复用：Zellij；监控：btop
- Linux 桌面：Hyprland（主，0.55+ Lua 入口）/ niri（备选）/ QuickShell（状态栏 + 控制中心）/ xremap / systemd user 单元
- macOS：yabai / skhd / sketchybar / fcitx5
- AI 工具：Claude / Codex / Kimi skills、agents 与共享 PreToolUse 守卫，opencode、pi

以上是主要部分，完整清单以 `tree/` 实际内容为准。

## 更多文档

- [LUA_API.md](LUA_API.md) — `dots.lua` 全部 Lua API 的参考（参数、行为细节、坑与配方）

## 注意事项

- 顶层 `README.md` 是指向 `docs/README.md` 的符号链接；`AGENTS.md` 指向 `CLAUDE.md`。
- `.gen/`、`.dots/` 是机器本地派生物，不入库。
- 新机必须先准备 C 编译器与 Cargo；使用 Pi Distribution 时还需 Node 22.19+ 与 pnpm 11.18。系统包、rustup、Node、pnpm 和 AI CLI 由使用者管理；声明式 hook 与 crates.io binary 由 sync 执行。
