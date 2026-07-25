-- 工具函数模块

local M = {}

-- 延迟加载所有子模块
setmetatable(M, {
  __index = function(t, k)
    -- 尝试加载自定义工具模块
    local ok, module = pcall(require, "utils." .. k)
    if ok then
      rawset(t, k, module)
      return module
    end
    -- 回退到 lazy.nvim 的工具函数（如果存在）
    local ok2, LazyUtil = pcall(require, "lazy.core.util")
    if ok2 and LazyUtil[k] then
      return LazyUtil[k]
    end
  end,
})

--- 获取指定插件的 `lazy.nvim` 规格。
---@param name string
---@return LazyPluginSpec|nil
function M.get_plugin(name)
  return require("lazy.core.config").spec.plugins[name]
end

--- 检查指定插件是否已注册到 `lazy.nvim`。
---@param plugin string
---@return boolean
function M.has(plugin)
  return M.get_plugin(plugin) ~= nil
end

--- 在 `VeryLazy` 事件触发后执行回调。
---@param fn fun()
function M.on_very_lazy(fn)
  vim.api.nvim_create_autocmd("User", {
    pattern = "VeryLazy",
    callback = function()
      fn()
    end,
  })
end

--- 读取指定插件解析后的 `opts` 配置。
---@param name string
---@return table
function M.opts(name)
  local plugin = M.get_plugin(name)
  if not plugin then
    return {}
  end
  local Plugin = require("lazy.core.plugin")
  return Plugin.values(plugin, "opts", false)
end

--- 通知函数（带默认标题）。动态生成的三个函数拿不到 LuaLS 补全，
--- 这里补上签名——它们是全仓用得最多的 utils 成员，而 utils 的惰性加载
--- 对拼错的名字只会静默返回 nil，类型标注是第一道防线。
---@class utils
---@field info fun(msg: string|string[], opts?: table)
---@field warn fun(msg: string|string[], opts?: table)
---@field error fun(msg: string|string[], opts?: table)
---@field format table
---@field lsp table
---@field lualine table
---@field math table
---@field os table
---@field path table
---@field treesitter table
for _, level in ipairs({ "info", "warn", "error" }) do
  M[level] = function(msg, opts)
    opts = opts or {}
    opts.title = opts.title or "Nvim"
    local LazyUtil = require("lazy.core.util")
    return LazyUtil[level](msg, opts)
  end
end

--- 规范化路径分隔符并展开 `~`。
---@param path string
---@return string
function M.norm(path)
  if path:sub(1, 1) == "~" then
    local home = vim.uv.os_homedir()
    if home and (home:sub(-1) == "\\" or home:sub(-1) == "/") then
      home = home:sub(1, -2)
    end
    path = home .. path:sub(2)
  end
  path = path:gsub("\\", "/"):gsub("/+", "/")
  return path:sub(-1) == "/" and path:sub(1, -2) or path
end

--- 安全执行函数，并在出错时统一上报。
---@param fn function
---@param opts? {msg?:string, on_error?:fun(msg)}
---@return boolean
function M.try(fn, opts)
  opts = opts or {}
  local ok, err = pcall(fn)
  if not ok then
    local msg = opts.msg and (opts.msg .. "\n\n" .. err) or err
    if opts.on_error then
      opts.on_error(msg)
    else
      M.error(msg)
    end
  end
  return ok
end

--- 在本地选项尚未被显式设置过时写入默认值。
---
--- 判据是「有没有被设过」（`nvim_get_option_info2().was_set`），不是「值是不是空串」。
--- 旧实现比较空串，而 'foldmethod' 的取值域是 manual/indent/expr/marker/… 永远非空，
--- 于是 treesitter 和 LSP 的折叠分支全成了死代码。'indentexpr' 那侧行为不变：
--- 有 ftplugin 的语言（lua 的 GetLuaIndent()）was_set 为真，照旧让位给 ftplugin。
---@param option string
---@param value any
---@return boolean 是否实际写入
function M.set_default(option, value)
  local ok, info = pcall(vim.api.nvim_get_option_info2, option, {})
  if not ok or info.was_set then
    return false
  end
  vim.api.nvim_set_option_value(option, value, { scope = "local" })
  return true
end

-- 原有的 icons
M.icons = {
  misc = {
    dots = "󰇘",
  },
  ft = {
    octo = " ",
    gh = " ",
    ["markdown.gh"] = " ",
  },
  dap = {
    Stopped = { "󰁕 ", "DiagnosticWarn", "DapStoppedLine" },
    Breakpoint = " ",
    BreakpointCondition = " ",
    BreakpointRejected = { " ", "DiagnosticError" },
    LogPoint = ".>",
  },
  diagnostics = {
    Error = " ",
    Warn = " ",
    Hint = " ",
    Info = " ",
  },
  git = {
    added = " ",
    modified = " ",
    removed = " ",
  },
  kinds = {
    Array = " ",
    Boolean = "󰨙 ",
    Class = " ",
    Codeium = "󰘦 ",
    Color = " ",
    Control = " ",
    Collapsed = " ",
    Constant = "󰏿 ",
    Constructor = " ",
    Copilot = " ",
    Enum = " ",
    EnumMember = " ",
    Event = " ",
    Field = " ",
    File = " ",
    Folder = " ",
    Function = "󰊕 ",
    Interface = " ",
    Key = " ",
    Keyword = " ",
    Method = "󰊕 ",
    Module = " ",
    Namespace = "󰦮 ",
    Null = " ",
    Number = "󰎠 ",
    Object = " ",
    Operator = " ",
    Package = " ",
    Property = " ",
    Reference = " ",
    Snippet = "󱄽 ",
    String = " ",
    Struct = "󰆼 ",
    Supermaven = " ",
    TabNine = "󰏚 ",
    Text = " ",
    TypeParameter = " ",
    Unit = " ",
    Value = " ",
    Variable = "󰀫 ",
  },
}

return M
