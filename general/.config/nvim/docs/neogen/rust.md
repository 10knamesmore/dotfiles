# Rust 文档注释可用的 Template Items 完整指南

<!--toc:start-->
- [Rust 文档注释可用的 Template Items 完整指南](#rust-文档注释可用的-template-items-完整指南)
  - [所有可用的 Template Items](#所有可用的-template-items)
  - [Rust 当前支持的 Items](#rust-当前支持的-items)
    - [默认配置支持](#默认配置支持)
      - [对于函数 (`func`)](#对于函数-func)
      - [对于结构体/特征 (`class`)](#对于结构体特征-class)
      - [对于文件 (`file`)](#对于文件-file)
    - [❌ Rust 当前**不支持**的（但可以手动添加）](#rust-当前不支持的但可以手动添加)
  - [为什么 Rust 配置比较简单？](#为什么-rust-配置比较简单)
  - [如何扩展 Rust 配置](#如何扩展-rust-配置)
    - [方法 1: 只使用占位符（推荐，简单）](#方法-1-只使用占位符推荐简单)
    - [方法 2: 扩展 Rust 配置以提取更多信息（高级）](#方法-2-扩展-rust-配置以提取更多信息高级)
      - [示例：提取参数类型和返回类型](#示例提取参数类型和返回类型)
  - [实用示例](#实用示例)
    - [示例 1: 简单但功能完整的模板（推荐）](#示例-1-简单但功能完整的模板推荐)
    - [示例 2: 使用条件 items（Has* 系列）](#示例-2-使用条件-itemshas-系列)
    - [示例 3: 为不同 Rust 项目使用不同风格](#示例-3-为不同-rust-项目使用不同风格)
  - [总结](#总结)
    - [Rust 默认支持的 Items](#rust-默认支持的-items)
    - [推荐做法](#推荐做法)
    - [快速参考](#快速参考)
<!--toc:end-->


---

## 所有可用的 Template Items

来自 `neogen.types.template.item` 的完整列表：

```lua
local i = require("neogen.types.template").item

-- 所有可用的 items：
i.Tparam          = "tparam"              -- 类型参数（泛型）
i.Parameter       = "parameters"          -- 函数参数
i.Return          = "return_statement"    -- 返回语句
i.ReturnTypeHint  = "return_type_hint"    -- 返回类型提示
i.ReturnAnonym    = "return_anonym"       -- 匿名返回
i.ClassName       = "class_name"          -- 类名
i.Throw           = "throw_statement"     -- 异常/错误语句
i.Yield           = "expression_statement" -- Yield 语句（生成器）
i.Vararg          = "varargs"             -- 可变参数
i.Type            = "type"                -- 类型
i.ClassAttribute  = "attributes"          -- 类属性
i.HasParameter    = "has_parameters"      -- 是否有参数（布尔标记）
i.HasReturn       = "has_return"          -- 是否有返回值（布尔标记）
i.HasThrow        = "has_throw"           -- 是否抛出异常（布尔标记）
i.HasYield        = "has_yield"           -- 是否有 yield（布尔标记）
i.ArbitraryArgs   = "arbitrary_args"      -- 任意参数（如 Python 的 *args）
i.Kwargs          = "kwargs"              -- 关键字参数（如 Python 的 **kwargs）
```

---

## Rust 当前支持的 Items

### 默认配置支持

根据 `lua/neogen/configurations/rust.lua`，Rust **当前只提取**：

#### 对于函数 (`func`)
- ✅ **`i.Parameter`** - 函数参数名称

#### 对于结构体/特征 (`class`)
- ✅ **`i.Parameter`** - 字段名称（实际上是复用了 Parameter）

#### 对于文件 (`file`)
- ❌ 无特殊提取

### ❌ Rust 当前**不支持**的（但可以手动添加）

以下是 Rust 配置中**未实现**但理论上可以添加的：

- ❌ `i.Type` - 参数类型
- ❌ `i.ReturnTypeHint` - 返回类型
- ❌ `i.Tparam` - 泛型参数
- ❌ `i.Throw` - 错误类型（Result/panic）
- ❌ `i.ClassName` - 结构体/特征名称
- ❌ `i.ClassAttribute` - 结构体属性（带类型）

---

## 为什么 Rust 配置比较简单？

Rust 的默认配置只提取了**参数名称**，没有提取类型、返回值类型、泛型等信息。原因可能是：

1. **Tree-sitter 限制**：需要额外的查询逻辑来提取类型信息
2. **Rust 的类型推断**：很多情况下类型是可选的
3. **简化设计**：Neogen 默认配置倾向于简洁

**但是**，你可以自己扩展配置来支持更多特性！

---

## 如何扩展 Rust 配置

### 方法 1: 只使用占位符（推荐，简单）

即使 Rust 配置不提取类型，你仍然可以在模板中添加占位符让用户手动填写：

```lua
local my_rust_template = {
    { nil, "/ $1", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Type Parameters:", { type = { "func" } } },
    { nil, "/ - `T`: $1", { type = { "func" } } },  -- 手动添加泛型说明
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Params:", { type = { "func" } } },
    { i.Parameter, "/ - `%s`: $1", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Return Type:", { type = { "func" } } },
    { nil, "/ `$1`", { type = { "func" } } },  -- 手动填写返回类型
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Errors:", { type = { "func" } } },
    { nil, "/ - `$1`: $1", { type = { "func" } } },  -- 手动填写错误类型
}
```

**优点**：
- ✅ 简单，不需要修改提取逻辑
- ✅ 灵活，用户可以自己决定填什么

**缺点**：
- ❌ 需要手动填写所有内容
- ❌ 无法自动从代码中提取信息

### 方法 2: 扩展 Rust 配置以提取更多信息（高级）

如果你想自动提取类型、泛型等信息，需要修改 Rust 的 `data` 配置。

#### 示例：提取参数类型和返回类型

创建自定义配置文件 `~/.config/nvim/lua/my_neogen_rust.lua`：

```lua
local extractors = require("neogen.utilities.extractors")
local i = require("neogen.types.template").item
local nodes_utils = require("neogen.utilities.nodes")
local template = require("neogen.template")

-- 自定义提取函数
local function extract_function_with_types(node)
    local tree = {
        {
            retrieve = "first",
            node_type = "parameters",
            subtree = {
                -- 提取参数
                {
                    retrieve = "all",
                    node_type = "parameter",
                    extract = true,
                    as = i.Tparam,  -- 使用 Tparam 来存储参数（包括类型）
                },
            },
        },
        -- 尝试提取返回类型
        {
            retrieve = "first",
            node_type = "return_type",  -- Rust 的返回类型节点
            extract = true,
            as = i.ReturnTypeHint,
        },
        -- 提取泛型参数
        {
            retrieve = "first",
            node_type = "type_parameters",
            subtree = {
                {
                    retrieve = "all",
                    node_type = "type_parameter",
                    extract = true,
                    as = i.Type,
                },
            },
        },
    }
    
    local nodes = nodes_utils:matching_nodes_from(node, tree)
    local res = extractors:extract_from_matched(nodes)
    
    -- 进一步处理参数以提取名称和类型
    if res[i.Tparam] then
        local params = {}
        local types = {}
        for _, param_node in ipairs(res[i.Tparam]) do
            local param_tree = {
                { retrieve = "first", node_type = "identifier", extract = true, as = "name" },
                { retrieve = "first", node_type = "type_identifier", extract = true, as = "type" },
            }
            local param_info = nodes_utils:matching_nodes_from(param_node, param_tree)
            param_info = extractors:extract_from_matched(param_info)
            
            if param_info.name then
                table.insert(params, param_info.name[1])
                if param_info.type then
                    table.insert(types, param_info.type[1])
                else
                    table.insert(types, "unknown")
                end
            end
        end
        res[i.Parameter] = params
        res[i.Type] = types
    end
    
    return res
end

-- 自定义模板（可以使用提取的类型）
local advanced_rust_template = {
    { nil, "/ $1", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    
    -- 泛型参数
    { i.Type, "/ # Type Parameters:", { before_first_item = { "" }, type = { "func" } } },
    { i.Type, "/ - `%s`: $1", { type = { "func" } } },
    
    -- 参数（带类型）
    { nil, "/ # Params:", { type = { "func" } } },
    { i.Parameter, "/ - `%s`: $1", { type = { "func" } } },
    
    -- 返回类型
    { i.ReturnTypeHint, "/ ", { before_first_item = { "", "/ # Return" }, type = { "func" } } },
    { i.ReturnTypeHint, "/ `%s` - $1", { type = { "func" } } },
    
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Examples:", { type = { "func" } } },
    { nil, "/ ```rust", { type = { "func" } } },
    { nil, "/ $1", { type = { "func" } } },
    { nil, "/ ```", { type = { "func" } } },
}

return {
    parent = {
        func = { "function_item", "function_signature_item" },
        class = { "struct_item", "trait_item", "impl_item" },
        file = { "source_file" },
    },
    data = {
        func = {
            ["function_item|function_signature_item"] = {
                ["0"] = {
                    extract = extract_function_with_types,
                },
            },
        },
        class = {
            ["struct_item|trait_item|impl_item"] = {
                ["0"] = {
                    extract = function(node)
                        local tree = {
                            {
                                retrieve = "first",
                                node_type = "field_declaration_list",
                                subtree = {
                                    {
                                        retrieve = "all",
                                        node_type = "field_declaration",
                                        extract = true,
                                        as = i.ClassAttribute,
                                    },
                                },
                            },
                        }
                        local nodes = nodes_utils:matching_nodes_from(node, tree)
                        local res = extractors:extract_from_matched(nodes)
                        
                        -- 提取字段名称和类型
                        if res[i.ClassAttribute] then
                            local fields = {}
                            local types = {}
                            for _, field in ipairs(res[i.ClassAttribute]) do
                                local field_tree = {
                                    { retrieve = "first", node_type = "field_identifier", extract = true, as = "name" },
                                    { retrieve = "first", node_type = "type_identifier", extract = true, as = "type" },
                                }
                                local field_info = nodes_utils:matching_nodes_from(field, field_tree)
                                field_info = extractors:extract_from_matched(field_info)
                                
                                if field_info.name then
                                    table.insert(fields, field_info.name[1])
                                    if field_info.type then
                                        table.insert(types, field_info.type[1])
                                    end
                                end
                            end
                            res[i.Parameter] = fields
                            res[i.Type] = types
                        end
                        
                        return res
                    end,
                },
            },
        },
        file = {
            ["source_file"] = {
                ["0"] = {
                    extract = function()
                        return {}
                    end,
                },
            },
        },
    },
    template = template
        :config({ use_default_comment = true })
        :add_custom_annotation("advanced", advanced_rust_template, true),
}
```

**在 Neogen 配置中使用**：

```lua
require('neogen').setup({
    languages = {
        rust = require('my_neogen_rust'),  -- 加载自定义配置
    }
})
```

**注意**：这个方法需要对 Tree-sitter 和 Neogen 的内部机制有深入了解，可能需要调试。

---

## 实用示例

### 示例 1: 简单但功能完整的模板（推荐）

```lua
local i = require("neogen.types.template").item

local practical_rust_template = {
    -- 函数：有参数
    { nil, "/ $1", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Arguments", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { i.Parameter, "/ * `%s` - $1", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Returns", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ $1", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ # Examples", { type = { "func" } } },
    { nil, "/ ", { type = { "func" } } },
    { nil, "/ ```", { type = { "func" } } },
    { nil, "/ $1", { type = { "func" } } },
    { nil, "/ ```", { type = { "func" } } },
    
    -- 函数：无参数
    { nil, "/ $1", { no_results = true, type = { "func" } } },
    { nil, "/ ", { no_results = true, type = { "func" } } },
    { nil, "/ # Examples", { no_results = true, type = { "func" } } },
    { nil, "/ ", { no_results = true, type = { "func" } } },
    { nil, "/ ```", { no_results = true, type = { "func" } } },
    { nil, "/ $1", { no_results = true, type = { "func" } } },
    { nil, "/ ```", { no_results = true, type = { "func" } } },
    
    -- 结构体/特征：有字段
    { nil, "/ $1", { type = { "class" } } },
    { nil, "/ ", { type = { "class" } } },
    { nil, "/ # Fields", { type = { "class" } } },
    { nil, "/ ", { type = { "class" } } },
    { i.Parameter, "/ * `%s` - $1", { type = { "class" } } },
    
    -- 结构体/特征：无字段
    { nil, "/ $1", { no_results = true, type = { "class" } } },
}

require('neogen').setup({
    languages = {
        rust = {
            template = {
                annotation_convention = "practical",
                practical = practical_rust_template,
            }
        }
    }
})
```

### 示例 2: 使用条件 items（Has* 系列）

虽然 Rust 配置默认不提取 `HasParameter` 等，但你可以在自定义提取函数中添加：

```lua
-- 在提取函数中添加
if res[i.Parameter] and #res[i.Parameter] > 0 then
    res[i.HasParameter] = { true }
end

-- 在模板中使用
local conditional_template = {
    { nil, "/ $1", { type = { "func" } } },
    { i.HasParameter, "/ ", { type = { "func" } } },
    { i.HasParameter, "/ # Arguments", { type = { "func" } } },
    { i.Parameter, "/ * `%s` - $1", { type = { "func" } } },
}
```

### 示例 3: 为不同 Rust 项目使用不同风格

```lua
-- 在项目 A 使用简洁风格
vim.keymap.set("n", "<leader>nfs", function()
    require('neogen').generate({
        annotation_convention = { rust = "rustdoc" }  -- 简洁
    })
end)

-- 在项目 B 使用详细风格
vim.keymap.set("n", "<leader>nfd", function()
    require('neogen').generate({
        annotation_convention = { rust = "my_detailed_template" }
    })
end)
```

---

## 总结

### Rust 默认支持的 Items

| Item | 支持 | 用途 |
|------|------|------|
| `i.Parameter` | ✅ | 函数参数名、结构体字段名 |
| `i.Type` | ❌ | 需要自己实现 |
| `i.ReturnTypeHint` | ❌ | 需要自己实现 |
| `i.Tparam` | ❌ | 需要自己实现（泛型） |
| `i.ClassName` | ❌ | 需要自己实现 |
| `i.Throw` | ❌ | Rust 中可用于 Result 类型 |

### 推荐做法

1. **简单项目**：使用占位符 `$1`，手动填写所有内容
2. **中等需求**：提取参数名（已支持），类型用占位符
3. **高级需求**：扩展 Rust 配置，实现类型、泛型等提取

### 快速参考

```lua
local i = require("neogen.types.template").item

-- 在模板中使用
{ i.Parameter, "/ - `%s`: $1", { type = { "func" } } }
-- %s 会被替换为参数名
-- $1 是光标跳转占位符

-- 条件显示（仅在有参数时显示）
{ nil, "/ # Params:", { type = { "func" } } }
-- 只有当提取到参数时才会显示

-- 无参数时显示
{ nil, "/ 简单函数", { no_results = true, type = { "func" } } }
```

希望这个指南能帮助你更好地理解和定制 Rust 的文档注释！🦀
