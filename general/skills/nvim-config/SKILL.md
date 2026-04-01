---
name: nvim-config
description: 处理 Neovim 配置相关问题，包括插件调试、选项诊断、Lua 配置编写、LSP 配置、键位映射等。
---

# Neovim 配置助手

## 目标

协助用户诊断和解决 Neovim 配置问题，包括但不限于：插件行为异常、LSP 配置、键位映射、选项设置、colorscheme、自动命令、用户命令等。

## 核心原则

**先读源码，再下结论。** 不要依赖记忆或猜测插件的行为，直接阅读安装在本机的插件源码。

## 调查流程

### 1) 明确问题

- 让用户描述具体症状（报错信息、异常行为、期望行为）。
- 询问相关插件名称或配置文件路径（如果未明确给出）。

### 2) 定位插件源码

插件安装目录：`/Users/wanger/.local/share/nvim/`

常见子路径：
- lazy.nvim 管理的插件：`/Users/wanger/.local/share/nvim/lazy/<plugin-name>/`
- Mason 安装的工具：`/Users/wanger/.local/share/nvim/mason/`

**操作步骤：**
1. 用 Glob 或 Grep 在上述路径中定位相关文件。
2. 阅读插件的 `lua/`、`plugin/`、`doc/` 目录。
3. 重点关注：公开 API、默认配置、setup 函数签名、已知的 breaking change。

### 3) 阅读用户配置

配置根目录：`~/.config/nvim/`，Lua 配置主目录：`~/.config/nvim/lua/`。先读用户的实际配置，理解现有设置，再提建议。

查插件精确版本：`~/.config/nvim/lazy-lock.json`，比任何其他来源都可靠。

### 4) 诊断与修复

- 对比插件文档（`doc/*.txt`）与用户配置，找出差异。
- 优先提供最小改动的修复方案。
- 若涉及多个插件交互，逐一排查加载顺序与配置时机。

### 5) 棘手问题：克隆 Neovim 源码

当问题涉及 Neovim **核心行为**（内置函数、API、事件系统、默认选项等）且文档不足以解答时：

```bash
# 克隆 Neovim 源码（稳定版）
git clone --depth=1 --branch stable https://github.com/neovim/neovim /tmp/neovim-src

# 或克隆最新 nightly
git clone --depth=1 https://github.com/neovim/neovim /tmp/neovim-src
```

重点阅读：
- `runtime/lua/vim/` — 内置 Lua 模块（`lsp/`、`treesitter/`、`diagnostic.lua` 等）
- `src/nvim/` — C 核心实现
- `runtime/doc/` — 官方文档源文件

## 常用调查命令

在 Neovim 内部诊断（可指导用户执行）：

```vim
" 查看某选项当前值与来源
:verbose set <option>?

" 查看键位映射
:verbose map <key>

" 查看加载的运行时文件
:scriptnames

" 检查 LSP 状态
:LspInfo
:checkhealth lsp

" 查看所有已注册的自动命令
:autocmd <event>

" 运行时快速求值（调试配置）
:lua =vim.o.someopt
:lua =vim.api.nvim_get_keymap('n')
```

## 参考文档

- lazy.nvim 配置最佳实践：`references/lazy-nvim-best-practices.md`
  - 遇到 lazy.nvim spec 写法、懒加载、opts merge、cond/enabled 等问题时加载此文档。

## 注意事项

- 不要凭记忆描述插件 API，必须先读源码确认。
- 插件版本可能与文档不符，以本机安装版本为准。
- 提修复方案前，先确认用户的 Neovim 版本（`nvim --version`）是否满足插件要求。
- 涉及 `vim.keymap.set`、`vim.opt`、`vim.api` 等 Neovim API 时，优先查阅 `:help` 文档或 Neovim 源码中的 `runtime/lua/vim/`。
