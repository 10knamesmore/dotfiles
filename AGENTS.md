## 项目概述

这是一个跨平台 dotfiles 仓库。管理入口是自写的 Rust CLI **`dots`**（源码在 `cli/`）。

核心机制是**声明式 Resource 管理**：仓库即配置的单一真相源——`tree/` 镜像 `$HOME`，`dots.lua` 声明显式 Resource。仓库不安装系统包、rustup、Node、pnpm 或 AI CLI；`dots install` 只执行声明的 Cargo binary 安装或升级，Cargo binary 不进入 sync lifecycle。`dots sync` 在 planning 前执行声明的 lifecycle hook，再收敛 Resource。

修改核心 CLI 或 Lua API 时，同步更新 [LUA_API.md](docs/LUA_API.md)。

## 核心命令

```bash
# 开发期透传（cargo run --release，自动编译）
./dots.sh install         # 执行声明的 Cargo binary 安装或升级，不卸载
./dots.sh sync            # 执行 lifecycle hook，再把 tree/ 链接到 $HOME
./dots.sh status          # 只读展示 sync 将执行的完整 Plan
./dots.sh forget <资源>   # 不改真实对象，只放弃对应 ownership

# 正式安装：bootstrap.sh 编译 release 产物
# 新机：git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh

# 日常跳转（zsh alias，由 .zshrc_dotfiles 提供）
dot                       # cd 到仓库根
skill                     # cd 到 skills 目录
```

## 目录架构

```
dots.lua          # 例外清单（人手编辑，LuaLS 类型补全见 .luarc.json）
cli/              # Rust workspace：dots-core（纯逻辑）+ dots（bin）+ agent-hooks（bin: agent-hook，多 agent hooks），lua-api/（类型标注）
pi/               # vanilla Pi 的 TypeScript extension source 与 pnpm workspace
tree/             # ★ 映射根：目录结构即链接声明
  home/           #   → $HOME（跨平台）
  home.linux/     #   → $HOME（仅 Linux，条目级覆盖通用层）
  home.macos/     #   → $HOME（仅 macOS）
scripts/          # 脚本源（common/ linux/ macos/），聚合到 .gen/scripts/ 进 PATH
docs/
.gen/  (gitignore) # 派生区：scripts/（聚合软链）、injected/（minijinja 渲染产物）
.dots/ (gitignore) # state.json Applied Inventory（声明删除与 Drift 判断依据）
```

## 映射规则

1. **纯 $HOME 镜像**：`tree/home/X` → `$HOME/X`；`tree/home.<os>/X` 仅该平台生效，条目级覆盖通用层。
2. **链接粒度启发式**：文件直接链；层根的一级目录是「容器」（下钻、逐子项链）；二级及更深目录整目录链。
3. **覆盖**：启发式不对时在 `dots.lua` 写 `granularity(path, {mode=…, ignore=…})`。
4. **三方收敛**：Desired Set + Applied Inventory + Observed State 生成唯一 Plan。未拥有且完全一致的 target 自动接管；不同则 Collision。Declaration 删除时只清理仍匹配 Applied 状态的 Resource，漂移内容保留。

## dots.lua（例外清单）

只写约定盖不住的：`granularity`（粒度覆盖）、`distribute`（一源多落点；AI skills/agents/commands 源统一住 `tree/home/.agents/`）、`scripts{ignore_tree=…}`（子目录默认保树形，列出的才拍平）、`dots.hook.before_sync`（真实 sync 在 planning 前执行的具名程序）、`dots.resource.symlink|copied_file|managed_block|systemd_user_unit`（显式持续 Resource）、`dots.resource.cargo_binary`（只由 `dots install` 直接映射为 `cargo install` 的声明）、`dots.path.exists`（只读条件）。字符串 `cargo_binary.source` 表示 crates.io package；删除声明不卸载已经安装的 binary。条目级 `pre`/`post`、任意 shell Action、`dots.file.*`、`dots.cargo.build`、`dots.json.*` 与 toolchain group 均不存在。CLI 不编辑 `dots.lua`。

## 路径注入

- 配置只引用「安装后路径」（`$HOME` 侧）或自身相对路径。`$DOTFILES_DIR` 只指仓库本身。
- `dots sync` 写 `~/.config/dots/env.zsh`（export `DOTFILES_DIR`/`DOTS_SCRIPTS` + PATH），`.zshrc_dotfiles` 首行 source 它。配置不使用 `*_TEMPLATE` 占位符。
- 读不到 shell 环境的消费者（systemd unit）才渲染：`.inject` 后缀 + minijinja `{{ DOTFILES }}` / `{{ SCRIPTS }}`，产物落 `.gen/injected/`。
- Hyprland：`hyprland.lua` 读 `os.getenv("DOTFILES_DIR")`，兜底读 `~/.config/dots/root`。

## 按任务查阅

- 修改 shell 配置时，查阅 [Zsh 结构](docs/README.md#zsh-结构) 中的加载顺序与键绑定约定。
- 修改 agent 指令、skill 或 hook 时，查阅 [AI 工具链](docs/README.md#ai-工具链skills--agents--hooks)，编辑 `tree/home/.agents/` 下的源文件。
- 配置工具概览见 [README](docs/README.md#当前管理的主要配置)。

## 修改配置时的约定

1. 新增/改配置：编辑 `tree/` 下对应文件，`dots sync`（多数情况是普通文件，改完直接生效，无需渲染）。
2. 新增整目录或非标目标：把源文件放进 `tree/` 对应位置，再运行 `dots sync`。
3. `~/.zshrc` 由 `dots-env` managed block 注入 `source ~/.zshrc_dotfiles`，block 外的软件内容（conda/nvm）始终保留——不要当主配置维护。
`dots sync --dry-run` 显示但不执行 lifecycle hook，也不修改受管 target 或 Applied Inventory；`dots status` 不执行或显示 hook。

## 仓库约定

- 顶层 `README.md` → `docs/README.md` 符号链接；`AGENTS.md` 是项目指令的真实源文件。
- 配置管理入口是 `dots`，安装入口是 `bootstrap.sh`。
- 不安装系统包、rustup、Node、pnpm 或 AI CLI；`bootstrap.sh` 要求本机已有 `cc`、Cargo、Node 22.19+ 与 pnpm 11.18，依次运行 `dots install` 和 `dots sync`。sync 的 Pi hook 按 frozen lockfile 安装 workspace 依赖。
- `.gen/`、`.dots/` 是机器本地派生物，不入库。
