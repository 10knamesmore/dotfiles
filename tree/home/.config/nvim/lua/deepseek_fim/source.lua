-- DeepSeek FIM 作为 blink.cmp 的一个补全 source：
-- 取光标前后文 -> 异步 curl 打 FIM -> 把 choices[0].text 包成一个补全项回调给 blink。
-- 骨架参照 giuxtaposition/blink-cmp-copilot，把它的 LSP client 换成 curl。
---@module 'blink.cmp'
local api = require("deepseek_fim.api")
local ctxmod = require("deepseek_fim.context")

--- @class deepseek_fim.Source : blink.cmp.Source
local M = {}

function M.new()
  return setmetatable({}, { __index = M })
end

function M:enabled()
  local core = require("deepseek_fim")
  if not core.config.enabled then
    return false
  end
  local key = vim.env[core.config.api_key_env]
  if not key or key == "" then
    return false
  end
  local fts = core.config.filetypes
  if fts then
    return vim.tbl_contains(fts, vim.bo.filetype)
  end
  return true
end

-- 把补全文本包成 blink CompletionItem。
-- textEdit 为纯插入(range start==end==光标)，接受时把整段(可能多行)插到光标处。
--- @param text string
--- @param context blink.cmp.Context
--- @return blink.cmp.CompletionItem[]
local function build_items(text, context)
  local row0 = context.cursor[1] - 1 -- LSP range 用 0-indexed 行
  local col0 = context.cursor[2] -- 0-indexed 列
  local first_nl = text:find("\n", 1, true)
  local first_line = first_nl and text:sub(1, first_nl - 1) or text
  local label = vim.trim(first_line)
  if label == "" then
    label = vim.trim(text)
  end
  return {
    {
      label = label ~= "" and label or "DeepSeek",
      kind = vim.lsp.protocol.CompletionItemKind.Text,
      kind_name = "DeepSeek",
      kind_icon = "󰚩",
      kind_hl = "BlinkCmpKindDeepSeek", -- 覆盖默认 kind 高亮为 DeepSeek 主题色(见 init.lua)
      textEdit = {
        newText = text,
        range = {
          start = { line = row0, character = col0 },
          ["end"] = { line = row0, character = col0 },
        },
      },
      documentation = {
        kind = "markdown",
        value = string.format("```%s\n%s\n```", vim.bo.filetype, text),
      },
    },
  }
end

local function empty(callback, incomplete)
  callback({
    is_incomplete_forward = incomplete or false,
    is_incomplete_backward = incomplete or false,
    items = {},
  })
end

function M:get_completions(context, callback)
  local core = require("deepseek_fim")
  local cfg = core.config
  local cursor = context.cursor
  local line = context.line or ""
  local col = cursor[2]
  core.log("get_completions line=[" .. line .. "] col=" .. col)

  -- 触发门槛：只跳过纯空白行(无上下文可补)。
  -- 注意：光标后有内容是正常的——那正是 FIM 的 suffix，不要因此跳过，
  -- 否则在自动配对的闭合符(" ) } 等)前打字将永远不触发补全。
  if line:match("^%s*$") then
    core.log("  skip: blank line")
    return empty(callback)
  end

  local prompt, suffix = ctxmod.collect(context.bufnr, cursor, cfg)
  local cache_key = prompt .. "\0" .. suffix

  -- 缓存命中：直接返回，零 API 调用。
  local cached = core.cache_get(cache_key)
  if cached ~= nil then
    core.log("  cache hit")
    return callback({
      is_incomplete_forward = false,
      is_incomplete_backward = false,
      items = build_items(cached, context),
    })
  end

  local cancelled = false
  local cancel_curl

  -- 防抖：停顿 debounce_ms 后才真正发请求；期间若上下文变化，
  -- blink 会调用下面返回的 cancel 函数，stop 掉这个 timer。
  local timer = vim.defer_fn(function()
    if cancelled then
      return
    end
    core.log("  -> request prompt_len=" .. #prompt .. " suffix_len=" .. #suffix)
    cancel_curl = api.request(cfg, prompt, suffix, function(text, err, usage)
      -- api 回调可能在 fast-event，操作 vim.fn/blink/buffer 前切回主循环。
      vim.schedule(function()
        -- 用量统计不受取消影响：只要 API 返回就已计费，先记账再判断是否丢弃结果。
        require("deepseek_fim.stats").record(usage)
        if cancelled then
          core.log("  <- result dropped (cancelled)")
          return
        end
        if err or not text then
          core.log("  <- error: " .. tostring(err))
          return empty(callback, true)
        end
        core.log("  <- ok text=[" .. text:gsub("\n", "\\n") .. "]")
        core.cache_set(cache_key, text)
        callback({
          is_incomplete_forward = false,
          is_incomplete_backward = false,
          items = build_items(text, context),
        })
      end)
    end)
  end, cfg.debounce_ms)

  -- blink 在上下文变化(继续打字/移动)时调用此函数取消在途请求。
  return function()
    cancelled = true
    pcall(function()
      timer:stop()
    end)
    if cancel_curl then
      cancel_curl()
    end
  end
end

return M
