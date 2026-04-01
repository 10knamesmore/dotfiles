# lazy.nvim 最佳实践

> 基于 `/Users/wanger/.local/share/nvim/lazy/lazy.nvim` 源码分析。

---

## 1. Plugin Spec 核心字段

| 字段 | 说明 |
|-----|------|
| `[1]` | 短 URL，如 `"folke/tokyonight.nvim"` |
| `name` | 显示名称，也用于查找配置文件 |
| `lazy` | 是否延迟加载；有 event/cmd/ft/keys 时自动为 `true` |
| `enabled` | 局部启用/禁用，支持函数 |
| `cond` | 全局条件，影响整个依赖链 |
| `priority` | 加载顺序，仅对 `lazy=false` 的插件生效，默认 50 |
| `optional` | 仅作为可选依赖存在，父插件禁用时此插件也会被移除 |
| `virtual` | 虚拟插件，不安装、不加入 rtp，仅作为配置容器 |

---

## 2. 懒加载触发方式

### event

- 事件名称**大小写敏感**，必须是驼峰形式：`BufReadPost`，不是 `bufreadpost`。
- 自定义 User 事件写法：`"User VeryLazy"`，空格分隔。
- 事件触发后，autocmd 自动删除自身，加载插件，然后**重新触发**事件链（事件有依赖关系：`FileType → BufReadPost → BufReadPre`）。
- `VeryLazy` 是内置别名，映射到 `User VeryLazy`，在 UI 启动后触发，适合非紧急插件。

### cmd

- 命令首字母必须大写。
- 触发时：删除占位命令 → 加载插件 → 重新执行原始命令（包括参数）。
- **插件必须实际定义该命令**，否则加载后报错。
- Tab 补全也会触发加载。

### ft

- 等价于 `event = "FileType <ft>"`。
- 加载时会额外执行 `ftdetect` 脚本。

### keys

- 格式：`{ "<leader>x", cmd_or_func, desc = "...", mode = "n" }`
- **mode 默认为 `"n"`**，多模式需显式指定：`mode = { "n", "v" }`。
- 触发时：删除临时映射 → 加载插件 → 重新喂入原始按键序列。
- **插件必须实际定义同名映射**，否则按键落空。

---

## 3. init / config / opts 的执行顺序

```
Neovim 启动
  │
  ├─ 1. 所有插件的 init() — 无论 lazy 与否，立即执行
  │
  ├─ 2. lazy=false 插件按 priority 降序加载
  │       └─ 每个插件：加载依赖 → packadd → config(plugin, opts)
  │
  └─ 3. 懒加载插件在触发时：
            init() 已在启动时执行
            加载时：依赖 → packadd → config(plugin, opts)
```

| | init | config | opts |
|--|------|--------|------|
| **执行时机** | 启动时，总是执行 | 插件加载后执行 | 作为参数传给 config |
| **适合做什么** | 设置 `vim.g.*`、`vim.opt.*` 等全局状态 | 调用 `require("plugin").setup(opts)` | 声明插件选项 |
| **不适合做什么** | 调用插件 API（插件还未加载） | 修改未加载插件的状态 | — |

### opts 的 merge 行为

多个 spec fragment 的 opts 会**深度 merge**，后者覆盖前者：

```lua
-- fragment 1
{ "plugin", opts = { a = 1, b = 2 } }

-- fragment 2（覆盖 a，保留 b）
{ "plugin", opts = { a = 10 } }
-- 最终 opts = { a = 10, b = 2 }
```

函数形式可访问继承的 opts，适合需要在上一层基础上修改的场景：

```lua
{
  "plugin",
  opts = function(_, opts)
    -- opts 已包含其他 fragment 合并的结果
    opts.on_attach = wrap(opts.on_attach)
    return opts
  end,
}
```

---

## 4. 多文件 Spec 组织

`import` 会递归加载指定 Lua 模块路径下的所有文件，**按字母序**处理：

```lua
require("lazy").setup({
  { import = "plugins.editor" },
  { import = "plugins.lsp" },
  { import = "plugins.lang" },
})
```

推荐目录结构：

```
~/.config/nvim/lua/
└── plugins/
    ├── editor.lua      -- 编辑增强
    ├── ui.lua          -- 界面
    ├── lsp/
    │   ├── init.lua    -- LSP 框架配置
    │   └── servers.lua -- 各 LSP server 配置
    └── lang/
        ├── python.lua
        └── rust.lua
```

注意：
- 每个模块必须 `return { ... }` 单一返回值。
- 如果模块间有顺序依赖，不能靠文件名排序保证，需改用 `dependencies`。

---

## 5. dependencies

- 依赖加载顺序：**递归加载所有依赖（含依赖的依赖）→ packadd 主插件 → config 主插件**。
- 每个依赖的 `config` 在自身加载时立即执行，早于主插件的 `config`。
- 仅作为依赖存在的插件自动标记为 `lazy=true`，不会影响启动时间。
- `optional=true` 的依赖：父插件被 `enabled=false` 时，可选依赖也会被移除。

---

## 6. build 钩子

在插件安装或更新后触发，支持多种形式：

| 形式 | 示例 |
|-----|------|
| Shell 命令 | `build = "make"` |
| Vim 命令 | `build = ":helptags doc"` |
| Lua 文件 | `build = "build.lua"` |
| 函数 | `build = function(plugin) ... end` |
| 组合 | `build = { "make", ":TSUpdate" }` |
| 禁用 | `build = false` |

无 `build` 字段时，自动检测插件目录下的 `build.lua`。

---

## 7. priority

- **只对 `lazy=false` 的启动插件有效**，懒加载插件设置 priority 无意义。
- 数值越大越先加载，默认 50。
- Colorscheme 插件标准做法：

```lua
{
  "folke/tokyonight.nvim",
  lazy = false,
  priority = 1000,
  config = function()
    vim.cmd.colorscheme("tokyonight")
  end,
}
```

---

## 8. cond vs enabled

| | cond | enabled |
|--|------|---------|
| **评估时机** | spec 解析阶段（启动时一次） | 禁用阶段（同样是启动时） |
| **为 false 时** | 插件及全部依赖链标记为忽略，不加入清理列表（文件保留磁盘） | 可选插件删除；必需插件进入 disabled 集合 |
| **典型用途** | 全局环境条件（如 VSCode 内嵌 Neovim）| 功能性条件（如某 CLI 工具不存在） |
| **级联效应** | 有，影响整个依赖链 | 无 |

```lua
-- cond: 全局条件，整个依赖链跟着禁用
{ "plugin", cond = function() return not vim.g.vscode end }

-- enabled: 局部条件，只影响该插件
{ "plugin", enabled = function() return vim.fn.executable("ccls") == 1 end }
```

---

## 9. version 和 pin

semver 匹配规则（来自 `manage/semver.lua`）：

| 写法 | 匹配范围 |
|-----|---------|
| `"*"` 或 `""` | 任何版本 |
| `"1.2.3"` | ≥1.2.3, <1.2.4 |
| `"1.2"` | ≥1.2.0, <1.3.0 |
| `"^1.2.3"` | ≥1.2.3, <2.0.0 |
| `"~1.2.3"` | ≥1.2.3, <1.3.0 |
| `">1.2.3"` | >1.2.3 |
| `"1.2.3 - 2.0.0"` | ≥1.2.3, <2.0.0 |

- `pin = true`：锁定当前 commit，不更新。
- 预发布版本（`1.0.0-rc1`）只匹配具有相同预发布标签的范围。

---

## 10. 常见陷阱

### 事件名称大小写
```lua
-- ❌
{ event = "bufreadpost" }
-- ✓
{ event = "BufReadPost" }
```

### keys 的 mode 默认为 "n"
```lua
-- ❌ 期望 n+v，实际只有 n
{ keys = { { "<leader>c", cmd } } }
-- ✓
{ keys = { { "<leader>c", cmd, mode = { "n", "v" } } } }
```

### cmd/keys 要求插件实际定义对应命令/映射
```lua
-- ❌ 加载后插件没有定义 MyCommand，报错
{ "plugin", cmd = "MyCommand" }
```

### init 里不能调用插件 API
```lua
-- ❌ 插件还未加载
init = function()
  require("telescope").setup(...)  -- 错误
end
-- ✓ 应放在 config
config = function()
  require("telescope").setup(...)
end
```

### opts 是 merge 不是覆盖
```lua
-- 两个 fragment 的 opts 会深度 merge，不是后者完全覆盖前者
-- 如需清除继承的某个 key，显式设为 nil 或用函数形式
{ "plugin", opts = function(_, opts) opts.key = nil; return opts end }
```

### cond 的级联效应
```lua
-- cond=false 会连带禁用整个依赖链，包括被其他插件共享的依赖
-- 被共享的依赖仍会被其他插件加载，不会真正消失
{ "plugin", cond = false, dependencies = { "shared-dep" } }
```

### priority 对懒加载插件无效
```lua
-- ❌ priority 不影响懒加载插件的触发顺序
{ "plugin", event = "VeryLazy", priority = 100 }
-- ✓ priority 只对 lazy=false 有效
{ "plugin", lazy = false, priority = 100 }
```

### import 按字母序，不保证顺序
```lua
-- 如果 a.lua 依赖 b.lua 中的配置，不能靠文件名排序保证
-- 应使用 dependencies 显式声明顺序
```
