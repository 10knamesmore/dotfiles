# Dotfiles

跨平台个人 dotfiles 仓库，管理入口是自写的 Rust CLI **`dots`**（源码在 `cli/`）。

核心机制是**软链接镜像**：`tree/` 下的目录结构镜像 `$HOME`，仓库即配置的单一真相源。`dots sync` 把它们链接到位——编辑仓库即生效，无需复制、无需重新安装。

仓库**不做日常包管理**：`packages/` 清单只供 `dots bootstrap` 装机时一次性使用。

## 快速开始

新机器一条龙：

```bash
git clone <your-repo-url> ~/dotfiles
~/dotfiles/bootstrap.sh
```

`bootstrap.sh` 很薄：缺 cargo 则装 rustup（minimal profile）→ `cargo build --release` 编出 `dots` → 转交 `dots bootstrap`。后者探测包后端（pacman / apt / brew）、按 `packages/<backend>.txt` 装包、按 `packages/toolchains.toml` 幂等装工具链（当前为 uv / starship / zoxide），最后自动跑 `dots sync`。

已有环境只想链配置：

```bash
cd ~/dotfiles
./dots.sh sync --dry-run   # 先看会做什么
./dots.sh sync             # 实际执行
```

`dots.sh` 是开发期透传脚本（`cargo run --release`，自动编译）。编译产物在 `cli/target/release/dots`——想直接敲 `dots <子命令>` 需自行把它加进 PATH 或建链（bootstrap 不代劳）。

日常跳转有两个 alias（由 `.zshrc_dotfiles` 提供）：`dot` cd 到仓库根，`skill` cd 到 skills 目录。

## 命令一览

| 命令 | 作用 |
| --- | --- |
| `dots sync [--dry-run]` | 把仓库配置链接到 `$HOME`（幂等，可反复跑） |
| `dots status` | 三态巡检：✔ 已链 / ~ 漂移 / ✘ 缺失；非全绿时退出码非零 |
| `dots adopt <path> [--layer …] [--mode …]` | 把 `$HOME` 里现成的文件收进仓库管理（搬进 `tree/` + 原地建反向链接） |
| `dots unlink <path> [--keep-in-repo]` | 停止管理某文件（默认把文件搬回 `$HOME`） |
| `dots undo` | 撤销上一次 adopt 或 unlink |
| `dots doctor` | 只读体检：未覆盖主机、脚本重名、外部链接、钩子键漂移等；有问题时退出码非零 |
| `dots secret set <key>` / `dots secret list` | 管理 age 加密的敏感值（密文入库，明文只在渲染时出现） |
| `dots bootstrap` | 新机装包 + 工具链 + sync（见上） |
| `dots completions <shell>` | 生成 shell 补全脚本 |

## 仓库结构

```text
dotfiles/
├── dots.lua          # 例外清单（人手编辑，LuaLS 类型补全；CLI 永不改它）
├── dots.sh           # 开发期透传（cargo run --release）
├── bootstrap.sh      # 新机入口（自举 cargo → 编译 → dots bootstrap）
├── cli/              # Rust workspace：crates/（dots-core 纯逻辑 + dots bin）、lua-api/（类型标注）
├── tree/             # ★ 映射根：目录结构即链接声明
│   ├── home/         #   → $HOME（跨平台）
│   ├── home.linux/   #   → $HOME（仅 Linux，条目级覆盖通用层）
│   └── home.macos/   #   → $HOME（仅 macOS）
├── scripts/          # 脚本源（common/ linux/ macos/），聚合到 .gen/scripts/ 进 PATH
├── hosts/  (按需)    # per-host 资产：files/<host>/、secrets.age（密文入库；目录按需创建，当前还没有）
├── system/           # root 级文件源（udev/systemd），dots 不链接，手动安装
├── packages/         # bootstrap 装机清单（pacman.txt 等纯文本 + toolchains.toml）
├── common/           # 手动同步参考资料（VS Code 配置；dots 不处理）
├── docs/             # 文档（本文件实际位置）
├── .gen/   (不入库)  # 派生区：scripts/ 聚合软链、injected/ 模板渲染产物
├── .dots/  (不入库)  # state.json 链接台账（undo/unlink/漂移检测用）
├── backup/ (不入库)  # 覆盖普通文件前的时间戳备份（sync 时按需创建）
├── README.md         # → docs/README.md 的符号链接
└── AGENTS.md         # → CLAUDE.md 的符号链接
```

## 映射规则

`dots sync` 按四条规则工作：

1. **纯 `$HOME` 镜像**：`tree/home/X` → `$HOME/X`；`tree/home.<os>/X` 仅在该平台生效，同名条目时平台层覆盖通用层。
2. **链接粒度启发式**：文件直接链；层根的一级目录（如 `.config/`）是「容器」，下钻逐子项链；二级及更深目录（如 `.config/nvim/`）整目录链。
3. **粒度覆盖**：启发式不对时在 `dots.lua` 写一行 `granularity(path, {mode=…, ignore=…})`。
4. **链接判定**：目标已是指向仓库内的链接（含旧路径、断链）→ 无条件重建；真实文件 → 备份到 `backup/<时间戳>/` 后链；链接指向仓库外 → 报漂移、不动它。

## dots.lua（例外清单）

约定盖不住的才写这里，预期长期 < 60 行。可用 API：

- `granularity(path, spec)` — 覆盖某路径的链接粒度（`mode = "dir" | "children" | "file"` + `ignore`）
- `distribute(name, spec)` — 一源多落点（如 skills 同时分发到 codex / copilot）
- `root(name, spec)` — 声明 `$HOME` 之外的映射层（罕用，如 macOS App Support）
- `systemd_user { … }` — sync 时 `systemctl --user enable`（幂等）
- `scripts { ignore_tree = … }` — 脚本聚合时递归拍平的子目录（子目录默认保持树形）
- `hosts { ["<hostname>"] = function() vars{…}; link(…) end }` — per-host 变量与专属链接；当前机器未命中且表非空时 sync 硬报错
- `on { phase = fn }` — 全局生命周期钩子（pre_sync / on_host_activate / post_link / post_sync；value 可为函数数组），钩子内可用 `dots.json.merge` / `dots.file.ensure_block` / `dots.run_once` 等幂等写原语
- `granularity`/`distribute` 的 spec 还支持条目级 `pre`/`post` 钩子——`pre` 返回 false 可跳过该条目

完整参考（每个 API 的参数、行为细节、坑与配方）见 [LUA_API.md](LUA_API.md)。

CLI 永不编辑 `dots.lua`；需要清单变更时它打印建议行让你粘贴。编辑器里有完整类型补全（`.luarc.json` 挂载 `cli/lua-api/dots.meta.lua`）。

## 路径注入

没有模板渲染占位符——配置只引用「安装后路径」或自身相对路径：

- `dots sync` 写 `~/.config/dots/env.zsh`（export `DOTFILES_DIR` / `DOTS_SCRIPTS` + PATH），`.zshrc_dotfiles` 首行 source 它。
- 读不到 shell 环境的消费者（systemd unit）才用模板：`.inject` 后缀 + minijinja `{{ }}`，可引用 `{{ DOTFILES }}`（仓库根）、`{{ SCRIPTS }}`（聚合脚本目录）、`{{ host.* }}`（per-host 变量）、`{{ secret.* }}`（age 解密值），产物落 `.gen/injected/` 再链过去。缺变量直接报错（strict 模式）。

## Zsh 结构

无框架（Oh My Zsh 已退役），两层结构：

- `~/.zshrc` — 受管 stub（首行 `# DOTS_MANAGED:` 标记），只负责 source `~/.zshrc_dotfiles`；其他软件追加的内容（conda / nvm 等）安全保留。**不要把它当主配置维护。**
- `~/.zshrc_dotfiles` — 主配置（源在 `tree/home/.zshrc_dotfiles`），按序加载 `~/.config/zsh/*.zsh` 模块：
  - `10-options.zsh` — 历史 / 目录 / 补全 / 键绑定
  - `20-functions.zsh` — cd-ls、copypath、copyfile、allclear 等内联微函数
  - `90-syntax-highlighting.zsh` — zsh-syntax-highlighting（冻结 vendor）
- 平台差异在 `tree/home.linux/.zshrc_linux` / `tree/home.macos/.zshrc_macos`。

提示符是 starship + 自写 transient prompt；`z` 由 zoxide 提供。

## 修改配置的正确方式

- **改已有配置**：直接编辑 `tree/` 下对应文件。已链接的文件改完即生效，无需重跑任何命令。
- **加新配置**：放进 `tree/` 对应位置后 `dots sync`；或者用 `dots adopt <path>` 把 `$HOME` 里现成的文件收编（它会搬文件、建链、记台账供 undo）。
- **root 级配置**（键盘 inhibit 等）：源在 `system/`，需手动 `sudo cp` 安装，见 [MANUAL_SETUP.md](MANUAL_SETUP.md)。

## 备份与漂移处理

安装时如果目标位置已有内容：

- 是符号链接且指向仓库内 → 直接重建，不备份
- 是普通文件或目录 → 移到 `backup/<时间戳>/` 后再链
- 是指向仓库外的链接 → 报漂移，不动它（`dots status` / `dots doctor` 可见）

预览一切写盘动作：`dots sync --dry-run`。

## AI Skills

`tree/home/.claude/skills/` 是本地 AI skills 的唯一真相源：

- 主落点 `~/.claude/skills` 走常规镜像
- 经 `dots.lua` 的 `distribute()` 额外分发到 `~/.codex/skills`、`~/.copilot/skills`（逐 skill 链接）

接入新工具 = `to` 列表加一行 + `dots sync`。

## 当前管理的主要配置

- Shell：Zsh（自管 conf.d，无框架）+ Starship
- 编辑器：Neovim（LazyVim）/ Vim
- 终端：Kitty；文件管理器：Yazi；多路复用：Zellij；监控：btop
- Linux 桌面：Hyprland（主，0.55+ Lua 入口）/ niri（备选）/ QuickShell（状态栏 + 控制中心）/ xremap / systemd user 单元
- macOS：yabai / skhd / sketchybar / fcitx5
- AI 工具：Claude / Codex / Copilot skills 与 agents、opencode

以上是主要部分，完整清单以 `tree/` 实际内容为准。

## 更多文档

- [LUA_API.md](LUA_API.md) — `dots.lua` 全部 Lua API 的参考（参数、行为细节、坑与配方）
- [MANUAL_SETUP.md](MANUAL_SETUP.md) — 需要 root 手动安装的部分（udev / systemd 键盘 inhibit）
- `docs/superpowers/specs/` — dots CLI 的设计文档（镜像规则、dots.lua、钩子、路径注入的完整论证）

## 注意事项

- 顶层 `README.md` 是指向 `docs/README.md` 的符号链接；`AGENTS.md` 指向 `CLAUDE.md`。
- `.gen/`、`.dots/`、`backup/` 是机器本地派生物，不入库。
- 仓库不做日常包管理；`packages/` 清单仅供装机，当前只有 `pacman.txt` 有实际内容（主力机是 Arch）。
