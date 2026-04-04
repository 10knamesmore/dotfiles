# Rust 项目内单独配置 rust-analyzer 编译 Feature

这份配置已经支持在项目根目录放 `.nvim.lua` 做本地覆盖，因此可以针对单个 Rust 项目单独调整 `rust-analyzer` 的 Cargo feature 行为。

相关基础配置：

- 已启用 `exrc`，允许读取项目根目录下的 `.nvim.lua`
- Rust 使用 `rustaceanvim`
- `rustaceanvim` 初始化时会优先保留 `vim.g.rustaceanvim` 中已有的配置，再补默认值

当前全局默认值里，`rust-analyzer` 的 Cargo 配置包含：

```lua
cargo = {
    allFeatures = true,
    loadOutDirsFromCheck = true,
    buildScripts = {
        enable = true,
    },
}
```

因此如果某个项目想改成“只编译指定 feature”，必须显式把 `allFeatures` 改成 `false`。

## 用法

在 Rust 项目根目录创建 `.nvim.lua`：

```lua
vim.g.rustaceanvim = {
    server = {
        default_settings = {
            ["rust-analyzer"] = {
                cargo = {
                    allFeatures = false,
                    noDefaultFeatures = true,
                    features = { "feat_a", "feat_b" },
                },
            },
        },
    },
}
```

保存后重新进入该项目，或在 Neovim 中执行：

```vim
:LspRestart
```

## 常见配置示例

### 1. 只启用指定 feature，保留默认 feature

```lua
vim.g.rustaceanvim = {
    server = {
        default_settings = {
            ["rust-analyzer"] = {
                cargo = {
                    allFeatures = false,
                    features = { "feat_a", "feat_b" },
                },
            },
        },
    },
}
```

### 2. 只启用指定 feature，并关闭默认 feature

```lua
vim.g.rustaceanvim = {
    server = {
        default_settings = {
            ["rust-analyzer"] = {
                cargo = {
                    allFeatures = false,
                    noDefaultFeatures = true,
                    features = { "feat_a", "feat_b" },
                },
            },
        },
    },
}
```

### 3. 不启用任何额外 feature

```lua
vim.g.rustaceanvim = {
    server = {
        default_settings = {
            ["rust-analyzer"] = {
                cargo = {
                    allFeatures = false,
                    features = {},
                },
            },
        },
    },
}
```

## 说明

- `.nvim.lua` 作用域是当前项目，不会影响别的 Rust 仓库
- 如果没有把 `allFeatures` 设为 `false`，那么全局默认的 `allFeatures = true` 仍然会生效
- 这里只覆盖 `cargo` 下需要改的字段，其他全局 `rust-analyzer` 配置会继续沿用默认值
- 如果项目里有多个 crate，`rust-analyzer` 仍会按 workspace 维度工作，这份配置影响的是当前 workspace 的分析方式
