## 项目概述

这是一个跨平台 dotfiles 仓库。管理入口是自写的 Rust CLI **`dots`**（源码在 `cli/dots/`）。

核心机制是**声明式 Resource 管理**：仓库即配置的单一真相源——`tree/` 镜像 `$HOME`，`dots.lua` 声明显式 Resource。

## 核心命令

```bash
./dots.sh install         # 补装当前平台系统包，再安装或升级 Cargo binary，不卸载
./dots.sh install --dry-run # 查询系统包状态并预览安装命令，不执行安装
./dots.sh sync            # 执行 lifecycle hook，再把 tree/ 链接到 $HOME
./dots.sh status          # 只读展示 sync 将执行的完整 Plan
./dots.sh forget <资源>   # 不改真实对象，只放弃对应 ownership

# 正式安装：bootstrap.sh 编译 release 产物
# 新机：git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh
```

## 目录架构

```
dots.lua          # 例外清单（人手编辑，LuaLS 类型补全见 .luarc.json）
cli/              # 按职责组织管理工具与独立程序，各项目自行构建
  dots/           # 专属 workspace：crates/dots + crates/dots-core，lua-api/ 类型标注
  programs/       # 随仓库分发的独立程序，不限定语言、工具链或依赖管理方式
    agent-hooks/  # 跨平台，binary: agent-hook
    desktop-backend/ # 仅 Linux，Quickshell 的截图、录屏与状态采集后端
    window-borders/ # 仅 macOS，binary: window-borders
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

## dots.lua（例外清单）

只写约定盖不住的：`granularity`（粒度覆盖）、`distribute`（一源多落点；共享 AI skills/agents/commands 源统一住 `tree/home/.agents/`）、`scripts{ignore_tree=…}`（子目录默认保树形，列出的才拍平）、`dots.hook.before_sync`（真实 sync 在 planning 前执行的具名程序）、`dots.resource.directory|symlink|copied_file|managed_block|systemd_user_unit`（显式持续 Resource）、`dots.install.pacman|brew|cargo`（只由 `dots install` 执行）、`dots.path.exists`（只读条件）。条目级 `pre`/`post`、任意 shell Action、`dots.file.*`、`dots.cargo.build`、`dots.json.*` 与 toolchain group 均不存在。CLI 不编辑 `dots.lua`。

`directory({target="~/.local/state/kitty", mode="0700"})` 只管理真实目录本身，允许独立声明其子资源。`mode` 是可选的四位八进制字符串：声明后持续收敛权限；省略时保留已有权限，新建遵循 umask。目标为文件或 symlink 时报告冲突，不覆盖。删除声明后，只删除权限仍匹配 Applied 的空目录；本轮计划删除的子资源先于父目录删除。仍有其他内容、无法列出内容或仍承载 Desired 子路径时，显示 `retain`，保留目录与 ownership，sync/status 返回非零；可清理内容后再同步，或用 `dots forget` 放弃 ownership。目录内容从不递归删除。

安装清单按来源分别声明，Lua 无需判断平台：`pacman({...})` 仅在存在 `/etc/arch-release` 的 Linux 上执行，`brew({...})` 仅在 macOS 上执行，其他平台显示跳过。当前平台的管理器不可用或执行失败会报错，不切换来源。两者只补装缺失包，不主动升级已有声明包；pacman 不刷新数据库或执行全系统升级。删除安装声明不卸载软件。

`cargo({source=..., platforms={"macos"}})` 可限定执行平台，只接受 `linux`、`macos`；省略 `platforms` 时在所有平台执行，空表表示全部跳过。字符串 source 表示 crates.io package，table source 表示本地 Cargo package 的 binary，后者每次强制重建，不要求属于 workspace。平台不匹配时显示跳过，不调用 Cargo。

## 路径注入

- 配置只引用「安装后路径」（`$HOME` 侧）或自身相对路径。`$DOTFILES_DIR` 只指仓库本身。
- `dots sync` 写 `~/.config/dots/env.zsh`（export `DOTFILES_DIR`/`DOTS_SCRIPTS` + PATH），`.zshrc_dotfiles` 首行 source 它。配置不使用 `*_TEMPLATE` 占位符。
- 读不到 shell 环境的消费者（systemd unit）才渲染：`.inject` 后缀 + minijinja `{{ DOTFILES }}` / `{{ SCRIPTS }}`，产物落 `.gen/injected/`。

## 修改配置时的约定

1. 新增/改配置：编辑 `tree/` 下对应文件，`dots sync`（多数情况是普通文件，改完直接生效，无需渲染）。
2. 新增整目录或非标目标：把源文件放进 `tree/` 对应位置，再运行 `dots sync`。
3. `~/.zshrc` 由 `dots-env` managed block 注入 `source ~/.zshrc_dotfiles`，block 外的软件内容（conda/nvm）始终保留——不要当主配置维护。
   `dots sync --dry-run` 显示但不执行 lifecycle hook，也不修改受管 target 或 Applied Inventory；`dots status` 不执行或显示 hook。

## 仓库约定

- 这是个人 dotfiles 仓库**不是公开生产项目**，**禁止**编写测试,**禁止**考虑兼容, **禁止**考虑`这是涉及公开API修改, 所以得慎重` 用户明确了就直接改 。改动使用编译、类型检查和必要的真实运行验证。
- 仓库内所有东西只考虑 macos & linux 的使用场景
- 配置管理入口是 `dots`，安装入口是 `bootstrap.sh`。
- `dots install` 安装显式声明的系统包与 Cargo binary；不安装包管理器本身、rustup、Node、pnpm 或 AI CLI。`bootstrap.sh` 要求本机已有 `cc`、Cargo、Node 22.19+ 与 pnpm 11.18，系统包管理器仅在有适用声明时需要；依次运行 `dots install` 和 `dots sync`。Pi SDK 依赖跟随 npm `latest`；sync 的 Pi hook 执行 `pnpm run update:pi`，更新已解析版本并写入 lockfile。
- `.gen/`、`.dots/` 是机器本地派生物，不入库。
