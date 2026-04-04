# Snippet Engine 完全解析

<!--toc:start-->
- [Snippet Engine 完全解析](#snippet-engine-完全解析)
  - [🎯 什么是 Snippet Engine？](#🎯-什么是-snippet-engine)
    - [简单类比](#简单类比)
  - [🔍 在 Neogen 中的作用](#🔍-在-neogen-中的作用)
    - [没有 Snippet Engine 时](#没有-snippet-engine-时)
    - [有 Snippet Engine 时](#有-snippet-engine-时)
  - [🛠️ 常见的 Snippet Engines](#🛠️-常见的-snippet-engines)
    - [1. **LuaSnip** （最流行，推荐）](#1-luasnip-最流行推荐)
    - [2. **nvim** （Neovim 原生，0.10+ 内置）](#2-nvim-neovim-原生010-内置)
    - [3. **snippy**](#3-snippy)
    - [4. **vsnip**](#4-vsnip)
    - [5. **mini.snippets**](#5-minisnippets)
  - [📊 对比表格](#📊-对比表格)
  - [🎬 实际演示](#🎬-实际演示)
    - [场景：填写 Rust 函数注释](#场景填写-rust-函数注释)
      - [Step 1: 生成注释](#step-1-生成注释)
      - [Step 2: 填写第1个占位符](#step-2-填写第1个占位符)
      - [Step 3: 按 `Tab` 跳到下一个](#step-3-按-tab-跳到下一个)
      - [Step 4: 继续填写](#step-4-继续填写)
      - [Step 5: 依次填完所有占位符](#step-5-依次填完所有占位符)
  - [🚀 配置示例](#🚀-配置示例)
    - [完整配置（使用 LuaSnip）](#完整配置使用-luasnip)
    - [简单配置（使用 Neovim 原生）](#简单配置使用-neovim-原生)
  - [❓ 不使用 Snippet Engine 会怎样？](#不使用-snippet-engine-会怎样)
    - [方案 1: 使用 Neogen 内置跳转](#方案-1-使用-neogen-内置跳转)
    - [方案 2: 完全手动填写](#方案-2-完全手动填写)
  - [🎯 推荐方案](#🎯-推荐方案)
    - [初学者 / 轻度使用](#初学者-轻度使用)
    - [进阶用户](#进阶用户)
    - [极简主义者](#极简主义者)
  - [🔧 调试技巧](#🔧-调试技巧)
    - [检查 Snippet Engine 是否工作](#检查-snippet-engine-是否工作)
    - [常见问题](#常见问题)
      - [问题 1: 按 Tab 没反应](#问题-1-按-tab-没反应)
      - [问题 2: LuaSnip 未加载](#问题-2-luasnip-未加载)
  - [📝 总结](#📝-总结)
    - [Snippet Engine 是什么？](#snippet-engine-是什么)
    - [在 Neogen 中的作用？](#在-neogen-中的作用)
    - [我应该用哪个？](#我应该用哪个)
    - [快速开始](#快速开始)
<!--toc:end-->

## 🎯 什么是 Snippet Engine？

**Snippet Engine（代码片段引擎）**是一个专门用于管理和扩展**代码片段（code snippets）**的插件系统。

### 简单类比

想象你在填写一个表单：

```
姓名：[_____]
年龄：[_____]
地址：[_____]
```

**Snippet Engine** 就是帮你：
1. 按 `Tab` 键跳到下一个空格 `[_____]`
2. 高亮显示当前要填的位置
3. 记住你填过哪些，还有哪些没填

---

## 🔍 在 Neogen 中的作用

### 没有 Snippet Engine 时

Neogen 生成注释后：

```rust
/// [TODO:description]
///
/// # Params:
/// - `param1`: [TODO:parameter]
/// - `param2`: [TODO:parameter]
```

**问题**：
- ❌ 你需要**手动**用鼠标点击或上下键移动到每个 `[TODO:xxx]`
- ❌ 不知道还有多少个位置需要填写
- ❌ 填写效率低

### 有 Snippet Engine 时

```rust
/// █              ← 光标在这里，高亮显示
///
/// # Params:
/// - `param1`: [TODO:parameter]
/// - `param2`: [TODO:parameter]
```

**好处**：
- ✅ 按 `Tab` → 自动跳到下一个 `[TODO:parameter]`
- ✅ 按 `Shift+Tab` → 跳回上一个位置
- ✅ 当前位置高亮显示
- ✅ 跳出所有占位符后自动结束

---

## 🛠️ 常见的 Snippet Engines

### 1. **LuaSnip** （最流行，推荐）

**特点**：
- 🚀 纯 Lua 编写，速度快
- 🎨 功能强大，支持复杂片段
- 🔌 与 nvim-cmp 完美集成

**安装**：
```lua
{
    "L3MON4D3/LuaSnip",
    dependencies = { "rafamadriz/friendly-snippets" }, -- 可选：常用片段集合
}
```

**Neogen 配置**：
```lua
require('neogen').setup({
    snippet_engine = "luasnip"
})
```

### 2. **nvim** （Neovim 原生，0.10+ 内置）

**特点**：
- ✨ Neovim 0.10+ 自带，无需额外插件
- 🪶 轻量，功能基础
- 🎯 简单场景够用

**Neogen 配置**：
```lua
require('neogen').setup({
    snippet_engine = "nvim"  -- 无需安装额外插件
})
```

### 3. **snippy**

**特点**：
- 🎯 轻量级替代品
- 📦 比 LuaSnip 简单

**安装**：
```lua
{ "dcampos/nvim-snippy" }
```

**Neogen 配置**：
```lua
require('neogen').setup({
    snippet_engine = "snippy"
})
```

### 4. **vsnip**

**特点**：
- 🔄 兼容 VSCode 片段格式
- 📚 可以使用 VSCode 的片段库

**安装**：
```lua
{ "hrsh7th/vim-vsnip" }
```

**Neogen 配置**：
```lua
require('neogen').setup({
    snippet_engine = "vsnip"
})
```

### 5. **mini.snippets**

**特点**：
- 🧩 mini.nvim 生态系统的一部分
- 🎨 现代化设计

**安装**：
```lua
{ "echasnovski/mini.nvim" }
```

**Neogen 配置**：
```lua
require('neogen').setup({
    snippet_engine = "mini"
})
```

---

## 📊 对比表格

| 引擎 | 安装难度 | 功能 | 性能 | 推荐场景 |
|------|---------|------|------|---------|
| **LuaSnip** | 中等 | ⭐⭐⭐⭐⭐ | ⚡⚡⚡ | 重度使用者，需要自定义片段 |
| **nvim** | 无需安装 | ⭐⭐⭐ | ⚡⚡⚡ | 简单需求，不想装插件 |
| **snippy** | 简单 | ⭐⭐⭐⭐ | ⚡⚡⚡ | 中等需求，追求简洁 |
| **vsnip** | 简单 | ⭐⭐⭐⭐ | ⚡⚡ | 从 VSCode 迁移 |
| **mini** | 简单 | ⭐⭐⭐⭐ | ⚡⚡⚡ | 使用 mini.nvim 生态 |

---

## 🎬 实际演示

### 场景：填写 Rust 函数注释

#### Step 1: 生成注释
光标在函数内，执行 `:Neogen func`

```rust
/// █              ← 第1个占位符（描述）
///
/// # Params:
/// - `numbers`: [TODO:parameter]
/// - `multiplier`: [TODO:parameter]
///
/// # Return
/// [TODO:return]
fn calculate(numbers: Vec<i32>, multiplier: i32) -> i32 {
    // ...
}
```

#### Step 2: 填写第1个占位符
输入：`计算数字总和并乘以倍数`

```rust
/// 计算数字总和并乘以倍数█
///
/// # Params:
/// - `numbers`: [TODO:parameter]
/// - `multiplier`: [TODO:parameter]
```

#### Step 3: 按 `Tab` 跳到下一个
```rust
/// 计算数字总和并乘以倍数
///
/// # Params:
/// - `numbers`: █              ← 第2个占位符
/// - `multiplier`: [TODO:parameter]
```

#### Step 4: 继续填写
输入：`要计算的整数数组`，按 `Tab`

```rust
/// 计算数字总和并乘以倍数
///
/// # Params:
/// - `numbers`: 要计算的整数数组
/// - `multiplier`: █           ← 第3个占位符
```

#### Step 5: 依次填完所有占位符
最终结果：

```rust
/// 计算数字总和并乘以倍数
///
/// # Params:
/// - `numbers`: 要计算的整数数组
/// - `multiplier`: 乘数因子
///
/// # Return
/// 计算后的总和
fn calculate(numbers: Vec<i32>, multiplier: i32) -> i32 {
    // ...
}
```

**全程只需要**：
- 打字填内容
- 按 `Tab` 跳转
- 无需移动鼠标或方向键！

---

## 🚀 配置示例

### 完整配置（使用 LuaSnip）

```lua
-- 1. 安装 LuaSnip
{
    "L3MON4D3/LuaSnip",
    dependencies = { "rafamadriz/friendly-snippets" },
    config = function()
        require("luasnip.loaders.from_vscode").lazy_load()
    end,
}

-- 2. 配置 Neogen
{
    "danymat/neogen",
    config = function()
        require('neogen').setup({
            snippet_engine = "luasnip",  -- 使用 LuaSnip
            enable_placeholders = true,  -- 启用占位符
        })
        
        vim.keymap.set("n", "<leader>nf", ":Neogen func<CR>")
    end,
}

-- 3. 配置跳转键（与 nvim-cmp 集成）
{
    "hrsh7th/nvim-cmp",
    config = function()
        local cmp = require('cmp')
        local luasnip = require('luasnip')
        
        cmp.setup({
            mapping = {
                ["<Tab>"] = cmp.mapping(function(fallback)
                    if luasnip.expand_or_jumpable() then
                        luasnip.expand_or_jump()
                    else
                        fallback()
                    end
                end, { "i", "s" }),
                
                ["<S-Tab>"] = cmp.mapping(function(fallback)
                    if luasnip.jumpable(-1) then
                        luasnip.jump(-1)
                    else
                        fallback()
                    end
                end, { "i", "s" }),
            },
        })
    end,
}
```

### 简单配置（使用 Neovim 原生）

```lua
{
    "danymat/neogen",
    config = function()
        require('neogen').setup({
            snippet_engine = "nvim",  -- 使用原生引擎（0.10+）
        })
        
        vim.keymap.set("n", "<leader>nf", ":Neogen func<CR>")
    end,
}
```

---

## ❓ 不使用 Snippet Engine 会怎样？

### 方案 1: 使用 Neogen 内置跳转

```lua
require('neogen').setup({
    snippet_engine = nil,  -- 不使用外部引擎
})

-- 手动配置跳转键
vim.keymap.set("i", "<C-l>", function()
    require('neogen').jump_next()
end, { desc = "下一个占位符" })

vim.keymap.set("i", "<C-h>", function()
    require('neogen').jump_prev()
end, { desc = "上一个占位符" })
```

**效果**：
- ✅ 可以跳转
- ❌ 没有高亮
- ❌ 功能较弱

### 方案 2: 完全手动填写

```lua
require('neogen').setup({
    snippet_engine = nil,
    enable_placeholders = false,  -- 禁用占位符
})
```

**生成结果**：
```rust
/// 
///
/// # Params:
/// - `numbers`: 
/// - `multiplier`: 
```

**需要**：
- 手动移动光标到每个位置填写
- 效率最低

---

## 🎯 推荐方案

### 初学者 / 轻度使用

```lua
snippet_engine = "nvim"  -- Neovim 0.10+ 内置
```

**理由**：
- ✅ 无需额外安装
- ✅ 功能够用
- ✅ 配置简单

### 进阶用户

```lua
snippet_engine = "luasnip"  -- 最强大
```

**理由**：
- ✅ 功能最全
- ✅ 社区最活跃
- ✅ 可自定义片段

### 极简主义者

```lua
snippet_engine = nil  -- 使用 Neogen 内置跳转
```

配合：
```lua
vim.keymap.set("i", "<Tab>", function()
    if require('neogen').jumpable() then
        require('neogen').jump_next()
    else
        return "<Tab>"
    end
end, { expr = true })
```

---

## 🔧 调试技巧

### 检查 Snippet Engine 是否工作

```lua
-- 生成注释后，检查能否跳转
:lua print(require('neogen').jumpable())  -- 应该返回 true
```

### 常见问题

#### 问题 1: 按 Tab 没反应

**原因**：可能 Tab 被其他插件占用（如 nvim-cmp）

**解决**：配置优先级

```lua
["<Tab>"] = cmp.mapping(function(fallback)
    if require('neogen').jumpable() then
        require('neogen').jump_next()
    elseif cmp.visible() then
        cmp.select_next_item()
    else
        fallback()
    end
end)
```

#### 问题 2: LuaSnip 未加载

**检查**：
```vim
:lua print(vim.inspect(package.loaded['luasnip']))
```

**解决**：确保 LuaSnip 先加载

```lua
{
    "danymat/neogen",
    dependencies = { "L3MON4D3/LuaSnip" },  -- 声明依赖
}
```

---

## 📝 总结

### Snippet Engine 是什么？

**一句话**：让你用 `Tab` 键在占位符之间快速跳转的工具。

### 在 Neogen 中的作用？

生成注释后，帮你：
1. 🎯 定位到需要填写的位置
2. ⌨️ 按 Tab 快速跳转
3. 🎨 高亮当前位置
4. ⚡ 提高填写效率

### 我应该用哪个？

| 你的情况 | 推荐 |
|---------|------|
| 不想装太多插件 | `nvim` (原生) |
| 需要强大功能 | `luasnip` |
| 从 VSCode 来 | `vsnip` |
| 用 mini.nvim | `mini` |
| 极简主义 | `nil` (内置跳转) |

### 快速开始

```lua
-- 最简单的配置
{
    "danymat/neogen",
    config = function()
        require('neogen').setup({
            snippet_engine = "nvim",  -- Neovim 0.10+ 自带
        })
        vim.keymap.set("n", "<leader>nf", ":Neogen func<CR>")
    end,
}
```

就这么简单！🎉
