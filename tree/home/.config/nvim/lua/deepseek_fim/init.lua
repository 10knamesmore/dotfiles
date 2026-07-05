-- DeepSeek FIM 补全的入口：持有运行时配置、去重缓存，注册用户命令。
-- 真正的补全逻辑在 source.lua(blink source)，本模块提供其依赖的共享状态。
local config = require("deepseek_fim.config")

local M = {}
M.config = vim.deepcopy(config.defaults)

-- 去重缓存：相同 (prompt+suffix) 直接复用结果，挡住 blink 因每次按键重复触发导致的多余 API 调用。
-- 简单 LRU：cache 存 key->text，cache_order 记插入顺序，超上限淘汰最旧。
local cache = {}
local cache_order = {}
local CACHE_MAX = 64

--- @param key string
--- @return string|nil
function M.cache_get(key)
  local e = cache[key]
  return e and e.text or nil
end

--- @param key string
--- @param text string
function M.cache_set(key, text)
  if cache[key] == nil then
    cache_order[#cache_order + 1] = key
    if #cache_order > CACHE_MAX then
      local old = table.remove(cache_order, 1)
      cache[old] = nil
    end
  end
  cache[key] = { text = text }
end

--- 调试日志：debug 开时追加一行到 config.log_file。
function M.log(...)
  if not M.config.debug then
    return
  end
  local parts = {}
  for _, v in ipairs({ ... }) do
    parts[#parts + 1] = type(v) == "string" and v or vim.inspect(v)
  end
  local f = io.open(M.config.log_file, "a")
  if f then
    f:write(os.date("%H:%M:%S ") .. table.concat(parts, " ") .. "\n")
    f:close()
  end
end

--- @param opts deepseek_fim.Config|nil
function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})

  -- DeepSeek 品牌蓝：菜单里 DeepSeek 补全项的 kind 图标/文本高亮。
  -- 挂 ColorScheme 上重设，避免切主题被覆盖。
  local function set_hl()
    vim.api.nvim_set_hl(0, "BlinkCmpKindDeepSeek", { fg = "#4D6BFE", bold = true })
  end
  set_hl()
  vim.api.nvim_create_autocmd("ColorScheme", { callback = set_hl })

  vim.api.nvim_create_user_command("DeepseekToggle", function()
    M.config.enabled = not M.config.enabled
    vim.notify("DeepSeek FIM: " .. (M.config.enabled and "on" or "off"))
  end, { desc = "开关 DeepSeek FIM 补全" })

  vim.api.nvim_create_user_command("DeepseekDebug", function()
    M.config.debug = not M.config.debug
    vim.notify("DeepSeek FIM debug: " .. (M.config.debug and ("on → " .. M.config.log_file) or "off"))
  end, { desc = "开关 DeepSeek FIM 调试日志" })

  vim.api.nvim_create_user_command("DeepseekUsage", function()
    vim.notify(require("deepseek_fim.stats").report(M.config.pricing), vim.log.levels.INFO)
  end, { desc = "查看 DeepSeek FIM 累计用量与估算费用" })

  vim.api.nvim_create_user_command("DeepseekUsageReset", function()
    require("deepseek_fim.stats").reset()
    vim.notify("DeepSeek FIM 用量已清零")
  end, { desc = "清零 DeepSeek FIM 用量统计" })

  -- key 缺失时一次性提示(source 层也会 enabled=false 保证不发请求)。
  local key = vim.env[M.config.api_key_env]
  if not key or key == "" then
    vim.schedule(function()
      vim.notify(
        "DeepSeek FIM 已禁用：环境变量 " .. M.config.api_key_env .. " 未设置",
        vim.log.levels.WARN
      )
    end)
  end
end

return M
