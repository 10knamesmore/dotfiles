-- 手动格式化工具：选择已注册的 formatter，由 `=` 键调用。
local M = {}

---@class Formatter
---@field name string
---@field primary? boolean
---@field format fun(bufnr:number)
---@field sources fun(bufnr:number):string[]
---@field priority number

M.formatters = {} ---@type Formatter[]

--- 注册一个格式化器，并按优先级降序排序。
---@param formatter Formatter
function M.register(formatter)
  M.formatters[#M.formatters + 1] = formatter
  table.sort(M.formatters, function(a, b)
    return a.priority > b.priority
  end)
end

--- 返回当前缓冲区使用的 `formatexpr` 实现。
---@return string|function
function M.formatexpr()
  local Util = require("utils")
  if Util.has("conform.nvim") then
    return require("conform").formatexpr()
  end
  return vim.lsp.formatexpr({ timeout_ms = 3000 })
end

--- 解析当前缓冲区可用的格式化器，并标记当前激活项。
---@param buf? number
---@return (Formatter|{active:boolean,resolved:string[]})[]
function M.resolve(buf)
  buf = buf or vim.api.nvim_get_current_buf()
  local have_primary = false
  ---@param formatter Formatter
  return vim.tbl_map(function(formatter)
    local sources = formatter.sources(buf)
    local active = #sources > 0 and (not formatter.primary or not have_primary)
    have_primary = have_primary or (active and formatter.primary) or false
    return setmetatable({
      active = active,
      resolved = sources,
    }, { __index = formatter })
  end, M.formatters)
end

--- 格式化当前缓冲区；没有可用 formatter 时提示用户。
function M.format()
  local buf = vim.api.nvim_get_current_buf()
  local Util = require("utils")
  local done = false
  for _, formatter in ipairs(M.resolve(buf)) do
    if formatter.active then
      done = true
      Util.try(function()
        return formatter.format(buf)
      end, { msg = "Formatter `" .. formatter.name .. "` failed" })
    end
  end

  if not done then
    Util.warn("No formatter available", { title = "Format" })
  end
end

return M
