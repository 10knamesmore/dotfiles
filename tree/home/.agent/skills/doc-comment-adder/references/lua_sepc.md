# Lua 文档注释规范（LuaLS / EmmyLua 风格）

## 基本形式

* 使用 `---` 编写文档注释摘要。
* 使用 `---@...` 编写结构化标签，例如 `---@param`、`---@return`、`---@class`、`---@field`。
* 文档注释紧贴在被注释对象上方。
* 适用于模块、导出函数、方法、类风格对象、配置表、常量以及关键字段。

## 放置位置

* 函数、方法、局部变量、模块导出项的文档注释放在声明前。
* `---@class`、`---@field` 通常放在类型定义或约定表结构前。
* 对于简单私有辅助函数，可按需要省略；公共 API、关键逻辑、对调用者有约束的函数应补充注释。

## 基本结构

推荐结构如下：

* 第一行：`---` 摘要说明
* 后续：若有需要，使用 `---@param`、`---@return`、`---@nodiscard`、`---@class`、`---@field` 等标签补充

例如：

```lua
--- 解析用户 ID。
---@param raw string 原始用户 ID 字符串
---@return integer? user_id 解析成功后的用户 ID
---@return string? err 解析失败时的错误信息
local function parse_user_id(raw)
end
```

## 常用标签

### `---@param`

用于说明参数名称、类型、可选性与语义。

写法：

```lua
---@param name type 描述
```

可选参数使用 `?`：

```lua
---@param opts? table 可选配置
```

联合类型可写为：

```lua
---@param mode '"sync"'|'"async"' 执行模式
```

也可写常见基础联合：

```lua
---@param value string|number 输入值
```

### `---@return`

用于说明返回值类型、返回值名称与语义。

写法：

```lua
---@return type name 描述
```

多个返回值连续写：

```lua
---@return boolean ok 是否成功
---@return string? err 失败时的错误信息
```

### `---@class`

用于定义类风格对象或结构化 table。

```lua
---@class WorkerConfig
---@field name string 工作进程名称
---@field retry_limit integer 最大重试次数
---@field poll_interval_ms integer 轮询间隔，单位毫秒
```

### `---@field`

用于描述类、配置对象、记录表的字段。

```lua
---@field bufnr integer 缓冲区编号
```

### `---@type`

用于给变量、字段、返回值承载对象补充静态类型。

```lua
---@type WorkerConfig
local config = {
    name = "sync-worker",
    retry_limit = 3,
    poll_interval_ms = 1000,
}
```

### `---@alias`

用于定义别名类型、枚举风格联合类型。

```lua
---@alias LogLevel
---| '"debug"'
---| '"info"'
---| '"warn"'
---| '"error"'
```

### `---@generic`

用于泛型函数或泛型容器。

```lua
---@generic T
---@param value T
---@return T
local function identity(value)
    return value
end
```

### `---@cast`

用于类型收窄或显式断言，通常属于实现辅助，不属于对外文档主体；仅在确有必要时使用。

## 函数注释规范

* 第一行必须有摘要，简要说明函数做什么。
* 参数复杂时必须写 `---@param`。
* 返回值不直观时必须写 `---@return`。
* 若函数采用 `nil, err` / `false, err` 风格，必须把失败返回写清楚。
* 若参数为配置表，优先给配置表定义结构类型，而不是把所有信息都堆在一行里。

### 简单函数示例

```lua
--- 返回默认 API 地址。
---@return string endpoint 默认接口地址
local function default_endpoint()
    return "https://api.example.com"
end
```

### 带错误返回的函数示例

```lua
--- 解析原始用户 ID。
---@param raw string 原始用户 ID 字符串
---@return integer? user_id 解析成功后的用户 ID
---@return string? err 解析失败时的错误信息
local function parse_user_id(raw)
    if raw == "" or not raw:match("^%d+$") then
        return nil, "invalid user id"
    end

    return tonumber(raw), nil
end
```

## 配置表参数规范

当函数接收配置表时，推荐两种写法。

### 写法一：内联 table 类型

适合字段少、只在单个函数局部使用的情况。

```lua
--- LSP 格式化。
---@param opts? {timeout_ms?: number, format_options?: table, bufnr?: number}
local function format(opts)
end
```

这就是你给出的这种风格。

### 写法二：提取为 `---@class`

适合字段较多、会复用、需要更清晰提示的情况。

```lua
---@class LspFormatOpts
---@field timeout_ms? number 超时时间，单位毫秒
---@field format_options? table 传给 LSP 的格式化选项
---@field bufnr? number 目标缓冲区编号

--- LSP 格式化。
---@param opts? LspFormatOpts
local function format(opts)
end
```

更推荐第二种，因为：

* 可复用
* 编辑器提示更清晰
* 文档更稳定
* 字段多时可读性更好

## 返回 table 的规范

如果函数返回结构化 table，优先为返回值定义类型。

```lua
---@class UserInfo
---@field id integer 用户 ID
---@field name string 用户名

--- 读取用户信息。
---@param user_id integer 用户 ID
---@return UserInfo? user 用户信息
---@return string? err 失败时的错误信息
local function get_user(user_id)
end
```

不推荐长期大量使用这种匿名复杂返回：

```lua
---@return {id: integer, name: string}? user
```

小结构偶尔可以，大结构最好提取成 `---@class`。

## 方法注释规范

对象方法与普通函数相同。若使用冒号方法，通常不必显式写 `self` 参数，除非项目有特殊约定。

```lua
--- 启动工作进程。
---@return boolean ok 是否成功
---@return string? err 启动失败时的错误信息
function Worker:start()
    if self.running then
        return false, "already started"
    end

    self.running = true
    return true, nil
end
```

如果需要特别强调调用者类型，也可以补 `self` 类型信息，但一般不是必须。

## 类与对象类型规范

类风格对象、模块状态对象、配置对象，优先使用 `---@class` + `---@field`。

```lua
---@class Worker
---@field name string 工作进程名称
---@field running boolean 是否正在运行
local Worker = {}
Worker.__index = Worker
```

构造函数也应补注释：

```lua
--- 创建工作进程实例。
---@param name string 工作进程名称
---@return Worker worker 新建的工作进程实例
function Worker.new(name)
    return setmetatable({
        name = name,
        running = false,
    }, Worker)
end
```

## 模块级规范

Lua 没有专门的模块文档语法，通常在文件顶部使用连续 `---` 摘要说明模块职责。

```lua
--- 提供 LSP 格式化、代码操作与诊断相关的辅助函数。
local M = {}
```

如果模块导出很多结构化对象，建议配合 `---@class`、`---@field` 为关键导出类型补充说明。

## 可选参数规范

可选参数统一使用 `?`：

```lua
---@param bufnr? integer 目标缓冲区，默认当前缓冲区
```

可选字段也使用 `?`：

```lua
---@class FormatOpts
---@field bufnr? integer
---@field timeout_ms? number
```

## 联合类型与字面量类型规范

对有限取值集合，优先用别名或字面量联合。

```lua
---@alias NotifyLevel
---| '"trace"'
---| '"debug"'
---| '"info"'
---| '"warn"'
---| '"error"'

--- 输出通知。
---@param level NotifyLevel 通知级别
---@param msg string 消息内容
local function notify(level, msg)
end
```

## 泛型规范

对容器工具函数、映射函数、过滤函数，优先使用 `---@generic`。

```lua
---@generic T
---@param list T[]
---@param predicate fun(item: T): boolean
---@return T[]
local function filter(list, predicate)
    local out = {}
    for _, item in ipairs(list) do
        if predicate(item) then
            out[#out + 1] = item
        end
    end
    return out
end
```

## 风格与边界

* 摘要应直接说明用途，不要只重复函数名。
* 参数、返回值、字段的描述应关注语义、约束、单位、失败条件。
* 公共 API 必须补文档；私有实现按需要补充。
* 同一仓库内统一风格：

  * 是否总给返回值命名
  * 配置表是内联还是 `---@class`
  * 整数写 `integer` 还是 `number`
  * 数组写 `T[]` 还是 `table<integer, T>`
* 字段较多的配置表、返回值、状态对象，优先提取成 `---@class`，不要长期堆匿名 table 类型。
* 不要写与调用者无关的实现细节。

## 针对你这种写法的推荐规则

如果你就是要统一成这种风格：

```lua
--- LSP 格式化
---@param opts? {timeout_ms?: number, format_options?: table, bufnr?: number}
```

那我建议补成下面这套更明确的约定：

### 对简单配置表参数

字段少时允许内联：

```lua
--- LSP 格式化。
---@param opts? {timeout_ms?: number, format_options?: table, bufnr?: number} 格式化选项
```

### 对复杂配置表参数

字段多时必须提取：

```lua
---@class LspFormatOpts
---@field timeout_ms? number 格式化超时时间，单位毫秒
---@field format_options? table 传给 LSP 的格式化参数
---@field bufnr? number 目标缓冲区编号

--- LSP 格式化。
---@param opts? LspFormatOpts 格式化选项
```

### 对返回值

尽量写完整：

```lua
--- 执行格式化。
---@param opts? LspFormatOpts 格式化选项
---@return boolean ok 是否成功发起格式化
---@return string? err 失败原因
```
