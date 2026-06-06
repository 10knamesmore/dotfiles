# overseer.nvim 完整中文文档

## 目录

- [插件简介](#插件简介)
- [主要特性](#主要特性)
- [系统要求](#系统要求)
- [安装配置](#安装配置)
  - [使用 lazy.nvim 安装](#使用-lazynvim-安装)
  - [最简配置](#最简配置)
  - [高级配置示例](#高级配置示例)
- [核心概念](#核心概念)
  - [任务 (Tasks)](#任务-tasks)
  - [组件 (Components)](#组件-components)
  - [模板 (Templates)](#模板-templates)
  - [策略 (Strategies)](#策略-strategies)
- [配置项详解](#配置项详解)
- [命令使用](#命令使用)
- [任务列表快捷键](#任务列表快捷键)
- [Lua API](#lua-api)
- [内置组件详解](#内置组件详解)
- [典型使用场景](#典型使用场景)
- [第三方集成](#第三方集成)
- [常见问题与注意事项](#常见问题与注意事项)

---

## 插件简介

**overseer.nvim** 是一个功能强大的 Neovim 任务运行器和作业管理插件。它提供了一个统一的界面来运行、管理和监控各种构建、测试和开发任务。

### 核心设计理念

- **极致的可扩展性**：基于实体组件系统 (ECS) 架构，任务行为完全由组件控制
- **序列化支持**：任务和组件可以保存到磁盘并恢复
- **模块化设计**：功能通过组件组合，易于定制和扩展
- **VS Code 任务兼容**：最完整的 VS Code tasks.json 支持

---

## 主要特性

1. **内置多种任务框架支持**
   - make, npm, cargo, composer, deno, just, mix, rake, tox
   - VS Code 的 `.vscode/tasks.json`
   - 自定义任务模板

2. **强大的输出处理**
   - 与 `vim.diagnostic` 和 quickfix 无缝集成
   - 支持多种输出解析器（errorformat, problem matcher, 自定义函数）
   - 实时输出监控和摘要

3. **灵活的任务管理**
   - 可视化任务列表 UI
   - 任务编辑器，可动态修改任务组件
   - 任务依赖和顺序执行支持

4. **丰富的组件系统**
   - 自动通知（完成时、输出时）
   - 文件监控，保存时自动重启任务
   - 超时控制
   - 诊断和 quickfix 集成
   - 任务去重

5. **集成生态**
   - nvim-dap 集成（支持 preLaunchTask 和 postDebugTask）
   - Lualine/Heirline 状态栏组件
   - Neotest 集成
   - 会话管理器支持

---

## 系统要求

- **Neovim 0.11+** （旧版本请使用 `nvim-0.x` 分支）

> **注意**：从最新版本开始，overseer 已放弃对 Neovim 0.11 以下版本的支持。

---

## 安装配置

### 使用 lazy.nvim 安装

#### 最简配置

```lua
{
  'stevearc/overseer.nvim',
  ---@module 'overseer'
  ---@type overseer.SetupOpts
  opts = {},
}
```

安装后只需在 `init.lua` 中添加：

```lua
require("overseer").setup()
```

然后使用：
- `:OverseerRun` - 选择并运行任务
- `:OverseerToggle` - 打开/关闭任务列表

#### 带依赖和快捷键的配置

```lua
{
  'stevearc/overseer.nvim',
  cmd = { 'OverseerRun', 'OverseerToggle', 'OverseerBuild' },
  keys = {
    { '<leader>ow', '<cmd>OverseerToggle<cr>', desc = 'Overseer: 任务列表' },
    { '<leader>or', '<cmd>OverseerRun<cr>', desc = 'Overseer: 运行任务' },
    { '<leader>oa', '<cmd>OverseerTaskAction<cr>', desc = 'Overseer: 任务操作' },
    { '<leader>os', '<cmd>OverseerShell<cr>', desc = 'Overseer: Shell 命令' },
  },
  opts = {
    -- 配置项
  },
}
```

### 高级配置示例

```lua
{
  'stevearc/overseer.nvim',
  config = function()
    local overseer = require('overseer')
    
    overseer.setup({
      -- 启用 nvim-dap 集成
      dap = true,
      
      -- 任务输出配置
      output = {
        use_terminal = true,     -- 使用终端缓冲区显示输出
        preserve_output = false, -- 任务重启时是否保留输出
      },
      
      -- 任务列表配置
      task_list = {
        direction = 'bottom',        -- 打开方向: "left", "right", "bottom"
        min_height = 10,
        max_height = { 20, 0.3 },   -- 最多 20 行或 30% 的高度
        default_detail = 1,          -- 显示详细程度
        
        -- 自定义按键映射
        keymaps = {
          ['?'] = 'keymap.show_help',
          ['<CR>'] = 'keymap.run_action',
          ['dd'] = { 'keymap.run_action', opts = { action = 'dispose' } },
          ['<C-e>'] = { 'keymap.run_action', opts = { action = 'edit' } },
          ['o'] = 'keymap.open',
          ['<C-v>'] = { 'keymap.open', opts = { dir = 'vsplit' } },
          ['<C-s>'] = { 'keymap.open', opts = { dir = 'split' } },
          ['<C-f>'] = { 'keymap.open', opts = { dir = 'float' } },
          ['p'] = 'keymap.toggle_preview',
          ['{'] = 'keymap.prev_task',
          ['}'] = 'keymap.next_task',
          ['<C-k>'] = 'keymap.scroll_output_up',
          ['<C-j>'] = 'keymap.scroll_output_down',
        },
      },
      
      -- 组件别名配置
      component_aliases = {
        default = {
          'on_exit_set_status',
          'on_complete_notify',
          { 'on_complete_dispose', timeout = 600 }, -- 10分钟后自动清理
        },
        -- 添加快速反馈的别名
        quick_feedback = {
          'on_exit_set_status',
          { 'on_output_quickfix', open = true },
          'on_complete_notify',
        },
      },
      
      -- 添加自定义任务目录
      template_dirs = {
        'my_custom_tasks',  -- 在 runtimepath 下查找 lua/my_custom_tasks/
      },
      
      -- 自定义操作
      actions = {
        ['重启并打开输出'] = {
          desc = '重启任务并在浮动窗口中打开输出',
          run = function(task)
            task:restart(true)
            vim.defer_fn(function()
              task:open_output('float')
            end, 100)
          end,
        },
      },
      
      -- 模板提供器超时设置
      template_timeout_ms = 3000,
      template_cache_threshold_ms = 200,
      
      -- 日志级别
      log_level = vim.log.levels.WARN,
    })
    
    -- 注册自定义任务别名
    overseer.register_alias('build_and_test', {
      'default',
      { 'on_output_quickfix', open = true },
    })
    
    -- 为特定项目添加钩子
    overseer.add_template_hook({
      dir = vim.fn.expand('~/projects/myproject')
    }, function(task_defn, util)
      util.add_component(task_defn, { 'on_output_quickfix', open = true })
      task_defn.env = vim.tbl_extend('force', task_defn.env or {}, {
        MY_PROJECT_VAR = 'custom_value'
      })
    end)
  end,
}
```

---

## 核心概念

### 任务 (Tasks)

任务代表要执行的单个命令。每个任务包含：

- **cmd**: 要运行的命令（字符串或数组）
- **cwd**: 工作目录
- **env**: 环境变量
- **components**: 附加的组件列表
- **status**: 当前状态（PENDING, RUNNING, SUCCESS, FAILURE, CANCELED, DISPOSED）
- **metadata**: 用户自定义元数据

**任务生命周期**：
```
PENDING → RUNNING → (SUCCESS|FAILURE|CANCELED) → DISPOSED
```

### 组件 (Components)

组件是任务的功能扩展单元，基于实体组件系统 (ECS) 设计。组件可以：

- 监听任务事件（如 `on_start`, `on_complete`, `on_output`）
- 修改任务行为
- 处理任务输出
- 与 Neovim 功能集成

**常见组件类型**：
- **生命周期管理**: `on_complete_dispose`, `timeout`
- **输出处理**: `on_output_parse`, `on_output_quickfix`
- **通知**: `on_complete_notify`, `on_output_notify`
- **诊断**: `on_result_diagnostics`, `on_result_diagnostics_quickfix`
- **自动化**: `restart_on_save`, `on_complete_restart`
- **任务控制**: `unique`, `dependencies`, `run_after`

### 模板 (Templates)

模板用于定义可重用的任务配置。模板出现在 `:OverseerRun` 命令中。

**模板定义**：
```lua
-- lua/overseer/template/user/my_task.lua
return {
  name = "我的任务",
  builder = function(params)
    return {
      cmd = { 'echo', params.message },
      components = { 'default' },
    }
  end,
  params = {
    message = {
      type = "string",
      default = "Hello World",
      description = "要显示的消息",
    },
  },
  condition = {
    filetype = { "lua" },  -- 仅在 lua 文件中显示
  },
  tags = { overseer.TAG.BUILD },
}
```

**模板提供器**：
动态生成多个模板，适用于需要根据配置文件生成任务的场景。

```lua
-- lua/overseer/template/user/my_provider.lua
return {
  generator = function(search)
    -- 根据 Makefile 生成任务
    local tasks = {}
    -- ... 解析并生成任务列表
    return tasks
  end,
  cache_key = function(opts)
    -- 返回配置文件路径用于缓存
    return vim.fs.find('Makefile', { path = opts.dir })[1]
  end,
}
```

### 策略 (Strategies)

策略控制任务如何实际运行。

**内置策略**：

1. **jobstart** (默认)
   - 使用 `vim.fn.jobstart()` 运行
   - 支持终端和普通缓冲区
   
2. **system**
   - 使用 `vim.system()` 运行
   - 适合需要系统调用的场景

3. **orchestrator**
   - 元任务，管理一系列任务的执行顺序
   - 支持顺序和并行执行

4. **test**
   - 用于单元测试

---

## 配置项详解

### `dap` (boolean, 默认: true)

是否为 nvim-dap 添加 `preLaunchTask` 和 `postDebugTask` 支持。

### `output` (table)

任务输出缓冲区配置：

```lua
output = {
  use_terminal = true,      -- 使用终端缓冲区（支持 ANSI 转义码）
  preserve_output = false,  -- 重启时保留输出
}
```

### `task_list` (table)

任务列表窗口配置：

```lua
task_list = {
  direction = "bottom",        -- 打开方向: "left"|"right"|"bottom"
  
  -- 尺寸配置（可以是数字或 0-1 的小数）
  max_width = { 100, 0.2 },   -- 最多 100 列或 20%
  min_width = { 40, 0.1 },    -- 至少 40 列或 10%
  max_height = { 20, 0.2 },
  min_height = 8,
  
  separator = "━━━━━━",        -- 任务分隔符
  child_indent = { "┃ ", "┣━", "┗━" },  -- 子任务缩进
  
  -- 自定义渲染函数
  render = function(task)
    return require("overseer.render").format_standard(task)
  end,
  
  -- 排序函数
  sort = function(a, b)
    return require("overseer.task_list").default_sort(a, b)
  end,
  
  -- 按键映射
  keymaps = {
    ['?'] = 'keymap.show_help',
    ['<CR>'] = 'keymap.run_action',
    -- ... 更多按键
  },
}
```

### `form` (table)

任务参数输入和编辑窗口配置：

```lua
form = {
  zindex = 40,
  min_width = 80,
  max_width = 0.9,
  min_height = 10,
  max_height = 0.9,
  win_opts = {},  -- 窗口选项
}
```

### `task_win` (table)

浮动任务输出窗口配置：

```lua
task_win = {
  padding = 2,   -- 边距
  win_opts = {}, -- 窗口选项
}
```

### `component_aliases` (table)

组件别名定义：

```lua
component_aliases = {
  default = {
    "on_exit_set_status",
    "on_complete_notify",
    { "on_complete_dispose", timeout = 300 },
  },
  default_vscode = {
    "default",
    "on_result_diagnostics",
  },
}
```

### `template_dirs` (string[])

额外的模板搜索目录：

```lua
template_dirs = {
  "my_tasks",  -- 将在 lua/my_tasks/ 下查找
}
```

### `template_timeout_ms` (number, 默认: 3000)

模板提供器超时时间（毫秒）。设为 0 表示永不超时。

### `template_cache_threshold_ms` (number, 默认: 200)

如果模板提供器运行时间超过此值，将缓存结果。

### `actions` (table)

自定义任务操作：

```lua
actions = {
  ["我的操作"] = {
    desc = "操作描述",
    condition = function(task)
      return task.status == overseer.STATUS.RUNNING
    end,
    run = function(task)
      -- 执行操作
    end,
  },
  
  -- 禁用内置操作
  watch = false,
}
```

### `log_level` (number)

日志级别：`vim.log.levels.TRACE|DEBUG|INFO|WARN|ERROR`

### `experimental_wrap_builtins` (table)

实验性功能，包装 `vim.system` 和 `vim.fn.jobstart`：

```lua
experimental_wrap_builtins = {
  enabled = false,
  condition = function(cmd, caller, opts)
    -- 决定是否为此调用创建 overseer 任务
    return true
  end,
}
```

---

## 命令使用

### `:OverseerOpen[!] [direction]`

打开任务列表窗口。
- `[direction]`: 可选，指定方向 `left|right|bottom`
- `!`: 打开后光标保持在当前窗口

**示例**：
```vim
:OverseerOpen
:OverseerOpen! bottom
```

### `:OverseerClose`

关闭任务列表窗口。

### `:OverseerToggle[!] [direction]`

切换任务列表窗口显示状态。

### `:OverseerRun [name/tags]`

运行任务模板。

**示例**：
```vim
:OverseerRun                  " 显示所有可用任务
:OverseerRun make build       " 运行名为 "make build" 的任务
:OverseerRun BUILD            " 运行带 BUILD 标签的任务
```

### `:OverseerShell[!] [command]`

以 overseer 任务形式运行 shell 命令。
- `!`: 创建任务但不立即启动

**示例**：
```vim
:OverseerShell echo "Hello World"
:OverseerShell! long_running_command.sh
```

### `:OverseerTaskAction`

选择一个任务并对其执行操作。

---

## 任务列表快捷键

在任务列表窗口中：

| 按键 | 操作 |
|------|------|
| `?`, `g?` | 显示帮助 |
| `<CR>` | 运行操作菜单 |
| `dd` | 销毁任务 |
| `<C-e>` | 编辑任务 |
| `o` | 在当前窗口打开任务输出 |
| `<C-v>` | 在垂直分割窗口打开输出 |
| `<C-s>` | 在水平分割窗口打开输出 |
| `<C-t>` | 在新标签页打开输出 |
| `<C-f>` | 在浮动窗口打开输出 |
| `<C-q>` | 在 quickfix 中打开输出 |
| `p` | 切换预览 |
| `{` | 上一个任务 |
| `}` | 下一个任务 |
| `<C-k>` | 向上滚动输出 |
| `<C-j>` | 向下滚动输出 |
| `g.` | 切换显示包装的任务 |
| `q` | 关闭任务列表 |

---

## Lua API

### 任务管理

#### `overseer.setup(opts)`

初始化插件。

#### `overseer.new_task(opts)`

创建新任务。

```lua
local task = overseer.new_task({
  cmd = { './build.sh', 'all' },
  name = '构建项目',
  cwd = '/path/to/project',
  env = { DEBUG = '1' },
  components = { 
    { 'on_output_quickfix', open = true },
    'default' 
  },
  metadata = { custom_field = 'value' },
})
task:start()
```

#### `overseer.run_task(opts, callback)`

从模板运行任务。

```lua
-- 运行指定名称的任务
overseer.run_task({ name = "make all" })

-- 运行带特定标签的任务
overseer.run_task({ 
  tags = { overseer.TAG.BUILD },
  autostart = false  -- 不自动启动
}, function(task)
  if task then
    -- 任务创建后的回调
    print("任务已创建:", task.name)
  end
end)

-- 传递参数
overseer.run_task({
  name = "serve",
  params = { port = 8080 },
  cwd = "/custom/dir",
})
```

#### `overseer.list_tasks(opts)`

列出所有任务。

```lua
-- 列出所有运行中的任务
local running = overseer.list_tasks({ 
  status = overseer.STATUS.RUNNING 
})

-- 自定义过滤
local my_tasks = overseer.list_tasks({
  filter = function(task)
    return task.metadata.my_tag == "special"
  end,
})

-- 去重非运行中的任务
local unique_tasks = overseer.list_tasks({ unique = true })
```

### 窗口操作

#### `overseer.open(opts)`, `overseer.close()`, `overseer.toggle(opts)`

```lua
overseer.open({ 
  direction = "right",
  enter = false,  -- 不聚焦到任务列表
})

overseer.toggle({ 
  focus_task_id = task.id  -- 打开后聚焦到指定任务
})
```

### 模板和组件

#### `overseer.register_template(defn)`

直接注册任务模板。

```lua
overseer.register_template({
  name = "我的任务",
  builder = function(params)
    return {
      cmd = { 'echo', params.msg },
    }
  end,
  params = {
    msg = { type = "string", default = "Hello" },
  },
})
```

#### `overseer.add_template_hook(opts, hook)`

为模板添加钩子函数。

```lua
-- 为所有 cargo 任务添加 quickfix
overseer.add_template_hook(
  { module = "^cargo$" },
  function(task_defn, util)
    util.add_component(task_defn, { 
      "on_output_quickfix", 
      open = true 
    })
  end
)

-- 为特定项目的任务添加环境变量
overseer.add_template_hook(
  { 
    dir = "/path/to/project",
    name = "^npm.*"
  },
  function(task_defn, util)
    task_defn.env = vim.tbl_extend('force', task_defn.env or {}, {
      NODE_ENV = "development"
    })
  end
)
```

#### `overseer.register_alias(name, components, override)`

注册组件别名。

```lua
overseer.register_alias("my_build", {
  "default",
  { "on_output_quickfix", open = true },
  "on_result_diagnostics",
})
```

### 任务操作

#### `overseer.run_action(task, name)`

在任务上运行操作。

```lua
local task = overseer.list_tasks()[1]
overseer.run_action(task, "restart")  -- 重启任务
overseer.run_action(task, "open float")  -- 在浮动窗口打开
overseer.run_action(task)  -- 显示操作菜单
```

### Task 对象方法

#### 状态查询

```lua
task:is_pending()   -- 是否等待中
task:is_running()   -- 是否运行中
task:is_complete()  -- 是否已完成
task:is_disposed()  -- 是否已销毁
```

#### 生命周期控制

```lua
task:start()              -- 启动任务
task:stop()               -- 停止任务
task:restart(force_stop)  -- 重启任务
task:dispose(force)       -- 销毁任务
```

#### 组件管理

```lua
-- 添加组件
task:add_component("timeout")
task:add_component({ "timeout", timeout = 60 })
task:add_components({ "default" })

-- 设置组件（覆盖已存在的）
task:set_component({ "timeout", timeout = 120 })

-- 获取组件
local comp = task:get_component("timeout")

-- 移除组件
task:remove_component("timeout")
task:remove_components({ "timeout", "unique" })

-- 检查组件
if task:has_component("restart_on_save") then
  -- ...
end
```

#### 事件订阅

```lua
-- 订阅事件
task:subscribe("on_complete", function(t, status, result)
  print("任务完成:", status)
end)

-- 订阅并自动取消
task:subscribe("on_output_lines", function(t, lines)
  for _, line in ipairs(lines) do
    if line:match("ERROR") then
      print("发现错误!")
      return true  -- 返回 true 自动取消订阅
    end
  end
end)

-- 取消订阅
task:unsubscribe("on_complete", callback_fn)
```

#### 输出操作

```lua
-- 获取输出缓冲区
local bufnr = task:get_bufnr()

-- 在不同位置打开输出
task:open_output()            -- 当前窗口
task:open_output("float")     -- 浮动窗口
task:open_output("vertical")  -- 垂直分割
task:open_output("horizontal")-- 水平分割
task:open_output("tab")       -- 新标签页
```

#### 其他方法

```lua
-- 克隆任务
local new_task = task:clone()

-- 序列化（用于保存到磁盘）
local serialized = task:serialize()

-- 引用计数（防止被过早销毁）
task:inc_reference()
task:dec_reference()

-- 向所有任务广播事件
task:broadcast("custom_event")

-- 向任务的所有组件分发事件
task:dispatch("custom_event")
```

### 缓存管理

#### `overseer.preload_task_cache(opts, cb)`

预加载任务模板缓存。

```lua
-- 在进入目录时预加载
vim.api.nvim_create_autocmd({"VimEnter", "DirChanged"}, {
  callback = function()
    local cwd = vim.fn.getcwd()
    overseer.preload_task_cache({ dir = cwd })
  end,
})
```

#### `overseer.clear_task_cache(opts)`

清除任务模板缓存。

### 高级功能

#### `overseer.create_task_output_view(winid, opts)`

创建动态任务输出视图。

```lua
-- 始终显示最新的测试任务输出
overseer.create_task_output_view(0, {
  select = function(self, tasks, task_under_cursor)
    -- 按启动时间排序
    table.sort(tasks, function(a, b)
      return (a.time_start or 0) > (b.time_start or 0)
    end)
    return tasks[1]
  end,
})
```

---

## 内置组件详解

### 生命周期组件

#### `on_exit_set_status`

根据退出码设置任务状态。

```lua
{ "on_exit_set_status", success_codes = { 0, 1 } }
```

#### `on_complete_dispose`

任务完成后自动销毁。

```lua
{
  "on_complete_dispose",
  timeout = 600,  -- 10分钟后销毁
  statuses = { "SUCCESS", "FAILURE" },  -- 仅这些状态
  require_view = { "FAILURE" },  -- FAILURE 必须被查看后才销毁
}
```

#### `timeout`

任务超时自动取消。

```lua
{ "timeout", timeout = 120 }  -- 120 秒
```

### 通知组件

#### `on_complete_notify`

任务完成时通知。

```lua
{
  "on_complete_notify",
  statuses = { "FAILURE", "SUCCESS" },
  on_change = false,  -- 仅状态改变时通知
  system = "never",   -- 系统通知: "always"|"never"|"unfocused"
}
```

#### `on_output_notify`

长时间运行的任务显示实时输出摘要（需要 nvim-notify）。

```lua
{
  "on_output_notify",
  delay_ms = 2000,          -- 2秒后显示
  max_lines = 3,            -- 显示3行输出
  max_width = 60,
  trim = true,
  output_on_complete = true,
}
```

#### `on_result_notify`

任务产生结果时通知（适合监视任务）。

```lua
{
  "on_result_notify",
  infer_status_from_diagnostics = true,
  on_change = true,
  system = "unfocused",
}
```

### 输出处理组件

#### `on_output_parse`

解析任务输出并设置结果。

```lua
-- 使用 errorformat
{ "on_output_parse", errorformat = "%f:%l:%c: %m" }

-- 使用 VS Code problem matcher
{ "on_output_parse", problem_matcher = "$tsc" }

-- 使用自定义函数
{
  "on_output_parse",
  parser = function(line)
    local file, lnum, msg = line:match("^(.+):(%d+): (.+)$")
    if file then
      return {
        filename = file,
        lnum = tonumber(lnum),
        text = msg,
        type = "E",
      }
    end
  end
}
```

#### `on_output_quickfix`

将输出发送到 quickfix。

```lua
{
  "on_output_quickfix",
  open = true,              -- 自动打开 quickfix
  open_height = 10,
  close = false,            -- 无错误时关闭
  tail = true,              -- 实时更新
  items_only = false,       -- 仅显示匹配 errorformat 的行
  errorformat = "%f:%l: %m",
  set_diagnostics = true,   -- 同时设置诊断
  open_on_match = true,     -- 匹配时打开
  open_on_exit = "failure", -- 退出时打开: "never"|"failure"|"always"
}
```

#### `on_output_write_file`

将输出写入文件。

```lua
{ "on_output_write_file", filename = "/tmp/output.log" }
```

### 诊断组件

#### `on_result_diagnostics`

显示任务结果中的诊断。

```lua
{
  "on_result_diagnostics",
  remove_on_restart = true,
  virtual_text = true,
  underline = true,
  signs = true,
}
```

#### `on_result_diagnostics_quickfix`

将诊断添加到 quickfix。

```lua
{
  "on_result_diagnostics_quickfix",
  open = true,
  close = false,
  set_empty_results = true,
  use_loclist = false,
}
```

#### `on_result_diagnostics_trouble`

在 trouble.nvim 中打开诊断。

```lua
{
  "on_result_diagnostics_trouble",
  close = false,
  args = {},  -- 传递给 Trouble 的参数
}
```

### 自动化组件

#### `restart_on_save`

文件保存时重启任务。

```lua
{
  "restart_on_save",
  paths = { vim.fn.getcwd() },  -- 监视的路径
  delay = 500,                  -- 延迟 500ms
  interrupt = true,             -- 中断运行中的任务
  mode = "autocmd",             -- "autocmd"|"uv"
}
```

#### `on_complete_restart`

任务完成时自动重启（适合监视任务）。

```lua
{
  "on_complete_restart",
  statuses = { "FAILURE" },  -- 仅失败时重启
  delay = 500,
}
```

### 任务编排组件

#### `dependencies`

设置任务依赖。

```lua
{
  "dependencies",
  tasks = {
    "npm install",                    -- 任务名称
    { "cargo build", release = true }, -- 带参数
    { cmd = { "sleep", "5" } },       -- 原始任务定义
  },
  sequential = true,  -- 顺序执行
}
```

#### `run_after`

任务完成后运行其他任务。

```lua
{
  "run_after",
  tasks = { "npm serve" },
  statuses = { "SUCCESS" },
  detach = false,  -- 是否与父任务分离
}
```

### 控制组件

#### `unique`

确保任务唯一性。

```lua
{
  "unique",
  replace = true,              -- 替换现有任务
  restart_interrupts = true,   -- 重启时中断
  soft = false,                -- 仅清理已完成的重复任务
}
```

#### `open_output`

自动打开任务输出。

```lua
{
  "open_output",
  direction = "dock",        -- "dock"|"float"|"tab"|"vertical"|"horizontal"
  focus = false,
  on_start = "always",       -- "always"|"never"|"if_no_on_output_quickfix"
  on_complete = "failure",   -- "always"|"never"|"success"|"failure"
  on_result = "if_diagnostics",  -- "always"|"never"|"if_diagnostics"
}
```

---

## 典型使用场景

### 1. 快速执行 Shell 命令

创建别名：

```lua
vim.cmd.cnoreabbrev("OS OverseerShell")
```

使用：
```vim
:OS npm install
:OS! docker-compose up  " ! 表示创建但不启动
```

### 2. 重启最近的任务

```lua
vim.api.nvim_create_user_command("OverseerRestartLast", function()
  local overseer = require("overseer")
  local tasks = overseer.list_tasks({
    status = {
      overseer.STATUS.SUCCESS,
      overseer.STATUS.FAILURE,
      overseer.STATUS.CANCELED,
    },
    sort = require("overseer.task_list").sort_finished_recently
  })
  
  if #tasks > 0 then
    overseer.run_action(tasks[1], "restart")
  else
    vim.notify("没有找到任务", vim.log.levels.WARN)
  end
end, {})
```

### 3. 异步 :Make 命令（类似 vim-dispatch）

```lua
vim.api.nvim_create_user_command("Make", function(params)
  local cmd, num_subs = vim.o.makeprg:gsub("%$%*", params.args)
  if num_subs == 0 then
    cmd = cmd .. " " .. params.args
  end
  
  local task = overseer.new_task({
    cmd = vim.fn.expandcmd(cmd),
    components = {
      { "on_output_quickfix", open = not params.bang, open_height = 8 },
      "default",
    },
  })
  task:start()
end, {
  desc = "异步运行 makeprg",
  nargs = "*",
  bang = true,
})
```

使用：
```vim
:Make
:Make clean
:Make! test  " ! 表示不打开 quickfix
```

### 4. 构建 C++ 文件

创建文件 `~/.config/nvim/lua/overseer/template/user/cpp_build.lua`：

```lua
return {
  name = "g++ build",
  builder = function()
    local file = vim.fn.expand("%:p")
    local output = vim.fn.expand("%:p:r")  -- 移除扩展名
    return {
      cmd = { "g++", "-std=c++17", "-Wall", file, "-o", output },
      components = {
        { "on_output_quickfix", open = true },
        "on_result_diagnostics",
        "default",
      },
    }
  end,
  condition = {
    filetype = { "cpp", "c" },
  },
  tags = { overseer.TAG.BUILD },
}
```

### 5. 保存时运行脚本

```lua
-- ~/.config/nvim/lua/overseer/template/user/run_on_save.lua
return {
  name = "run script",
  builder = function()
    local file = vim.fn.expand("%:p")
    local cmd = { file }
    
    local ft = vim.bo.filetype
    if ft == "go" then
      cmd = { "go", "run", file }
    elseif ft == "python" then
      cmd = { "python3", file }
    elseif ft == "javascript" then
      cmd = { "node", file }
    end
    
    return {
      cmd = cmd,
      components = {
        { "on_output_quickfix", set_diagnostics = true },
        "on_result_diagnostics",
        { "restart_on_save", paths = { file } },
        "default",
      },
    }
  end,
  condition = {
    filetype = { "sh", "python", "go", "javascript" },
  },
  tags = { overseer.TAG.RUN },
}
```

### 6. 运行目录中的所有脚本

```lua
-- ~/.config/nvim/lua/overseer/template/user/run_scripts.lua
local files = require("overseer.files")

return {
  generator = function(opts)
    local scripts = vim.tbl_filter(function(filename)
      return filename:match("%.sh$") or filename:match("%.py$")
    end, files.list_files(opts.dir))
    
    local ret = {}
    for _, filename in ipairs(scripts) do
      table.insert(ret, {
        name = filename,
        builder = function()
          return {
            cmd = { vim.fs.joinpath(opts.dir, filename) },
            components = { "default" },
          }
        end,
      })
    end
    
    return ret
  end,
}
```

### 7. 项目本地任务（使用 exrc）

启用 exrc：
```lua
vim.o.exrc = true
```

在项目根目录创建 `.nvim.lua`：

```lua
-- /path/to/project/.nvim.lua
local overseer = require("overseer")

overseer.register_template({
  name = "项目构建",
  condition = {
    dir = vim.fn.getcwd(),  -- 仅在此目录可用
  },
  builder = function()
    return {
      cmd = { "./scripts/build.sh" },
      components = {
        { "on_output_quickfix", open = true },
        "default",
      },
    }
  end,
  tags = { overseer.TAG.BUILD },
})
```

### 8. 复杂的构建流水线

```lua
-- 使用 orchestrator 策略
local task = overseer.new_task({
  name = "完整构建流水线",
  strategy = {
    "orchestrator",
    tasks = {
      -- 第1步：清理
      "make clean",
      
      -- 第2步：并行构建前端和后端
      {
        { cmd = { "npm", "run", "build" } },
        { cmd = { "cargo", "build", "--release" } },
      },
      
      -- 第3步：运行测试
      "npm test",
      
      -- 第4步：打包
      { cmd = { "./scripts/package.sh" } },
    },
  },
})
task:start()
```

### 9. 监控任务（自动重启）

```lua
overseer.run_task({
  name = "TypeScript 监视",
  autostart = false
}, function(task)
  if task then
    -- 添加自动重启组件
    task:add_components({
      { "on_complete_restart", statuses = { "SUCCESS", "FAILURE" } },
      { "restart_on_save", paths = { "./src" } },
    })
    task:start()
  end
end)
```

### 10. 集成 Git 钩子

```lua
-- 提交前运行测试
vim.api.nvim_create_user_command("GitPreCommit", function()
  overseer.run_task({
    name = "运行测试",
  }, function(task)
    if task then
      task:subscribe("on_complete", function(t, status)
        if status == overseer.STATUS.SUCCESS then
          vim.notify("✓ 测试通过，可以提交", vim.log.levels.INFO)
        else
          vim.notify("✗ 测试失败，请修复后再提交", vim.log.levels.ERROR)
        end
      end)
    end
  end)
end, {})
```

### 11. 自定义参数输入

```lua
return {
  name = "Docker 构建",
  params = {
    tag = {
      type = "string",
      default = "latest",
      description = "镜像标签",
    },
    platform = {
      type = "enum",
      choices = { "linux/amd64", "linux/arm64" },
      default = "linux/amd64",
      description = "目标平台",
    },
    push = {
      type = "boolean",
      default = false,
      description = "构建后推送",
    },
  },
  builder = function(params)
    local cmd = { "docker", "build", "-t", "myapp:" .. params.tag }
    if params.platform then
      vim.list_extend(cmd, { "--platform", params.platform })
    end
    table.insert(cmd, ".")
    
    local components = { "default" }
    if params.push then
      table.insert(components, {
        "run_after",
        tasks = {{ cmd = { "docker", "push", "myapp:" .. params.tag } }},
      })
    end
    
    return {
      cmd = cmd,
      components = components,
    }
  end,
  condition = {
    callback = function()
      return vim.fn.filereadable("Dockerfile") == 1
    end,
  },
}
```

---

## 第三方集成

### Lualine 集成

```lua
require("lualine").setup({
  sections = {
    lualine_x = {
      {
        "overseer",
        label = "",     -- 任务计数前缀
        colored = true,
        symbols = {
          [overseer.STATUS.FAILURE] = "✗ ",
          [overseer.STATUS.CANCELED] = "⊘ ",
          [overseer.STATUS.SUCCESS] = "✓ ",
          [overseer.STATUS.RUNNING] = "↻ ",
        },
        unique = false,
        status = nil,   -- 要显示的状态列表
        filter = nil,   -- 过滤函数
      },
    },
  },
})
```

### Heirline 集成

```lua
local Overseer = {
  condition = function()
    return package.loaded.overseer
  end,
  init = function(self)
    local tasks = require("overseer.task_list").list_tasks({ 
      unique = true 
    })
    local tasks_by_status = require("overseer.util").tbl_group_by(
      tasks, 
      "status"
    )
    self.tasks = tasks_by_status
  end,
  static = {
    symbols = {
      ["CANCELED"] = " ",
      ["FAILURE"] = "✗ ",
      ["SUCCESS"] = "✓ ",
      ["RUNNING"] = "↻ ",
    },
  },
  provider = function(self)
    local parts = {}
    for status, symbol in pairs(self.symbols) do
      if self.tasks[status] then
        table.insert(parts, symbol .. #self.tasks[status])
      end
    end
    return table.concat(parts, " ")
  end,
}
```

### Neotest 集成

```lua
require("neotest").setup({
  consumers = {
    overseer = require("neotest.consumers.overseer"),
  },
  overseer = {
    enabled = true,
    force_default = true,  -- 强制所有测试使用 overseer
  },
})

-- 自定义 neotest 任务组件
require("overseer").setup({
  component_aliases = {
    default_neotest = {
      "on_exit_set_status",
      { "on_output_quickfix", open = true },
      "on_complete_notify",
    },
  },
})
```

### nvim-dap 集成

启用后会自动支持 VS Code 的 `preLaunchTask` 和 `postDebugTask`：

```lua
require("overseer").setup({
  dap = true,  -- 默认启用
})
```

在 `.vscode/launch.json` 中：

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Launch",
      "type": "node",
      "request": "launch",
      "program": "${workspaceFolder}/app.js",
      "preLaunchTask": "npm: build"
    }
  ]
}
```

### 会话管理器集成

#### resession.nvim

```lua
local resession = require("resession")
resession.setup({
  extensions = {
    overseer = {
      status = { "RUNNING" },  -- 仅保存运行中的任务
    },
  },
})
```

---

## 常见问题与注意事项

### 1. 任务消失了？

**原因**：默认配置会在任务完成后 5 分钟自动销毁（`on_complete_dispose` 组件）。

**解决方案**：
```lua
-- 修改超时时间
component_aliases = {
  default = {
    "on_exit_set_status",
    "on_complete_notify",
    { "on_complete_dispose", timeout = 1800 },  -- 30分钟
  },
}

-- 或完全禁用自动销毁
component_aliases = {
  default = {
    "on_exit_set_status",
    "on_complete_notify",
    -- 移除 on_complete_dispose
  },
}
```

### 2. 如何调试问题？

1. 运行健康检查：
```vim
:checkhealth overseer
```

2. 查看日志：
```lua
overseer.setup({
  log_level = vim.log.levels.DEBUG,  -- 或 TRACE
})
```

日志位置：`:lua print(vim.fn.stdpath("cache") .. "/overseer.log")`

### 3. `:OverseerRun` 没有显示任务？

**可能原因**：
- 任务框架未安装（如 `make`, `cargo` 等）
- 没有对应的配置文件（如 `Makefile`, `package.json`）
- 当前文件类型不匹配 `condition.filetype`

**检查方法**：
```vim
:checkhealth overseer
```

### 4. 组件参数的优先级

当同一个组件在不同地方定义时：

```lua
local task = overseer.new_task({
  cmd = "test",
  components = {
    -- 这个会生效
    { "on_complete_notify", statuses = { "SUCCESS" } },
    -- 这个会被忽略（组件已存在）
    "default"  -- default 也包含 on_complete_notify
  }
})
```

**最佳实践**：自定义组件放在前面，别名放在后面。

### 5. 任务在后台运行看不到输出？

使用 `open_output` 组件：

```lua
components = {
  { "open_output", on_start = "always", direction = "float" },
  "default",
}
```

### 6. 解析输出时出现乱码

**原因**：任务输出包含 ANSI 转义码（颜色等）。

**解决方案**：
```lua
-- 方法1: 禁用颜色
env = { NO_COLOR = "1" }

-- 方法2: 使用普通缓冲区
strategy = { "jobstart", use_terminal = false }

-- 方法3: 在命令中禁用颜色
cmd = { "cargo", "build", "--color=never" }
```

### 7. 性能优化

#### 预加载任务缓存

```lua
vim.api.nvim_create_autocmd({"VimEnter", "DirChanged"}, {
  callback = function()
    require("overseer").preload_task_cache({ 
      dir = vim.fn.getcwd() 
    })
  end,
})
```

#### 调整缓存阈值

```lua
overseer.setup({
  template_cache_threshold_ms = 100,  -- 更激进的缓存
})
```

### 8. 任务不序列化某些字段

**限制**：
- 函数不能序列化
- 事件订阅不能序列化
- 某些 userdata 不能序列化

**解决方案**：使用模板重新创建任务，而不是直接序列化任务实例。

### 9. 自定义错误格式

```lua
-- 使用 vim errorformat
errorformat = "%f:%l:%c: %m"

-- 多个格式
errorformat = table.concat({
  "%f:%l:%c: %m",
  "%f:%l: %m",
  "%f: %m",
}, ",")
```

常用 errorformat 变量：
- `%f` - 文件名
- `%l` - 行号
- `%c` - 列号
- `%m` - 错误消息
- `%t` - 错误类型 (E/W/I)
- `%n` - 错误编号

### 10. 限制任务数量

```lua
-- 在组件中实现
{
  "unique",
  replace = true,  -- 替换重复任务
}

-- 或手动管理
vim.api.nvim_create_autocmd("User", {
  pattern = "OverseerTaskNew",
  callback = function()
    local tasks = overseer.list_tasks({ status = "RUNNING" })
    if #tasks > 5 then
      vim.notify("运行任务过多", vim.log.levels.WARN)
    end
  end,
})
```

---

## 高级技巧

### 1. 创建任务组

```lua
local group_tasks = function(name, task_list)
  local parent = overseer.new_task({
    name = name,
    cmd = { "echo", "Group:", name },
    components = { "default" },
  })
  parent:start()
  
  for _, task_name in ipairs(task_list) do
    overseer.run_task({
      name = task_name,
      on_build = function(defn)
        defn.parent_id = parent.id
      end,
    })
  end
end

-- 使用
group_tasks("前端构建", { "npm install", "npm build", "npm test" })
```

### 2. 条件组件

```lua
local components = { "default" }

-- 仅在 CI 环境中添加特定组件
if vim.env.CI then
  table.insert(components, { "on_output_write_file", filename = "ci.log" })
end

local task = overseer.new_task({
  cmd = "test",
  components = components,
})
```

### 3. 任务间通信

```lua
local task1 = overseer.new_task({ cmd = "build" })
local task2 = overseer.new_task({ cmd = "deploy" })

task1:subscribe("on_complete", function(_, status)
  if status == overseer.STATUS.SUCCESS then
    task2:start()
  end
end)

task1:start()
```

### 4. 动态修改任务

```lua
local task = overseer.new_task({ cmd = "serve" })

-- 启动后添加文件监视
task:subscribe("on_start", function()
  task:add_component({ 
    "restart_on_save", 
    paths = { "./config" } 
  })
end)

task:start()
```

### 5. 自定义渲染

```lua
overseer.setup({
  task_list = {
    render = function(task)
      local lines = {}
      
      -- 自定义任务信息显示
      local status_icon = ({
        RUNNING = "↻",
        SUCCESS = "✓",
        FAILURE = "✗",
        PENDING = "○",
      })[task.status] or "?"
      
      table.insert(lines, {
        { status_icon .. " ", "Overseer" .. task.status },
        { task.name, "OverseerTask" },
        { " [" .. (task.metadata.custom or "N/A") .. "]", "Comment" },
      })
      
      return lines
    end,
  },
})
```

---

## 总结

overseer.nvim 是一个功能全面、高度可定制的任务管理插件。通过：

1. **组件系统** - 实现模块化和可扩展性
2. **模板系统** - 定义可重用的任务配置
3. **丰富的 API** - 支持复杂的任务编排
4. **广泛的集成** - 与 Neovim 生态无缝协作

你可以构建从简单的脚本执行到复杂的 CI/CD 流水线的各种任务管理方案。

**推荐学习路径**：
1. 从最简配置开始，熟悉 `:OverseerRun` 和 `:OverseerToggle`
2. 创建第一个自定义模板
3. 学习常用组件的使用
4. 探索高级功能（钩子、编排、集成）
5. 根据项目需求定制工作流

**相关资源**：
- GitHub: https://github.com/stevearc/overseer.nvim
- 官方文档: `:help overseer`
- 问题反馈: https://github.com/stevearc/overseer.nvim/issues

---

*本文档基于 overseer.nvim 最新版本编写*
