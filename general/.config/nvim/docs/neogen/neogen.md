# Neogen 完整使用文档

<!--toc:start-->
- [Neogen 完整使用文档](#neogen-完整使用文档)
  - [插件简介](#插件简介)
    - [主要用途](#主要用途)
  - [安装配置](#安装配置)
    - [使用 lazy.nvim 安装](#使用-lazynvim-安装)
      - [最简配置](#最简配置)
      - [基础配置（推荐）](#基础配置推荐)
      - [高级配置示例](#高级配置示例)
    - [与代码片段引擎集成](#与代码片段引擎集成)
      - [LuaSnip 集成](#luasnip-集成)
      - [原生跳转（不使用片段引擎）](#原生跳转不使用片段引擎)
  - [配置项详解](#配置项详解)
    - [完整配置项说明](#完整配置项说明)
    - [配置项详细说明](#配置项详细说明)
    - [模板配置选项](#模板配置选项)
  - [命令与 API](#命令与-api)
    - [Vim 命令](#vim-命令)
      - [`:Neogen [type]`](#neogen-type)
    - [Lua API](#lua-api)
      - [`require('neogen').generate([opts])`](#requireneogengenerateopts)
      - [`require('neogen').jump_next()`](#requireneogenjumpnext)
      - [`require('neogen').jump_prev()`](#requireneogenjumpprev)
      - [`require('neogen').jumpable([reverse])`](#requireneogenjumpablereverse)
  - [支持的语言和注释风格](#支持的语言和注释风格)
    - [完整语言列表](#完整语言列表)
    - [各语言默认风格](#各语言默认风格)
  - [代码片段引擎集成](#代码片段引擎集成)
    - [支持的引擎](#支持的引擎)
    - [配置示例](#配置示例)
    - [不使用片段引擎](#不使用片段引擎)
  - [高级配置示例](#高级配置示例)
    - [示例 1：多语言项目配置](#示例-1多语言项目配置)
    - [示例 2：自定义占位符文本（中文）](#示例-2自定义占位符文本中文)
    - [示例 3：Python 不同注释风格对比](#示例-3python-不同注释风格对比)
    - [示例 4：为 C++ 添加自定义文件类型](#示例-4为-c-添加自定义文件类型)
    - [示例 5：完整的 Which-key 集成](#示例-5完整的-which-key-集成)
  - [自定义 Rust 注释格式](#自定义-rust-注释格式)
    - [问题：如何修改 Rust 文件的 doc comment 格式？](#问题如何修改-rust-文件的-doc-comment-格式)
    - [方法 1：修改默认风格](#方法-1修改默认风格)
    - [方法 2：创建完全自定义的 Rust 注释模板](#方法-2创建完全自定义的-rust-注释模板)
    - [两种 Rust 风格对比](#两种-rust-风格对比)
      - [rustdoc（默认，简洁）](#rustdoc默认简洁)
      - [rust_alternative（详细）](#rustalternative详细)
      - [我的风格](#我的风格)
    - [实际使用示例](#实际使用示例)
    - [在配置中切换风格](#在配置中切换风格)
  - [典型使用场景](#典型使用场景)
    - [场景 1：为 Python 函数生成文档](#场景-1为-python-函数生成文档)
    - [场景 2：为 TypeScript 类生成文档](#场景-2为-typescript-类生成文档)
    - [场景 3：为 C++ 函数生成 Doxygen 注释](#场景-3为-c-函数生成-doxygen-注释)
    - [场景 4：为文件添加模块说明](#场景-4为文件添加模块说明)
    - [场景 5：与 LSP 配合使用](#场景-5与-lsp-配合使用)
  - [常见问题与注意事项](#常见问题与注意事项)
    - [常见问题](#常见问题)
      - [Q1: 为什么生成的注释位置不对？](#q1-为什么生成的注释位置不对)
      - [Q2: 如何为不支持的语言添加支持？](#q2-如何为不支持的语言添加支持)
      - [Q3: 占位符跳转不工作？](#q3-占位符跳转不工作)
      - [Q4: 如何禁用某些类型的注释生成？](#q4-如何禁用某些类型的注释生成)
      - [Q5: 生成的注释与 `commentstring` 不符？](#q5-生成的注释与-commentstring-不符)
    - [注意事项](#注意事项)
    - [调试技巧](#调试技巧)
    - [推荐工作流](#推荐工作流)
  - [参考资源](#参考资源)
<!--toc:end-->


---

## 插件简介

**Neogen** (Neovim Generator) 是一个强大的代码注释/文档生成插件，专为 Neovim 设计。它的主要特点包括：

- **自动生成标准注释**：一键为函数、类、文件等生成符合规范的文档注释
- **多语言支持**：支持 18+ 种编程语言
- **多种注释风格**：每种语言支持多种注释约定（如 Python 的 Google、NumPy、reST 风格）
- **基于 Tree-sitter**：利用语法树精确解析代码结构
- **高度可定制**：可自定义注释模板、占位符、位置等
- **代码片段引擎集成**：支持 LuaSnip、snippy、vsnip、nvim 原生片段、mini.snippets
- **智能光标跳转**：在生成的注释占位符之间快速跳转

### 主要用途

- 快速生成函数/方法的参数、返回值文档
- 为类添加属性说明
- 生成文件头部的模块说明
- 为类型定义添加注释
- 提高代码文档的一致性和完整性

---

## 安装配置

### 使用 lazy.nvim 安装

#### 最简配置

```lua
{
    "danymat/neogen",
    config = true,
    -- 如果只想使用稳定版本
    -- version = "*"
}
```

#### 基础配置（推荐）

```lua
{
    "danymat/neogen",
    dependencies = "nvim-treesitter/nvim-treesitter",
    config = function()
        require('neogen').setup({
            enabled = true,
            snippet_engine = "luasnip"  -- 或 "snippy", "vsnip", "nvim", "mini"
        })

        -- 设置快捷键
        local opts = { noremap = true, silent = true }
        vim.api.nvim_set_keymap("n", "<Leader>nf", ":lua require('neogen').generate()<CR>", opts)
        vim.api.nvim_set_keymap("n", "<Leader>nc", ":lua require('neogen').generate({ type = 'class' })<CR>", opts)
        vim.api.nvim_set_keymap("n", "<Leader>nt", ":lua require('neogen').generate({ type = 'type' })<CR>", opts)
    end,
    version = "*"
}
```

#### 高级配置示例

```lua
{
    "danymat/neogen",
    dependencies = "nvim-treesitter/nvim-treesitter",
    config = function()
        require('neogen').setup({
            enabled = true,
            input_after_comment = true,     -- 自动跳转到插入模式
            snippet_engine = "luasnip",
            enable_placeholders = true,     -- 启用占位符

            -- 自定义占位符文本
            placeholders_text = {
                ["description"] = "[描述]",
                ["tparam"] = "[TODO:参数类型]",
                ["parameter"] = "[TODO:参数]",
                ["return"] = "[TODO:返回值]",
                ["class"] = "[TODO:类]",
                ["throw"] = "[TODO:异常]",
                ["varargs"] = "[TODO:可变参数]",
                ["type"] = "[TODO:类型]",
                ["attribute"] = "[TODO:属性]",
                ["args"] = "[TODO:参数]",
                ["kwargs"] = "[TODO:关键字参数]",
            },

            -- 占位符高亮（可设为 "None" 禁用）
            placeholders_hl = "DiagnosticHint",

            -- 语言特定配置
            languages = {
                lua = {
                    template = {
                        annotation_convention = "emmylua"
                    }
                },
                python = {
                    template = {
                        annotation_convention = "google_docstrings"
                    }
                },
                rust = {
                    template = {
                        annotation_convention = "rustdoc"
                    }
                },
            }
        })

        -- 快捷键配置
        local opts = { noremap = true, silent = true }
        vim.keymap.set("n", "<Leader>ng", ":Neogen<CR>", opts)
        vim.keymap.set("n", "<Leader>nf", ":Neogen func<CR>", opts)
        vim.keymap.set("n", "<Leader>nc", ":Neogen class<CR>", opts)
        vim.keymap.set("n", "<Leader>nt", ":Neogen type<CR>", opts)
        vim.keymap.set("n", "<Leader>nF", ":Neogen file<CR>", opts)
    end,
}
```

### 与代码片段引擎集成

#### LuaSnip 集成

```lua
-- 在 neogen 配置中
snippet_engine = "luasnip"

-- 配置跳转快捷键（与 nvim-cmp 集成）
local cmp = require('cmp')
local neogen = require('neogen')

cmp.setup {
    mapping = {
        ["<Tab>"] = cmp.mapping(function(fallback)
            if neogen.jumpable() then
                neogen.jump_next()
            else
                fallback()
            end
        end, { "i", "s" }),

        ["<S-Tab>"] = cmp.mapping(function(fallback)
            if neogen.jumpable(true) then
                neogen.jump_prev()
            else
                fallback()
            end
        end, { "i", "s" }),
    },
}
```

#### 原生跳转（不使用片段引擎）

```lua
local opts = { noremap = true, silent = true }
vim.api.nvim_set_keymap("i", "<C-l>", ":lua require('neogen').jump_next()<CR>", opts)
vim.api.nvim_set_keymap("i", "<C-h>", ":lua require('neogen').jump_prev()<CR>", opts)
```

---

## 配置项详解

### 完整配置项说明

```lua
require('neogen').setup({
    -- 是否启用 Neogen
    enabled = true,                    -- 默认: true

    -- 生成注释后自动跳转到第一个占位符并进入插入模式
    input_after_comment = true,        -- 默认: true

    -- 代码片段引擎（可选值: "luasnip", "snippy", "vsnip", "nvim", "mini", nil）
    snippet_engine = nil,              -- 默认: nil (使用原生跳转)

    -- 是否启用占位符
    enable_placeholders = true,        -- 默认: true

    -- 占位符文本自定义
    placeholders_text = {
        ["description"] = "[TODO:description]",
        ["tparam"] = "[TODO:tparam]",
        ["parameter"] = "[TODO:parameter]",
        ["return"] = "[TODO:return]",
        ["class"] = "[TODO:class]",
        ["throw"] = "[TODO:throw]",
        ["varargs"] = "[TODO:varargs]",
        ["type"] = "[TODO:type]",
        ["attribute"] = "[TODO:attribute]",
        ["args"] = "[TODO:args]",
        ["kwargs"] = "[TODO:kwargs]",
    },

    -- 占位符高亮组（设为 "None" 禁用高亮）
    placeholders_hl = "DiagnosticHint",

    -- 语言特定配置
    languages = {
        -- 示例：配置 Rust
        rust = {
            template = {
                annotation_convention = "rustdoc",  -- 默认注释风格
                use_default_comment = true,         -- 使用语言默认的注释符号

                -- 可选：自定义注释位置
                -- append = {
                --     position = "after",      -- "before" 或 "after"
                --     child_name = "comment",  -- 相对于哪个节点
                --     fallback = "block",      -- 找不到 child_name 时的回退节点
                --     disabled = { "file" }    -- 对哪些类型禁用此设置
                -- },

                -- 可选：完全自定义位置
                -- position = function(node, type)
                --     -- 返回 row, col
                -- end
            }
        },

        -- 为新文件类型添加支持（基于现有配置）
        ['cpp.doxygen'] = require('neogen.configurations.cpp'),

        -- CUDA 使用 C++ 配置
        cuda = require('neogen.configurations.cpp'),
    }
})
```

### 配置项详细说明

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `enabled` | boolean | `true` | 启用/禁用 Neogen |
| `input_after_comment` | boolean | `true` | 生成注释后自动进入插入模式 |
| `snippet_engine` | string/nil | `nil` | 片段引擎名称 |
| `enable_placeholders` | boolean | `true` | 启用占位符功能 |
| `placeholders_text` | table | 见上 | 自定义占位符文本 |
| `placeholders_hl` | string | `"DiagnosticHint"` | 占位符高亮组 |
| `languages` | table | `{}` | 语言特定配置 |

### 模板配置选项

```lua
template = {
    -- 默认使用的注释约定
    annotation_convention = "rustdoc",

    -- 是否在注释前添加语言的默认注释符
    use_default_comment = true,

    -- 自定义注释放置位置
    append = {
        position = "after",      -- "before" 或 "after"
        child_name = "comment",  -- 相对节点名称
        fallback = "block",      -- 回退节点
        disabled = { "file" }    -- 禁用的类型列表
    },

    -- 自定义位置函数（高级用法）
    position = function(node, type)
        if type == "file" then
            return 0, 0  -- 文件顶部
        end
        -- 返回 nil 使用默认位置
    end
}
```

---

## 命令与 API

### Vim 命令

#### `:Neogen [type]`

生成指定类型的注释。不指定类型时自动检测最近的父节点类型。

```vim
" 自动检测类型（智能生成）
:Neogen

" 生成函数注释
:Neogen func

" 生成类注释
:Neogen class

" 生成类型注释
:Neogen type

" 生成文件注释
:Neogen file
```

**工作原理**：
- 从当前光标位置向上查找语法树
- 找到第一个匹配指定类型的父节点
- 在该节点前/后生成注释

**示例**：

```csharp
public class HelloWorld
{
    public static void Main(string[] args)
    {
        // 光标在这里，执行 :Neogen class
        Console.WriteLine("Hello world!");
    }
}
```

会在 `HelloWorld` 类前生成注释：

```csharp
/// <summary>
/// [TODO:description]
/// </summary>
public class HelloWorld
{
    ...
}
```

### Lua API

#### `require('neogen').generate([opts])`

主要的生成函数。

**参数**：

```lua
opts = {
    type = "func",  -- 可选值: "func", "class", "type", "file", "any"
                    -- 默认: "any" (自动检测)

    annotation_convention = {
        python = "numpydoc",  -- 为特定语言指定注释风格
        rust = "rust_alternative",
    },

    return_snippet = false,  -- 返回片段而不是直接插入
}
```

**返回值**（当 `return_snippet = true`）：

```lua
local snippet, row, col = require('neogen').generate({
    type = "func",
    return_snippet = true
})

-- snippet: string[] - LSP 格式的片段
-- row: number - 插入位置行号
-- col: number - 插入位置列号
```

**示例**：

```lua
-- 基础用法
require('neogen').generate()

-- 指定类型
require('neogen').generate({ type = "func" })

-- 指定注释风格
require('neogen').generate({
    type = "class",
    annotation_convention = {
        python = "google_docstrings"
    }
})

-- 获取片段用于其他引擎
local snippet, row = require('neogen').generate({
    type = "func",
    return_snippet = true
})
-- 然后将 snippet 传给你的片段引擎
```

#### `require('neogen').jump_next()`

跳转到下一个占位符（插入模式）。

```lua
vim.keymap.set("i", "<C-l>", function()
    require('neogen').jump_next()
end)
```

#### `require('neogen').jump_prev()`

跳转到上一个占位符（插入模式）。

```lua
vim.keymap.set("i", "<C-h>", function()
    require('neogen').jump_prev()
end)
```

#### `require('neogen').jumpable([reverse])`

检查是否可以跳转。

```lua
local neogen = require('neogen')

-- 检查是否可以向前跳转
if neogen.jumpable() then
    neogen.jump_next()
end

-- 检查是否可以向后跳转
if neogen.jumpable(true) then
    neogen.jump_prev()
end
```

---

## 支持的语言和注释风格

### 完整语言列表

| 语言 | 注释风格 | 支持的类型 |
|------|----------|-----------|
| **Bash** | Google Style Guide (`google_bash`) | `func`, `file` |
| **C** | Doxygen (`doxygen`) | `func`, `file`, `type` |
| **C#** | Xmldoc (`xmldoc`), Doxygen (`doxygen`) | `func`, `file`, `class` |
| **C++** | Doxygen (`doxygen`) | `func`, `file`, `class` |
| **Go** | GoDoc (`godoc`) | `func`, `type` |
| **Java** | Javadoc (`javadoc`) | `func`, `class`, `type` |
| **JavaScript** | JSDoc (`jsdoc`) | `func`, `class`, `type`, `file` |
| **JavaScriptReact** | JSDoc (`jsdoc`) | `func`, `class`, `type`, `file` |
| **Julia** | Julia (`julia`) | `func`, `class` |
| **Kotlin** | KDoc (`kdoc`) | `func`, `class` |
| **Lua** | Emmylua (`emmylua`), LDoc (`ldoc`) | `func`, `class`, `type`, `file` |
| **PHP** | PHPDoc (`phpdoc`) | `func`, `type`, `class` |
| **Python** | Google (`google_docstrings`), NumPy (`numpydoc`), reST (`reST`) | `func`, `class`, `type`, `file` |
| **Ruby** | YARD (`yard`), Rdoc (`rdoc`), Tomdoc (`tomdoc`) | `func`, `type`, `class` |
| **Rust** | RustDoc (`rustdoc`), Alternative (`rust_alternative`) | `func`, `file`, `class` |
| **TypeScript** | JSDoc (`jsdoc`), TSDoc (`tsdoc`) | `func`, `class`, `type`, `file` |
| **TypeScriptReact** | JSDoc (`jsdoc`), TSDoc (`tsdoc`) | `func`, `class`, `type`, `file` |
| **Vue** | JSDoc (`jsdoc`) | `func`, `class`, `type`, `file` |

### 各语言默认风格

```lua
-- 查看源码中的默认配置
-- lua/neogen/configurations/<language>.lua

-- 示例：
Lua        -> emmylua
Python     -> google_docstrings
Rust       -> rust_alternative
TypeScript -> jsdoc
C/C++      -> doxygen
```

---

## 代码片段引擎集成

### 支持的引擎

1. **LuaSnip** (`luasnip`)
2. **Snippy** (`snippy`)
3. **Vsnip** (`vsnip`)
4. **Neovim 原生** (`nvim`) - Neovim 0.10+
5. **mini.snippets** (`mini`)

### 配置示例

```lua
-- 1. LuaSnip
require('neogen').setup({
    snippet_engine = "luasnip"
})

-- 2. Snippy
require('neogen').setup({
    snippet_engine = "snippy"
})

-- 3. Vsnip
require('neogen').setup({
    snippet_engine = "vsnip"
})

-- 4. Neovim 原生片段
require('neogen').setup({
    snippet_engine = "nvim"
})

-- 5. mini.snippets
require('neogen').setup({
    snippet_engine = "mini"
})
```

### 不使用片段引擎

如果不配置 `snippet_engine`，Neogen 会使用内置的跳转系统：

```lua
require('neogen').setup({
    snippet_engine = nil,  -- 或不设置此项
})

-- 配置跳转快捷键
local opts = { noremap = true, silent = true }
vim.api.nvim_set_keymap("i", "<C-l>", ":lua require('neogen').jump_next()<CR>", opts)
vim.api.nvim_set_keymap("i", "<C-h>", ":lua require('neogen').jump_prev()<CR>", opts)
```

---

## 高级配置示例

### 示例 1：多语言项目配置

```lua
{
    "danymat/neogen",
    config = function()
        require('neogen').setup({
            snippet_engine = "luasnip",
            languages = {
                python = {
                    template = {
                        annotation_convention = "google_docstrings"
                    }
                },
                typescript = {
                    template = {
                        annotation_convention = "tsdoc"
                    }
                },
                rust = {
                    template = {
                        annotation_convention = "rustdoc"
                    }
                },
                lua = {
                    template = {
                        annotation_convention = "ldoc"
                    }
                },
            }
        })
    end
}
```

### 示例 2：自定义占位符文本（中文）

```lua
require('neogen').setup({
    placeholders_text = {
        ["description"] = "[描述]",
        ["tparam"] = "[类型参数]",
        ["parameter"] = "[参数说明]",
        ["return"] = "[返回值]",
        ["class"] = "[类说明]",
        ["throw"] = "[异常]",
        ["varargs"] = "[可变参数]",
        ["type"] = "[类型]",
        ["attribute"] = "[属性]",
        ["args"] = "[参数]",
        ["kwargs"] = "[关键字参数]",
    },
})
```

### 示例 3：Python 不同注释风格对比

```python
# Google Style (google_docstrings)
def example_function(param1, param2):
    """[TODO:description]

    Args:
        param1 ([TODO:parameter]): [TODO:description]
        param2 ([TODO:parameter]): [TODO:description]

    Returns:
        [TODO:return]
    """
    pass

# NumPy Style (numpydoc)
def example_function(param1, param2):
    """[TODO:description]

    Parameters
    ----------
    param1 : [TODO:parameter]
        [TODO:description]
    param2 : [TODO:parameter]
        [TODO:description]

    Returns
    -------
    [TODO:return]
    """
    pass

# reST Style (reST)
def example_function(param1, param2):
    """[TODO:description]

    :param param1: [TODO:description]
    :type param1: [TODO:parameter]
    :param param2: [TODO:description]
    :type param2: [TODO:parameter]
    :return: [TODO:return]
    :rtype: [TODO:return]
    """
    pass
```

### 示例 4：为 C++ 添加自定义文件类型

```lua
require('neogen').setup({
    languages = {
        -- CUDA 文件使用 C++ 配置
        cuda = require('neogen.configurations.cpp'),

        -- 带 Doxygen 的 C++
        ['cpp.doxygen'] = require('neogen.configurations.cpp'),
    }
})
```

### 示例 5：完整的 Which-key 集成

```lua
{
    "danymat/neogen",
    dependencies = {
        "nvim-treesitter/nvim-treesitter",
        "folke/which-key.nvim",
    },
    config = function()
        require('neogen').setup({
            snippet_engine = "luasnip",
        })

        local wk = require("which-key")
        wk.register({
            ["<leader>n"] = {
                name = "Neogen",
                g = { ":Neogen<CR>", "Generate (auto)" },
                f = { ":Neogen func<CR>", "Function" },
                c = { ":Neogen class<CR>", "Class" },
                t = { ":Neogen type<CR>", "Type" },
                F = { ":Neogen file<CR>", "File" },
            }
        })
    end
}
```

---

## 自定义 Rust 注释格式

### 问题：如何修改 Rust 文件的 doc comment 格式？

Rust 支持两种注释风格：

1. **rustdoc**（标准风格，简洁）
2. **rust_alternative**（详细风格，包含参数列表）

### 方法 1：修改默认风格

在你的 `~/.config/nvim/init.lua` 或 `~/.config/nvim/lua/plugins/neogen.lua` 中：

```lua
{
    "danymat/neogen",
    config = function()
        require('neogen').setup({
            languages = {
                rust = {
                    template = {
                        annotation_convention = "rust_alternative"  -- 改为详细风格
                    }
                }
            }
        })
    end
}
```

### 方法 2：创建完全自定义的 Rust 注释模板

```lua
{
    "danymat/neogen",
    config = function()
        local i = require("neogen.types.template").item

        -- 自定义 Rust 注释模板
        local my_rust_template = {
            -- 文件级注释
            { nil, "! # 模块：$1", { no_results = true, type = { "file" } } },
            { nil, "! ", { no_results = true, type = { "file" } } },
            { nil, "! ## 说明", { no_results = true, type = { "file" } } },
            { nil, "! $1", { no_results = true, type = { "file" } } },

            -- 函数/类无参数时
            { nil, "/ # $1", { no_results = true, type = { "func", "class" } } },
            { nil, "/ ", { no_results = true, type = { "func", "class" } } },

            -- 函数/类有参数时
            { nil, "/ # $1", { type = { "func", "class" } } },
            { nil, "/ ", { type = { "func", "class" } } },
            { nil, "/ ## 参数", { type = { "func", "class" } } },
            { i.Parameter, "/ - `%s`: $1", { type = { "func", "class" } } },
            { nil, "/ ", { type = { "func", "class" } } },
            { nil, "/ ## 返回值", { type = { "func" } } },
            { nil, "/ $1", { type = { "func" } } },
        }

        require('neogen').setup({
            languages = {
                rust = {
                    template = {
                        annotation_convention = "my_custom",
                        my_custom = my_rust_template,
                    }
                }
            }
        })
    end
}
```

### 两种 Rust 风格对比

#### rustdoc（默认，简洁）

```rust
/// $1
fn example(param1: i32, param2: String) -> bool {
    true
}
```

#### rust_alternative（详细）

```rust
/// $1
///
/// * `param1`: $1
/// * `param2`: $1
fn example(param1: i32, param2: String) -> bool {
    true
}
```

#### 我的风格
```rust
/// 简要描述函数的作用。
///
/// # Params:
/// - `param1`: 参数1的含义和用途。
/// - `param2`: 参数2的含义和用途。
///
/// # Return
/// 返回值的含义和用途。
///
/// # Error:
/// 说明可能的错误情况（如果返回Result）。
///
/// # Examples:
/// ```rust
/// let result = function_name(arg1, arg2);
/// assert_eq!(result, expected_value);
/// ``\`
///
/// # Notes:
/// 其他需要说明的注意点或特殊行为。
fn example(param1: i32, param2: String) -> bool {
    true
}
```

### 实际使用示例

假设你有这样一个 Rust 函数：

```rust
fn calculate_sum(numbers: Vec<i32>, multiplier: i32) -> i32 {
    numbers.iter().sum::<i32>() * multiplier
}
```

**使用 `rustdoc` 风格**（光标在函数内，执行 `:Neogen func`）：

```rust
/// [TODO:description]
fn calculate_sum(numbers: Vec<i32>, multiplier: i32) -> i32 {
    numbers.iter().sum::<i32>() * multiplier
}
```

**使用 `rust_alternative` 风格**：

```rust
/// [TODO:description]
///
/// * `numbers`: [TODO:parameter]
/// * `multiplier`: [TODO:parameter]
fn calculate_sum(numbers: Vec<i32>, multiplier: i32) -> i32 {
    numbers.iter().sum::<i32>() * multiplier
}
```

### 在配置中切换风格

你可以在运行时动态指定风格：

```lua
-- 使用 rustdoc
require('neogen').generate({
    type = "func",
    annotation_convention = { rust = "rustdoc" }
})

-- 使用 rust_alternative
require('neogen').generate({
    type = "func",
    annotation_convention = { rust = "rust_alternative" }
})
```

**推荐配置**（为不同风格设置不同快捷键）：

```lua
{
    "danymat/neogen",
    config = function()
        require('neogen').setup({
            languages = {
                rust = {
                    template = {
                        annotation_convention = "rust_alternative"  -- 默认详细风格
                    }
                }
            }
        })

        -- 快捷键
        vim.keymap.set("n", "<leader>nd", function()
            -- 简洁风格
            require('neogen').generate({
                annotation_convention = { rust = "rustdoc" }
            })
        end, { desc = "Generate docs (simple)" })

        vim.keymap.set("n", "<leader>nD", function()
            -- 详细风格
            require('neogen').generate({
                annotation_convention = { rust = "rust_alternative" }
            })
        end, { desc = "Generate docs (detailed)" })
    end
}
```

---

## 典型使用场景

### 场景 1：为 Python 函数生成文档

```python
def calculate_statistics(data, method='mean', **kwargs):
    """计算统计数据"""
    pass
```

光标在函数内，执行 `:Neogen func`，生成：

```python
def calculate_statistics(data, method='mean', **kwargs):
    """[TODO:description]

    Args:
        data ([TODO:parameter]): [TODO:description]
        method ([TODO:parameter]): [TODO:description]
        **kwargs: [TODO:kwargs]

    Returns:
        [TODO:return]
    """
    pass
```

### 场景 2：为 TypeScript 类生成文档

```typescript
class UserManager {
    private users: User[];

    addUser(user: User): void {
        this.users.push(user);
    }
}
```

光标在类名处，执行 `:Neogen class`：

```typescript
/**
 *
 * @class
 * @classdesc [TODO:description]
 */
class UserManager {
    // ...
}
```

### 场景 3：为 C++ 函数生成 Doxygen 注释

```cpp
int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}
```

执行 `:Neogen func`：

```cpp
/**
 * @brief [TODO:description]
 *
 * @param n [TODO:parameter]
 * @return int [TODO:return]
 */
int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}
```

### 场景 4：为文件添加模块说明

```python
# my_module.py
import os
import sys

def main():
    pass
```

光标在文件顶部，执行 `:Neogen file`：

```python
"""[TODO:description]

[TODO:description]
"""

import os
import sys

def main():
    pass
```

### 场景 5：与 LSP 配合使用

生成注释后，LSP 会自动识别并提供悬停文档：

```lua
require('neogen').generate()
-- 填写占位符后，将鼠标悬停在函数上，LSP 会显示你刚写的文档
```

---

## 常见问题与注意事项

### 常见问题

#### Q1: 为什么生成的注释位置不对？

**A**: 检查以下几点：
1. 确保已安装对应语言的 Tree-sitter parser
   ```lua
   :TSInstall rust python lua
   ```
2. 检查光标位置是否在目标代码块内
3. 尝试使用 `:Neogen <type>` 明确指定类型

#### Q2: 如何为不支持的语言添加支持？

**A**: 参考文档：
1. 简单方式：复用现有配置
   ```lua
   languages = {
       mylang = require('neogen.configurations.similar_lang')
   }
   ```
2. 完整方式：创建新配置文件
   - 阅读 `docs/adding-languages.md`
   - 参考 `lua/neogen/configurations/` 下的示例

#### Q3: 占位符跳转不工作？

**A**: 检查配置：
- 使用片段引擎时，确保引擎已正确安装和配置
- 不使用片段引擎时，确保设置了跳转快捷键
- 检查 `input_after_comment` 是否为 `true`

#### Q4: 如何禁用某些类型的注释生成？

**A**: 在模板配置中使用 `type` 字段过滤：

```lua
languages = {
    python = {
        template = {
            annotation_convention = "custom",
            custom = {
                -- 只对 func 和 class 生效，忽略 file
                { nil, "...", { type = { "func", "class" } } }
            }
        }
    }
}
```

#### Q5: 生成的注释与 `commentstring` 不符？

**A**: 检查 `use_default_comment` 设置：

```lua
languages = {
    rust = {
        template = {
            use_default_comment = false  -- 不使用 vim.bo.commentstring
        }
    }
}
```

### 注意事项

1. **Tree-sitter 依赖**
   - Neogen 完全依赖 Tree-sitter
   - 必须安装对应语言的 parser
   - 使用 `:checkhealth neogen` 检查状态（如果可用）

2. **占位符功能**
   - 使用片段引擎时占位符自动高亮和跳转
   - 不使用时需手动配置跳转快捷键
   - `enable_placeholders = false` 可禁用占位符

3. **性能考虑**
   - 大文件中可能有轻微延迟
   - Tree-sitter 解析需要时间
   - 考虑使用异步加载插件

4. **自定义模板**
   - 模板修改后立即生效
   - 使用 `local i = require("neogen.types.template").item` 获取所有支持的节点类型
   - 参考现有模板文件学习语法

5. **版本兼容性**
   - 需要 Neovim 0.10.0+
   - 部分片段引擎有版本要求
   - 建议使用 `version = "*"` 跟踪稳定版本

### 调试技巧

```lua
-- 查看当前文件的语法树
:InspectTree

-- 查看光标下的节点
:Inspect

-- 测试 Tree-sitter 查询
:EditQuery

-- 查看 Neogen 配置
:lua print(vim.inspect(require('neogen.config').get()))

-- 查看支持的语言
:lua print(vim.inspect(require('neogen.config').get().languages))
```

### 推荐工作流

1. **初次使用**
   ```
   安装插件 -> 安装 Tree-sitter parsers -> 配置快捷键 -> 测试基本功能
   ```

2. **日常使用**
   ```
   编写代码 -> 光标移到函数/类内 -> <leader>nf 生成注释 -> Tab 跳转填写
   ```

3. **自定义配置**
   ```
   确定需求 -> 查看现有模板 -> 修改或创建模板 -> 测试验证
   ```

---

## 参考资源

- **官方仓库**: https://github.com/danymat/neogen
- **完整文档**: `:help neogen`
- **添加语言**: [docs/adding-languages.md](https://github.com/danymat/neogen/blob/main/docs/adding-languages.md)
- **高级集成**: [docs/advanced-integration.md](https://github.com/danymat/neogen/blob/main/docs/advanced-integration.md)
- **模板示例**: `lua/neogen/templates/*.lua`
- **语言配置**: `lua/neogen/configurations/*.lua`

---
