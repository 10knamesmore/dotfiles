# Mini.align 插件完整使用指南

## 📋 目录

1. [插件简介](#插件简介)
2. [安装配置](#安装配置)
3. [核心概念](#核心概念)
4. [配置选项](#配置选项)
5. [内置修饰符](#内置修饰符)
6. [使用场景与示例](#使用场景与示例)
7. [高级用法](#高级用法)
8. [API 参考](#api-参考)
9. [注意事项](#注意事项)

---

## 插件简介

### 主要功能

`mini.align` 是一个功能强大且高度可定制的文本对齐插件，具有以下特点：

- **交互式对齐**：通过按键修饰符实时调整对齐规则
- **灵活的对齐算法**：分为拆分（Split）、对齐（Justify）、合并（Merge）三个主要步骤
- **多模式支持**：支持字符模式（charwise）、行模式（linewise）、块模式（blockwise）
- **实时预览**：可选择是否实时显示对齐结果
- **高度可定制**：所有步骤和修饰符都可自定义

### 核心特性

1. **三步对齐流程**：
   - **Split**：根据 Lua 模式或自定义规则将行拆分为部分
   - **Justify**：使每列中的部分宽度相同
   - **Merge**：将部分合并回行，使用自定义分隔符

2. **交互式修饰符**：
   - `s` - 输入拆分模式
   - `j` - 选择对齐方向（左/中/右/无）
   - `m` - 输入合并分隔符
   - `f` - 过滤特定列
   - `i` - 忽略注释和字符串
   - `p` - 配对相邻部分
   - `t` - 修剪空白字符
   - `<BS>` - 删除上一个预处理步骤

3. **预置配置**：
   - `=` - 对齐等号及赋值操作符
   - `,` - 对齐逗号
   - `|` - 对齐竖线（表格）
   - `<Space>` - 对齐空格

---

## 安装配置

### 使用 lazy.nvim 安装

#### 最简配置

```lua
{
  'echasnovski/mini.align',
  version = false,
  config = function()
    require('mini.align').setup()
  end,
}
```

#### 推荐配置

```lua
{
  'echasnovski/mini.align',
  version = false,
  event = 'VeryLazy', -- 懒加载
  config = function()
    require('mini.align').setup({
      -- 映射配置
      mappings = {
        start = 'ga',              -- 启动对齐（无预览）
        start_with_preview = 'gA', -- 启动对齐（带预览）
      },
      
      -- 默认选项
      options = {
        split_pattern = '',
        justify_side = 'left',
        merge_delimiter = '',
      },
      
      -- 静默模式（不显示提示信息）
      silent = false,
    })
  end,
}
```

#### 高级自定义配置

```lua
{
  'echasnovski/mini.align',
  version = false,
  keys = {
    { 'ga', mode = { 'n', 'x' } },
    { 'gA', mode = { 'n', 'x' } },
  },
  config = function()
    local align = require('mini.align')
    
    align.setup({
      mappings = {
        start = 'ga',
        start_with_preview = 'gA',
      },
      
      -- 自定义修饰符
      modifiers = {
        -- 循环切换对齐方向（类似 vim-easy-align）
        j = function(_, opts)
          local next_side = {
            left = 'center',
            center = 'right',
            right = 'none',
            none = 'left',
          }
          opts.justify_side = next_side[opts.justify_side] or 'left'
        end,
        
        -- 自定义 T 修饰符：删除所有空白（包括缩进）
        T = function(steps, _)
          table.insert(steps.pre_justify, align.gen_step.trim('both', 'remove'))
        end,
        
        -- 自定义 : 修饰符：专门对齐冒号
        [':'] = function(steps, opts)
          opts.split_pattern = ':'
          opts.justify_side = 'right'
          table.insert(steps.pre_justify, align.gen_step.trim('both'))
          opts.merge_delimiter = ' '
        end,
      },
      
      -- 默认选项
      options = {
        split_pattern = '',
        justify_side = 'left',
        merge_delimiter = '',
      },
      
      -- 自定义步骤
      steps = {
        pre_split = {},
        split = nil,  -- 使用默认
        pre_justify = {},
        justify = nil,  -- 使用默认
        pre_merge = {},
        merge = nil,  -- 使用默认
      },
      
      silent = false,
    })
  end,
}
```

---

## 核心概念

### 术语表

- **Parts（部分）**：二维字符串数组（数组的数组）
- **Row（行）**：Parts 的第一级数组，如 `parts[1]`
- **Column（列）**：从 parts 中相同索引位置提取的字符串数组
- **Step（步骤）**：一个命名的可调用对象
- **Split（拆分）**：将字符串数组转换为 parts 的过程
- **Justify（对齐）**：使 parts 中每列宽度相同的过程
- **Merge（合并）**：将 parts 转换回字符串数组的过程
- **Mode（模式）**：charwise（`v`）、linewise（`V`）或 blockwise（`<C-v>`）

### 对齐算法流程

```
字符串数组
    ↓
[Pre-split 步骤]
    ↓
Split 步骤 → Parts
    ↓
[Pre-justify 步骤]
    ↓
Justify 步骤 → 对齐的 Parts
    ↓
[Pre-merge 步骤]
    ↓
Merge 步骤
    ↓
对齐后的字符串数组
```

#### 详细说明

1. **Pre-split**：对字符串数组进行预处理（可选多个步骤）
2. **Split**：将字符串拆分为 parts（二维数组）
3. **Pre-justify**：对 parts 进行预处理（如过滤、修剪等）
4. **Justify**：对齐 parts，使每列宽度相同
5. **Pre-merge**：合并前的预处理
6. **Merge**：将 parts 合并回字符串数组

---

## 配置选项

### mappings（映射配置）

```lua
mappings = {
  start = 'ga',              -- 启动对齐（无预览）
  start_with_preview = 'gA', -- 启动对齐（带预览）
}
```

- **start**：启动对齐，设置分隔符后立即应用
- **start_with_preview**：启动对齐并实时预览，按 `<CR>` 接受，`<Esc>` 取消

### modifiers（修饰符配置）

修饰符是单个字符键，用于在对齐过程中交互式修改对齐行为。

```lua
modifiers = {
  -- 主要选项修饰符
  ['s'] = function(steps, opts) end,  -- 输入拆分模式
  ['j'] = function(steps, opts) end,  -- 选择对齐方向
  ['m'] = function(steps, opts) end,  -- 输入合并分隔符
  
  -- 添加预处理步骤的修饰符
  ['f'] = function(steps, opts) end,  -- 过滤表达式
  ['i'] = function(steps, opts) end,  -- 忽略匹配
  ['p'] = function(steps, opts) end,  -- 配对相邻部分
  ['t'] = function(steps, opts) end,  -- 修剪空白
  
  -- 删除步骤
  ['<BS>'] = function(steps, opts) end,  -- 删除上一个预处理步骤
  
  -- 特殊预配置
  ['='] = function(steps, opts) end,  -- 等号对齐
  [','] = function(steps, opts) end,  -- 逗号对齐
  ['|'] = function(steps, opts) end,  -- 竖线对齐
  [' '] = function(steps, opts) end,  -- 空格对齐
}
```

#### 修饰符函数签名

```lua
function(steps, opts)
  -- steps: 包含 pre_split, split, pre_justify, justify, pre_merge, merge
  -- opts: 选项表，可修改 split_pattern, justify_side, merge_delimiter 等
end
```

### options（默认选项）

```lua
options = {
  split_pattern = '',        -- 拆分模式（Lua 模式字符串或数组）
  justify_side = 'left',     -- 对齐方向：'left'/'center'/'right'/'none'
  merge_delimiter = '',      -- 合并时使用的分隔符
}
```

#### 选项详解

**split_pattern**
- 类型：`string` 或 `string[]`
- 默认：`''`（空字符串，不拆分）
- 示例：
  - `'='` - 按等号拆分
  - `'%s+'` - 按空白字符拆分
  - `{ '<', '>' }` - 先按 `<` 再按 `>` 循环拆分

**split_exclude_patterns**
- 类型：`string[]`
- 默认：`{}`
- 用途：定义哪些区域不应被匹配（如字符串、注释）
- 示例：`{ '".-"', "'.-'", '^%s*#.*' }`

**justify_side**
- 类型：`string` 或 `string[]`
- 可选值：`'left'`、`'center'`、`'right'`、`'none'`
- 默认：`'left'`
- 数组示例：`{ 'right', 'left' }` - 第一列右对齐，第二列左对齐，然后循环

**justify_offsets**
- 类型：`number[]`
- 默认：零数组
- 用途：调整第一列的偏移量（在 charwise 模式自动设置）

**merge_delimiter**
- 类型：`string` 或 `string[]`
- 默认：`''`
- 示例：
  - `' '` - 用单个空格合并
  - `{ '', ' ' }` - 第一个分隔符无空格，第二个有空格

### steps（步骤配置）

```lua
steps = {
  pre_split = {},    -- 预拆分步骤数组
  split = nil,       -- 拆分步骤（nil 使用默认）
  pre_justify = {},  -- 预对齐步骤数组
  justify = nil,     -- 对齐步骤（nil 使用默认）
  pre_merge = {},    -- 预合并步骤数组
  merge = nil,       -- 合并步骤（nil 使用默认）
}
```

#### 自定义步骤示例

```lua
-- 默认只对齐第一对列
steps = {
  pre_justify = { align.gen_step.filter('n == 1') }
}

-- 默认右对齐并移除缩进
steps = {
  pre_justify = { align.gen_step.trim('both', 'remove') }
}
```

### silent（静默模式）

```lua
silent = false  -- true 则不显示提示信息
```

---

## 内置修饰符

### 主要选项修饰符

#### `s` - 输入拆分模式

输入 Lua 模式用于拆分，按 `<CR>` 确认。

**示例**：
```
原始文本：
a-b-c
aa-bb-cc

输入：s-<CR>
结果：
a -b -c
aa-bb-cc
```

#### `j` - 选择对齐方向

提示输入单字符：`l`（左）、`c`（中）、`r`（右）、`n`（无）。

**示例**：
```
原始文本：
a_b_c
aa_bb_cc

输入：_jr（先拆分再右对齐）
结果：
 a_ b_ c
aa_bb_cc
```

#### `m` - 输入合并分隔符

输入合并时使用的分隔符，按 `<CR>` 确认。

**示例**：
```
原始文本：
a_b_c
aa_bb_cc

输入：_m--<CR>
结果：
a --_--b --_--c
aa--_--bb--_--cc
```

### 添加预处理步骤的修饰符

#### `f` - 过滤表达式

输入 Lua 表达式过滤要对齐的部分。

**可用变量**：
- `row` - 当前行号
- `ROW` - 总行数
- `col` - 当前列号
- `COL` - 当前行总列数
- `s` - 当前元素的字符串值
- `n` - 当前列对编号
- `N` - 当前行总列对数

**示例**：
```
原始文本：
a_b_c
aa_bb_cc

输入：_fn==1<CR>（只对齐第一对列）
结果：
a _b_c
aa_bb_cc

其他有用表达式：
- n >= (N - 1)  -- 对齐最后的等号
- row ~= 2      -- 跳过第二行
- col % 2 == 0  -- 只对齐偶数列
```

#### `i` - 忽略匹配

忽略字符串和注释中的拆分匹配。

**示例**：
```
原始文本：
/* This_is_comment */
a"_"_b
aa_bb

输入：_i
结果：
/* This_is_comment */
a"_"_b
aa  _bb
```

#### `p` - 配对相邻部分

将相邻的部分配对在一起对齐。

**示例**：
```
原始文本：
a_b_c
aaa_bbb_ccc

输入：_p
结果：
a_  b_  c
aaa_bbb_ccc
```

#### `t` - 修剪空白

从部分两侧删除空白（保留缩进）。

**示例**：
```
原始文本：
a   _   b   _   c
  aa _bb _cc

输入：_t
结果：
a   _b _c
  aa_bb_cc
```

#### `<BS>` - 删除预处理步骤

删除最后添加的预处理步骤。如果有多种类型，提示选择。

**示例**：
- `tp<BS>` - 只剩 "trim" 步骤
- `it<BS>` - 提示选择删除 pre-split 还是 pre-justify 步骤

### 特殊预配置修饰符

#### `=` - 等号对齐

特殊处理连续的 `=` 及相关操作符（`<=`、`==`、`===` 等）。

**示例**：
```
原始文本：
a=b
aa<=bb
aaa===bbb
aaaa   =   cccc

输入：=
结果：
a    =   b
aa   <=  bb
aaa  === bbb
aaaa =   cccc
```

#### `,` - 逗号对齐

按逗号拆分，修剪空白，配对相邻部分，用单空格合并。

**示例**：
```
原始文本：
a,b
aa,bb
aaa    ,    bbb

输入：,
结果：
a,   b
aa,  bb
aaa, bbb
```

#### `|` - 竖线对齐（表格）

按竖线拆分，修剪空白，用单空格合并。

**示例**：
```
原始文本：
|a|b|
|aa|bb|
|aaa    |    bbb   |

输入：|
结果：
| a   | b   |
| aa  | bb  |
| aaa | bbb |
```

#### `<Space>` - 空格对齐

压缩连续空白为单个空格，按空白拆分（保留缩进）。

**示例**：
```
原始文本：
a b c
  aa    bb   cc

输入：<Space>
结果：
  a  b  c
  aa bb cc
```

---

## 使用场景与示例

### 场景 1：对齐变量赋值

```
# 原始代码
a = 1
bb = 2
ccc = 3

# 操作：V 选择行，gA=
# 结果
a   = 1
bb  = 2
ccc = 3
```

### 场景 2：对齐 Markdown 表格

```
# 原始
|Name|Age|City|
|John|25|NYC|
|Alice|30|LA|

# 操作：V 选择行，gA|
# 结果
| Name  | Age | City |
| John  | 25  | NYC  |
| Alice | 30  | LA   |
```

### 场景 3：对齐 JSON/对象

```
# 原始
{
  name: "John",
  age: 25,
  city: "NYC"
}

# 操作：V 选择行，gA:jr
# 结果
{
   name: "John",
    age: 25,
   city: "NYC"
}
```

### 场景 4：对齐注释

```
# 原始
local a = 1 -- first
local bb = 2 -- second
local ccc = 3 -- third

# 操作：V 选择行，gAs--<CR>
# 结果
local a   = 1 -- first
local bb  = 2 -- second
local ccc = 3 -- third
```

### 场景 5：复杂等号对齐

```
# 原始（来自官方示例）
a = 1
bbbb = 2
ccccccc = 3
ddd = 4
eeee === eee = eee = eee=f
fff = ggg += gg &&= gg
g != hhhhhhhh == 888

# 操作：V 选择行，gA=
# 结果
a       =   1
bbbb    =   2
ccccccc =   3
ddd     =   4
eeee    === eee = eee = eee=f
fff     =   ggg += gg &&= gg
g       !=  hhhhhhhh == 888

# 继续操作：jc（居中对齐）
a        =   1
bbbb     =   2
ccccccc  =   3
ddd      =   4
eeee    ===  eee = eee = eee=f
fff      =   ggg += gg &&= gg
g       !=   hhhhhhhh == 888
```

### 场景 6：使用过滤器

```
# 原始
a = b = c
aa = bb = cc
aaa = bbb = ccc

# 操作：V 选择，gA=fn==1<CR>（只对齐第一个等号）
# 结果
a   = b = c
aa  = bb = cc
aaa = bbb = ccc
```

### 场景 7：块选择对齐

```
# 原始
function foo(a, b, c)
function bar(aa, bb, cc)
function baz(aaa, bbb, ccc)

# 操作：<C-v> 块选择参数部分，gA,
# 结果
function foo(a,   b,   c)
function bar(aa,  bb,  cc)
function baz(aaa, bbb, ccc)
```

---

## 高级用法

### 自定义修饰符

#### 示例 1：循环切换对齐方向

```lua
modifiers = {
  j = function(_, opts)
    local cycle = { left = 'center', center = 'right', right = 'left' }
    opts.justify_side = cycle[opts.justify_side] or 'left'
  end,
}
```

#### 示例 2：创建 Lua 表对齐修饰符

```lua
modifiers = {
  L = function(steps, opts)
    -- 按等号拆分
    opts.split_pattern = '='
    -- 右对齐第一列，左对齐第二列
    opts.justify_side = { 'right', 'left' }
    -- 修剪空白
    table.insert(steps.pre_justify, align.gen_step.trim('both'))
    -- 用 ' = ' 合并
    opts.merge_delimiter = ' = '
  end,
}
```

#### 示例 3：仅对齐第一次出现

```lua
modifiers = {
  F = function(steps, _)
    table.insert(steps.pre_justify, align.gen_step.filter('n == 1'))
  end,
}
```

### 自定义默认步骤

#### 示例 1：默认只对齐第一对列

```lua
steps = {
  pre_justify = { align.gen_step.filter('n == 1') },
}
```

#### 示例 2：默认居中对齐

```lua
options = {
  justify_side = 'center',
}
```

#### 示例 3：默认使用双空格合并

```lua
options = {
  merge_delimiter = '  ',
}
```

### 编程式使用 API

#### 对齐字符串数组

```lua
local align = require('mini.align')

local lines = {
  'a = 1',
  'bb = 2',
  'ccc = 3',
}

local aligned = align.align_strings(lines, {
  split_pattern = '=',
  justify_side = 'left',
  merge_delimiter = ' ',
})

-- 结果：
-- {
--   'a   = 1',
--   'bb  = 2',
--   'ccc = 3',
-- }
```

#### 使用自定义步骤

```lua
local align = require('mini.align')

local lines = { 'a=b', 'aa=bb' }

local aligned = align.align_strings(
  lines,
  { split_pattern = '=' },
  {
    pre_justify = {
      align.gen_step.trim('both'),
      align.gen_step.filter('n == 1')
    }
  }
)
```

#### 操作 Parts 对象

```lua
local align = require('mini.align')

local parts = align.as_parts({ { 'a', 'b' }, { 'c' } })

-- 获取维度
print(vim.inspect(parts.get_dims())) -- { row = 2, col = 2 }

-- 应用函数到每个元素
parts.apply_inplace(function(s, data)
  return s .. data.col
end)
-- 结果：{ { 'a1', 'b2' }, { 'c1' } }

-- 修剪并配对
parts.trim('both').pair('left')
-- 结果：{ { 'a1b2' }, { 'c1' } }
```

---

## API 参考

### MiniAlign.setup(config)

设置模块。

```lua
require('mini.align').setup({
  mappings = { start = 'ga', start_with_preview = 'gA' },
  modifiers = {},
  options = {},
  steps = {},
  silent = false,
})
```

### MiniAlign.align_strings(strings, opts, steps)

对齐字符串数组。

**参数**：
- `strings` (table): 字符串数组
- `opts` (table|nil): 选项表
- `steps` (table|nil): 步骤表

**返回**：
- (table): 对齐后的字符串数组

**示例**：
```lua
local result = align.align_strings(
  { 'a=1', 'bb=2' },
  { split_pattern = '=', justify_side = 'left' }
)
```

### MiniAlign.align_user(mode)

使用用户提供的步骤对齐当前区域。主要用于映射。

**参数**：
- `mode` (string): 选择模式（"char"、"line"、"block"）

### MiniAlign.as_parts(arr2d)

将二维字符串数组转换为 parts 对象。

**返回**：
- (table): Parts 对象，包含以下方法：
  - `apply(f)` - 应用函数到每个部分
  - `apply_inplace(f)` - 就地应用函数
  - `get_dims()` - 获取维度
  - `group(mask, direction)` - 基于掩码分组
  - `pair(direction)` - 配对相邻元素
  - `slice_col(j)` - 获取列
  - `slice_row(i)` - 获取行
  - `trim(direction, indent)` - 修剪空白

### MiniAlign.new_step(name, action)

创建步骤对象。

**参数**：
- `name` (string): 步骤名称
- `action` (function|table): 步骤动作（可调用对象）

**返回**：
- (table): 步骤对象 `{ name = ..., action = ... }`

### MiniAlign.gen_step

生成常用步骤的表。

#### MiniAlign.gen_step.default_split()

生成默认拆分步骤。使用 `split_pattern` 和 `split_exclude_patterns` 选项。

#### MiniAlign.gen_step.default_justify()

生成默认对齐步骤。使用 `justify_side` 和 `justify_offsets` 选项。

#### MiniAlign.gen_step.default_merge()

生成默认合并步骤。使用 `merge_delimiter` 选项。

#### MiniAlign.gen_step.filter(expr)

生成过滤步骤。

**参数**：
- `expr` (string): Lua 表达式字符串

**示例**：
```lua
align.gen_step.filter('n == 1')  -- 只对齐第一对列
align.gen_step.filter('row ~= 2')  -- 跳过第二行
```

#### MiniAlign.gen_step.ignore_split(patterns, exclude_comment)

生成忽略步骤，添加模式到 `split_exclude_patterns`。

**参数**：
- `patterns` (table): 模式数组，默认 `{ [[".-"]] }`
- `exclude_comment` (boolean|nil): 是否排除注释，默认 `true`

#### MiniAlign.gen_step.pair(direction)

生成配对步骤。

**参数**：
- `direction` (string): "left"（默认）或 "right"

#### MiniAlign.gen_step.trim(direction, indent)

生成修剪步骤。

**参数**：
- `direction` (string|nil): "both"、"left"、"right"、"none"，默认 "both"
- `indent` (string|nil): "keep"、"low"、"high"、"remove"，默认 "keep"

---

## 注意事项

### 使用建议

1. **块选择与 virtualedit**
   ```lua
   vim.o.virtualedit = 'block'  -- 或 'all'
   ```
   块选择模式（`<C-v>`）在设置 `virtualedit` 后效果最佳。

2. **预览与 showmode**
   ```lua
   vim.o.showmode = false
   ```
   带预览的对齐在禁用 `showmode` 时效果更好。

3. **注释字符串设置**
   确保正确设置 `commentstring`，以便 `i` 修饰符正确识别注释：
   ```lua
   vim.bo.commentstring = '# %s'  -- Python
   vim.bo.commentstring = '// %s'  -- C/C++/JavaScript
   ```

### 常见问题

#### Q1: 为什么预览不工作？

**A**: 确保：
- 使用 `gA` 而不是 `ga`
- 未设置 `silent = true`
- 检查是否有键位映射冲突

#### Q2: 如何对齐最后一个等号？

**A**: 使用过滤器表达式：
```
gA=fn>=(N-1)<CR>
```

#### Q3: 如何跳过某些行？

**A**: 使用过滤器：
```
gA=frow~=2<CR>  -- 跳过第二行
```

#### Q4: 如何只对齐前 N 列？

**A**: 使用过滤器：
```
gA=fn<=2<CR>  -- 只对齐前两对列
```

#### Q5: 修饰符不生效？

**A**: 检查：
- 修饰符顺序（某些修饰符需要先设置拆分模式）
- 是否与内置修饰符冲突
- 查看状态消息了解当前步骤

#### Q6: 如何禁用插件？

**A**: 全局或 buffer 局部禁用：
```lua
vim.g.minialign_disable = true  -- 全局
vim.b.minialign_disable = true  -- buffer 局部
```

#### Q7: 如何查看当前对齐状态？

**A**: 插件会自动显示状态消息，除非设置了 `silent = true`。状态消息包含：
- 当前拆分模式
- 当前对齐方向
- 已应用的步骤

### 性能考虑

- 对于大文件（>1000 行），考虑只选择需要对齐的部分
- 块选择模式比行模式更高效
- 复杂的过滤表达式可能影响性能

### 与其他插件比较

**vs vim-easy-align**:
- `mini.align` 允许完全自定义修饰符
- `mini.align` 不区分分隔符和其他部分
- `mini.align` 默认对齐所有匹配
- `mini.align` 支持 Lua 表达式过滤

**vs tabular**:
- `mini.align` 使用交互式修饰符而非命令参数
- `mini.align` 需要明确选择区域（不自动检测）
- `mini.align` 提供实时预览功能

---

## 实用配置片段

### 快速上手配置

```lua
-- 只需基础功能
require('mini.align').setup({
  mappings = {
    start = 'ga',
    start_with_preview = 'gA',
  },
})
```

### 增强体验配置

```lua
local align = require('mini.align')

align.setup({
  mappings = {
    start = 'ga',
    start_with_preview = 'gA',
  },
  
  modifiers = {
    -- 更友好的对齐方向切换
    j = function(_, opts)
      local sides = { 'left', 'center', 'right' }
      local current = vim.tbl_contains(sides, opts.justify_side) 
        and vim.fn.index(sides, opts.justify_side) + 1 
        or 0
      opts.justify_side = sides[(current % #sides) + 1]
    end,
  },
})

-- 推荐设置
vim.o.virtualedit = 'block'
vim.o.showmode = false
```

### 为特定文件类型自定义

```lua
-- 在 ftplugin/lua.lua 中
vim.b.minialign_config = {
  modifiers = {
    -- Lua 表键值对对齐
    ['='] = function(steps, opts)
      opts.split_pattern = '='
      opts.justify_side = { 'right', 'left' }
      opts.merge_delimiter = ' = '
      table.insert(steps.pre_justify, require('mini.align').gen_step.trim('both'))
    end,
  },
}
```

---

## 总结

`mini.align` 是一个功能强大、高度可定制的对齐插件。其核心优势在于：

1. **交互式体验**：通过单键修饰符实时调整对齐行为
2. **灵活的算法**：三步对齐流程支持复杂场景
3. **完全可定制**：所有修饰符和步骤都可自定义
4. **实时预览**：立即看到对齐效果
5. **纯 Lua 实现**：与 Neovim 深度集成

建议从基础使用开始，逐步探索高级功能和自定义选项。

---

**相关链接**：
- [GitHub 仓库](https://github.com/echasnovski/mini.nvim)
- [完整文档](https://github.com/echasnovski/mini.nvim/blob/main/doc/mini-align.txt)
- [Mini.nvim 生态](https://github.com/echasnovski/mini.nvim)
