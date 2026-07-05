-- 持久化累计 DeepSeek FIM 的 token 用量(跨 nvim 会话)，供 :DeepseekUsage 查看。
-- 存到 stdpath('data')/deepseek_fim_usage.json，每次请求成功后累加落盘。
local M = {}

local function path()
  return vim.fn.stdpath("data") .. "/deepseek_fim_usage.json"
end

local function default()
  return {
    since = os.date("%Y-%m-%d %H:%M"), -- 统计起始时刻
    requests = 0,
    prompt_tokens = 0, -- 输入总量(= 命中 + 未命中)
    completion_tokens = 0, -- 输出总量
    cache_hit_tokens = 0, -- 输入·缓存命中
    cache_miss_tokens = 0, -- 输入·缓存未命中
  }
end

local data = nil -- 内存缓存，懒加载

function M.load()
  if data then
    return data
  end
  local f = io.open(path(), "r")
  if f then
    local content = f:read("*a")
    f:close()
    local ok, decoded = pcall(vim.json.decode, content)
    if ok and type(decoded) == "table" then
      data = vim.tbl_extend("keep", decoded, default()) -- 补齐新增字段
      return data
    end
  end
  data = default()
  return data
end

function M.save()
  if not data then
    return
  end
  local f = io.open(path(), "w")
  if f then
    f:write(vim.json.encode(data))
    f:close()
  end
end

--- 累加一次请求的用量并落盘。
--- @param usage table|nil DeepSeek 响应里的 usage 表
function M.record(usage)
  if type(usage) ~= "table" then
    return
  end
  local d = M.load()
  d.requests = d.requests + 1
  d.prompt_tokens = d.prompt_tokens + (usage.prompt_tokens or 0)
  d.completion_tokens = d.completion_tokens + (usage.completion_tokens or 0)
  d.cache_hit_tokens = d.cache_hit_tokens + (usage.prompt_cache_hit_tokens or 0)
  d.cache_miss_tokens = d.cache_miss_tokens + (usage.prompt_cache_miss_tokens or 0)
  M.save()
end

--- 清零统计。
function M.reset()
  data = default()
  M.save()
end

--- 生成人类可读报告。
--- @param pricing table config.pricing，元/百万 token
--- @return string
function M.report(pricing)
  local d = M.load()
  -- 费用 = 命中输入 + 未命中输入 + 输出，各按单价，token/百万 * 单价。
  local cost = (
    d.cache_hit_tokens * pricing.cache_hit
    + d.cache_miss_tokens * pricing.cache_miss
    + d.completion_tokens * pricing.output
  ) / 1e6
  return table.concat({
    "󰚩 DeepSeek FIM 用量 (自 " .. d.since .. ")",
    ("  请求次数   : %d"):format(d.requests),
    ("  输入 tokens: %d  (缓存命中 %d / 未命中 %d)"):format(
      d.prompt_tokens,
      d.cache_hit_tokens,
      d.cache_miss_tokens
    ),
    ("  输出 tokens: %d"):format(d.completion_tokens),
    ("  估算费用   : ￥%.4f"):format(cost),
  }, "\n")
end

return M
