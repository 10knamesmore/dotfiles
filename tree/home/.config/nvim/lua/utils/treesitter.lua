-- Treesitter 工具模块
-- 提供 treesitter 相关的辅助功能

---@class util.treesitter
local M = {}

M._installed = nil ---@type table<string,boolean>?
M._queries = {} ---@type table<string,boolean>

--- 获取已安装的 Treesitter parser 列表。
---@param update boolean?
---@return table<string, boolean>
function M.get_installed(update)
  if update then
    M._installed, M._queries = {}, {}
    local ok, TS = pcall(require, "nvim-treesitter")
    if ok and TS.get_installed then
      for _, lang in ipairs(TS.get_installed("parsers")) do
        M._installed[lang] = true
      end
    end
  end
  return M._installed or {}
end

--- 检查某语言是否存在指定 query，并缓存结果。
---@param lang string
---@param query string
---@return boolean
function M.have_query(lang, query)
  local key = lang .. ":" .. query
  if M._queries[key] == nil then
    M._queries[key] = vim.treesitter.query.get(lang, query) ~= nil
  end
  return M._queries[key]
end

--- 检查是否有特定的 parser 和 query
---@param what string|number|nil  buffer number 或 filetype
---@param query? string
---@return boolean
function M.have(what, query)
  what = what or vim.api.nvim_get_current_buf()
  what = type(what) == "number" and vim.bo[what].filetype or what --[[@as string]]
  local lang = vim.treesitter.language.get_lang(what)
  if lang == nil or M.get_installed()[lang] == nil then
    return false
  end
  if query and not M.have_query(lang, query) then
    return false
  end
  return true
end

--- 返回适用于 `foldexpr` 的 Treesitter 表达式。
---@return string
function M.foldexpr()
  return M.have(nil, "folds") and vim.treesitter.foldexpr() or "0"
end

--- 返回适用于 `indentexpr` 的 Treesitter 表达式。
---@return integer
function M.indentexpr()
  local ok, TS = pcall(require, "nvim-treesitter")
  return ok and M.have(nil, "indents") and TS.indentexpr() or -1
end

--- 在 Windows 环境下查找可用的 `cl.exe`。
---@return string?
local function win_find_cl()
  local path = "C:/Program Files (x86)/Microsoft Visual Studio"
  local pattern = "*/*/VC/Tools/MSVC/*/bin/Hostx64/x64/cl.exe"
  return vim.fn.globpath(path, pattern, true, true)[1]
end

--- 检查 Treesitter 运行依赖是否齐全。
---@return boolean ok, table<string,boolean> health
function M.check()
  local is_win = vim.fn.has("win32") == 1
  ---@param tool string
  ---@param win boolean?
  local function have(tool, win)
    return (win == nil or is_win == win) and vim.fn.executable(tool) == 1
  end

  local have_cc = vim.env.CC ~= nil or have("cc", false) or have("cl", true) or (is_win and win_find_cl() ~= nil)

  if not have_cc and is_win and vim.fn.executable("gcc") == 1 then
    vim.env.CC = "gcc"
    have_cc = true
  end

  ---@type table<string,boolean>
  local ret = {
    ["tree-sitter (CLI)"] = have("tree-sitter"),
    ["C compiler"] = have_cc,
    tar = have("tar"),
    curl = have("curl"),
  }
  local ok = true
  for _, v in pairs(ret) do
    ok = ok and v
  end
  return ok, ret
end

--- 检查依赖后执行 Treesitter 构建回调。
---@param cb fun()
function M.build(cb)
  M.ensure_treesitter_cli(function(ok, err)
    local health_ok, health = M.check()
    if health_ok then
      return cb()
    else
      local Util = require("utils")
      local lines = { "Unmet requirements for **nvim-treesitter** `main`:" }
      local keys = vim.tbl_keys(health) ---@type string[]
      table.sort(keys)
      for _, k in pairs(keys) do
        lines[#lines + 1] = ("- %s `%s`"):format(health[k] and "✅" or "❌", k)
      end
      vim.list_extend(lines, {
        "",
        "See: [nvim-treesitter requirements](https://github.com/nvim-treesitter/nvim-treesitter/tree/main?tab=readme-ov-file#requirements)",
        "Run `:checkhealth nvim-treesitter` for more information.",
      })
      if vim.fn.has("win32") == 1 and not health["C compiler"] then
        lines[#lines + 1] = "Install a C compiler with `winget install --id=BrechtSanders.WinLibs.POSIX.UCRT -e`"
      end
      vim.list_extend(lines, err and { "", err } or {})
      Util.error(lines, { title = "Treesitter" })
    end
  end)
end

--- 确保 `tree-sitter` CLI 可用，必要时尝试通过 Mason 安装。
---@param cb fun(ok:boolean, err?:string)
function M.ensure_treesitter_cli(cb)
  if vim.fn.executable("tree-sitter") == 1 then
    return cb(true)
  end

  -- 尝试用 mason 安装
  if not pcall(require, "mason") then
    return cb(false, "`mason.nvim` is disabled, cannot install tree-sitter CLI automatically.")
  end

  -- 再次检查（可能已经安装了）
  if vim.fn.executable("tree-sitter") == 1 then
    return cb(true)
  end

  local mr = require("mason-registry")
  mr.refresh(function()
    local p = mr.get_package("tree-sitter-cli")
    if not p:is_installed() then
      local Util = require("utils")
      Util.info("Installing `tree-sitter-cli` with `mason.nvim`...")
      p:install(
        nil,
        vim.schedule_wrap(function(success)
          if success then
            Util.info("Installed `tree-sitter-cli` with `mason.nvim`.")
            cb(true)
          else
            cb(false, "Failed to install `tree-sitter-cli` with `mason.nvim`.")
          end
        end)
      )
    end
  end)
end

return M
