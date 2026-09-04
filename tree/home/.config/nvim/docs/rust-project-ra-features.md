# Rust 工具链与项目 Feature 配置

## 工具归属

- rustup 管理 Rust、Cargo、rust-analyzer、rustfmt、Clippy 和 rust-src。
- rustaceanvim 是 Rust LSP 的唯一启动入口；nvim-lspconfig 不重复启动它。
- Rust 配置不要求 Mason 安装 rust-analyzer 或 CodeLLDB，且关闭 rustaceanvim 的 DAP 适配器和调试配置加载。
- 格式化沿用 Conform 的 LSP 路径，由 rust-analyzer 调用 rustfmt；保存时执行 Clippy 检查。

LSP 从项目根目录启动 `~/.cargo/bin/rust-analyzer`，其子进程优先使用同目录的 Rust 工具。路径随用户主目录展开，不写死用户名或工具链版本，也不依赖 Mason 的 PATH 顺序。

## 准备项目工具链

dotfiles 不自动安装 rustup。先安装 rustup，再为项目指定的工具链安装组件，例如：

```sh
rustup toolchain install 1.88.0 --profile minimal
rustup component add --toolchain 1.88.0 rust-analyzer rust-src rustfmt clippy
```

使用项目的 `rust-toolchain.toml` 固定版本；不需要修改全局默认版本。缺少组件时显式安装，不自动换用其他工具链。

在项目根目录核对实际版本：

```sh
rustup show active-toolchain
rustc --version
cargo --version
rust-analyzer --version
rustfmt --version
cargo clippy --version
```

更换工具链或 LSP 启动配置后，重启 Neovim。rustaceanvim 会复用已启动的 Rust LSP；不同工具链的项目使用独立 Neovim 会话。

## 项目内 Feature 覆盖

已启用 `exrc`，项目根目录的 `.nvim.lua` 可设置 `vim.g.rustaceanvim`，已有值优先于全局默认值。全局 Cargo 配置使用 `features = "all"`，启用 build scripts 和 `loadOutDirsFromCheck`。

仅启用指定 feature 并禁用默认 feature：

```lua
vim.g.rustaceanvim = {
  server = {
    default_settings = {
      ["rust-analyzer"] = {
        cargo = {
          features = { "feat_a", "feat_b" },
          noDefaultFeatures = true,
        },
      },
    },
  },
}
```

保留默认 feature 时删除 `noDefaultFeatures`；不启用额外 feature 时设置 `features = {}`。无需设置旧字段 `allFeatures`。从该项目根目录重启 Neovim 后生效，配置作用于整个 Cargo workspace。
