## 项目概述

这是一个跨平台 dotfiles 仓库。管理入口是自写的 Rust CLI **`dots`**（源码在 `cli/`）。

核心机制是**声明式 Resource 管理**：仓库即配置的单一真相源——`tree/` 镜像 `$HOME`，`dots.lua` 声明显式 Resource。

## 核心命令

```bash
./dots.sh install         # 执行声明的 Cargo binary 安装或升级，不卸载
./dots.sh sync            # 执行 lifecycle hook，再把 tree/ 链接到 $HOME
./dots.sh status          # 只读展示 sync 将执行的完整 Plan
./dots.sh forget <资源>   # 不改真实对象，只放弃对应 ownership

# 正式安装：bootstrap.sh 编译 release 产物
# 新机：git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh
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

## dots.lua（例外清单）

只写约定盖不住的：`granularity`（粒度覆盖）、`distribute`（一源多落点；共享 AI skills/agents/commands 源统一住 `tree/home/.agents/`）、`scripts{ignore_tree=…}`（子目录默认保树形，列出的才拍平）、`dots.hook.before_sync`（真实 sync 在 planning 前执行的具名程序）、`dots.resource.symlink|copied_file|managed_block|systemd_user_unit`（显式持续 Resource）、`dots.resource.cargo_binary`（只由 `dots install` 直接映射为 `cargo install` 的声明）、`dots.path.exists`（只读条件）。字符串 `cargo_binary.source` 表示 crates.io package；删除声明不卸载已经安装的 binary。条目级 `pre`/`post`、任意 shell Action、`dots.file.*`、`dots.cargo.build`、`dots.json.*` 与 toolchain group 均不存在。CLI 不编辑 `dots.lua`。

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
- 不安装系统包、rustup、Node、pnpm 或 AI CLI；`bootstrap.sh` 要求本机已有 `cc`、Cargo、Node 22.19+ 与 pnpm 11.18，依次运行 `dots install` 和 `dots sync`。Pi SDK 依赖跟随 npm `latest`；sync 的 Pi hook 执行 `pnpm run update:pi`，更新已解析版本并写入 lockfile。
- `.gen/`、`.dots/` 是机器本地派生物，不入库。
