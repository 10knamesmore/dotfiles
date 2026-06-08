## 项目概述

这是一个跨平台 dotfiles 仓库。管理入口是自写的 Rust CLI **`dots`**（源码在 `cli/`）。

核心机制是**软链接管理**：仓库即配置的单一真相源——`tree/` 下的目录结构镜像 `$HOME`，`dots sync` 把它们链接到位，编辑仓库即生效。仓库**不做包管理**（除 `dots bootstrap` 的装机清单外）。

修改核心cli/lua api/的时候 同步修改 [LUA_API.md](/docs/LUA_API.md)

修改 AI 工具链（`cli/crates/cc-hooks/`、`tree/home/.claude/`、`tree/home/.agents/`、`cc-hook-test`）的时候 同步修改 [AI_TOOLING.md](/docs/AI_TOOLING.md)

## 核心命令

```bash
# 开发期透传（cargo run --release，自动编译）
./dots.sh sync            # 把 tree/ 链接到 $HOME（幂等，可反复跑）
./dots.sh status          # 三态巡检：已链 / 漂移 / 缺失
./dots.sh adopt <path>    # 把 $HOME 里现成的文件收进仓库管理
./dots.sh doctor          # 体检（漂移/未覆盖主机/脚本冲突）

# 正式安装：bootstrap.sh 编译 release 产物
# 新机：git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh

# 日常跳转（zsh alias，由 .zshrc_dotfiles 提供）
dot                       # cd 到仓库根
skill                     # cd 到 skills 目录
```

## 目录架构

```
dots.lua          # 例外清单（人手编辑，LuaLS 类型补全见 .luarc.json；唯一例外：bootstrap 的 host 引导会插入 host 块）
cli/              # Rust workspace：dots-core（纯逻辑）+ dots（bin）+ cc-hooks（bin: cc-hook，Claude hooks 入口），lua-api/（类型标注）
tree/             # ★ 映射根：目录结构即链接声明
  home/           #   → $HOME（跨平台）
  home.linux/     #   → $HOME（仅 Linux，条目级覆盖通用层）
  home.macos/     #   → $HOME（仅 macOS）
scripts/          # 脚本源（common/ linux/ macos/），聚合到 .gen/scripts/ 进 PATH
hosts/            # per-host 资产：files/<host>/、secrets.age（age 加密，密文入库）
system/           # root 级文件源（udev/systemd），dots 不链接，手动 cp 到 /etc（见 docs/MANUAL_SETUP.md）
packages/         # bootstrap 装机清单（pacman.txt 等纯文本 + toolchains.toml）
common/           # 手动同步参考资料（VS Code 配置；dots 不处理）
docs/
.gen/  (gitignore) # 派生区：scripts/（聚合软链）、injected/（minijinja 渲染产物）
.dots/ (gitignore) # state.json 链接台账（undo/unlink/漂移检测用）
backup/ (gitignore)# 覆盖普通文件前的时间戳备份
```

## 映射规则（dots-core 的核心，替代旧 install.py 分支）

1. **纯 $HOME 镜像**：`tree/home/X` → `$HOME/X`；`tree/home.<os>/X` 仅该平台生效，条目级覆盖通用层。
2. **链接粒度启发式**：文件直接链；层根的一级目录是「容器」（下钻、逐子项链）；二级及更深目录整目录链。
3. **覆盖**：启发式不对时在 `dots.lua` 写 `granularity(path, {mode=…, ignore=…})`。
4. **链接判定**：目标已是指向仓库内的链接（含旧路径、断链）→ 无条件重建；真实文件 → 备份后链；指向仓库外 → 报漂移不动。

## dots.lua（例外清单）

只写约定盖不住的：`granularity`（粒度覆盖，spec 可带条目级 `pre`/`post` 钩子，pre 返回 false 跳过该条目）、`distribute`（一源多落点，如 `tree/home/.agents/skills` → claude/codex，同样支持 `pre`/`post`；AI skills/agents/commands 源统一住 `tree/home/.agents/`，各工具落点全走 distribute 订阅）、`systemd_user`（sync 时 `systemctl --user enable`）、`scripts{ignore_tree=…}`（子目录默认保树形，列出的才拍平）、`hosts{<name>=function() vars{…}; link(…); toolchains{only|skip={…}} end}`（per-host；`toolchains` 圈定 bootstrap 工具链组，组名 = toolchains.toml 节头，服务器 `only={"core"}`）、`on{phase=fn|{fn,…}}`（全局生命周期钩子：pre_sync/on_host_activate/post_link/post_sync）。CLI 基本不编辑它（需要时打印建议行让你粘贴）；**唯一例外**：`dots bootstrap` 在未登记主机 + 交互终端时跑 host 引导（`onboard.rs`），问别名/工具链组后把 host 块插进现有 `hosts({` 下方，并把真实主机名写进机器本地 `~/.config/dots/host`（不入 git，`hosts::current()` 优先读它匹配别名块——B 方案）。host 未命中现为非致命（warn + 继续链通用项），不再硬报错。

## 路径注入（已消灭模板渲染）

- 配置只引用「安装后路径」（`$HOME` 侧）或自身相对路径。`$DOTFILES_DIR` 只指仓库本身。
- `dots sync` 写 `~/.config/dots/env.zsh`（export `DOTFILES_DIR`/`DOTS_SCRIPTS` + PATH），`.zshrc_dotfiles` 首行 source 它。**不再有 `*_TEMPLATE` 占位符渲染**。
- 读不到 shell 环境的消费者（systemd unit）才渲染：`.inject` 后缀 + minijinja `{{ }}`，产物落 `.gen/injected/`。
- Hyprland：`hyprland.lua` 读 `os.getenv("DOTFILES_DIR")`，兜底读 `~/.config/dots/root`。

## shell 栈（Oh My Zsh 已退役）

- 无框架，显式 emacs 键模式（`bindkey -e`；解绑遗留四键后 Ctrl-S 已启用为 live-grep、Ctrl-Q 为 scratch，W/U 留白）。模块在 `tree/home/.config/zsh/`：`10-options.zsh`（历史/目录/补全/键绑定）、`20-functions.zsh`（cd-ls/allclear/copypath/copyfile/proxy/sc 临时文件 内联，剪贴板走 `_dots_clipcopy` 平台分派）、`25-fzf-tab.zsh`（fzf 键绑定 Ctrl-R、Alt-C + 自写 widget Ctrl-F 找文件/Ctrl-S live-grep 选中直接进 nvim（官方 Ctrl-T 路径插入已禁用）+ fzf-tab 补全菜单，冻结 vendor；顺序敏感——compinit 之后、autosuggestions 之前、fzf --zsh 之后）、`30-autosuggestions.zsh`（历史内联建议，冻结 vendor）、`40-aliases.zsh`（别名与交互函数，原 `~/.alias` 已退役迁入；**头部有 CLAUDECODE Agent 守卫**，agent 环境不加载）、`90-syntax-highlighting.zsh`（z-sy-h 冻结 vendor）。
- `z` → zoxide；提示符 starship + 自写 transient prompt。
- git diff 走 delta（`tree/home/.gitconfig` 受管，syntax-theme 复用 bat 主题库）；bat 主题 Catppuccin Mocha（`tree/home/.config/bat/config`），与 kitty/fzf 同色板。
- 改主配置改 `tree/home/.zshrc_dotfiles`；平台差异改 `.zshrc_linux`/`.zshrc_macos`。

## 修改配置时的约定

1. 新增/改配置：编辑 `tree/` 下对应文件，`dots sync`（多数情况是普通文件，改完直接生效，无需渲染）。
2. 新增整目录或非标目标：放进 `tree/` 即被镜像；`dots adopt <path>` 可把 `$HOME` 现成文件收编。
3. `~/.zshrc` 是受管 stub（首行 `# DOTS_MANAGED:`），只 source `~/.zshrc_dotfiles`，软件追加内容（conda/nvm）安全保留——不要当主配置维护。
4. root 级配置（keyboard inhibit）在 `system/`，需 root 手动安装，见 `docs/MANUAL_SETUP.md`。

## 备份机制

- 覆盖普通文件/目录前移到 `backup/<时间戳>/`；目标是符号链接则直接重建不备份。
- `dots sync --dry-run` 只预览不写盘。

## 当前仓库事实

- 顶层 `README.md` → `docs/README.md` 符号链接；`AGENTS.md` → `CLAUDE.md` 符号链接。
- 管理入口是 `dots`（Rust CLI），旧 `install.py` 已退役删除——描述安装方式以 `dots` / `bootstrap.sh` 为准。
- 不做日常包管理（无 pkg-install/pkg-export、无 Brewfile/pacman 清单的同步逻辑；`packages/` 仅供 `dots bootstrap` 装机）。
- `.gen/`、`.dots/` 是机器本地派生物，不入库。

## 主要配置工具

- Shell: Zsh（自管 conf.d，无框架）+ Starship
- 编辑器: Neovim / Vim；终端: Kitty；文件管理器: Yazi；多路复用: Zellij
- macOS: yabai / skhd / sketchybar / fcitx5
- Linux: Hyprland（主，0.55+ lua 入口）/ niri（备选）/ QuickShell（状态栏+控制中心）/ systemd user
