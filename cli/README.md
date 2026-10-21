# 本地程序

`cli/` 按职责分目录：`dots/` 管理 dotfiles，`programs/` 存放随仓库分发的
独立程序。`programs/` 不限定语言，各程序自行选择工具链和依赖管理方式。

```text
cli/
├── dots/
│   ├── Cargo.toml          # 仅管理 dots 与 dots-core
│   ├── Cargo.lock
│   ├── crates/
│   │   ├── dots/           # dotfiles 管理 CLI
│   │   └── dots-core/      # 声明解析与资源规划的纯逻辑
│   └── lua-api/            # dots.lua 的 LuaLS 类型标注
├── programs/
│   ├── agent-hooks/        # 独立 Cargo package；binary: agent-hook
│   └── window-borders/     # 独立 Cargo package；仅 macOS
├── rustfmt.toml            # Rust 项目的共享格式配置
└── clippy.toml             # Rust 项目的共享 Clippy 配置
```

## dots：管理仓库

`dots/` 拥有自己的 workspace 和 lockfile。`bootstrap.sh` 只构建这个
workspace，再执行 `dots install` 和 `dots sync`。

从仓库根运行：

```sh
./dots.sh --help
cargo build --locked --release --manifest-path cli/dots/Cargo.toml
```

## programs：随仓库安装的程序

新增程序时：

1. 将源码放在 `programs/<程序名>/`，使用该程序所需的语言和项目结构。
2. 在程序文档中说明依赖、构建、安装方式及支持的平台，再按实际需求接入仓库安装流程。
3. 在 `.pre-commit-config.yaml` 纳入该语言适用的检查；平台专属检查只在对应平台执行。

### 现有 Rust 程序

`agent-hooks` 和 `window-borders` 各自维护 Cargo manifest、lockfile、
构建产物和 lint 声明，不依赖 `dots` 的 workspace。
`cli/rustfmt.toml` 与 `cli/clippy.toml` 提供共享配置；Clippy 配置不替代
各 Cargo manifest 的 lint 级别声明。

这两个程序通过仓库根 `dots.lua` 的本地 Cargo 安装声明接入，例如：

```lua
dots.install.cargo({
  source = {
    path = "cli/programs/window-borders",
    binary = "window-borders",
  },
  root = "~/.local",
  platforms = { "macos" },
})
```

不限定平台的程序省略 `platforms`。`dots install` 按声明执行
`cargo install --locked --force --path … --bin … --root …`，不需要程序属于
任何共享 workspace。

```sh
./dots.sh install --dry-run
cargo build --locked --release --manifest-path cli/programs/agent-hooks/Cargo.toml
# 仅 macOS
cargo build --locked --release --manifest-path cli/programs/window-borders/Cargo.toml
```
