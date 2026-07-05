-- DeepSeek FIM 补全的默认配置。
-- FIM(fill-in-the-middle)接口：给光标前代码当 prompt、光标后代码当 suffix，
-- 模型直接返回中间该补的文本，无需 JSON 解析。见 api.lua。
local M = {}

---@class deepseek_fim.Config
M.defaults = {
  enabled = true,

  -- 认证与端点
  api_key_env = "DEEPSEEK_API_KEY", -- 从此环境变量读 key，缺失则整个 source 禁用
  base_url = "https://api.deepseek.com/beta", -- FIM 走 beta 端点
  -- FIM 仅非思考模式支持。v4-flash 快且便宜，适合 inline 高频补全；
  -- 想要更强补全质量可改 "deepseek-v4-pro"。
  -- (旧名 deepseek-chat 于 2026/07/24 弃用，勿再用)
  model = "deepseek-v4-flash",

  -- 生成参数
  max_tokens = 256, -- 单次补全上限，防止一次续写整个文件、也压成本
  temperature = 0.2, -- 代码补全用低温更确定
  stop = nil, -- 停止序列；nil=靠 suffix + max_tokens 自然收尾

  -- 上下文窗口（截断防 token 爆 + 降延迟）
  prefix_lines = 100, -- 光标前最多取多少行进 prompt
  suffix_lines = 50, -- 光标后最多取多少行进 suffix

  -- 成本护栏（blink 每敲一个字符都会来要补全，必须自己挡）
  debounce_ms = 400, -- 防抖：停顿这么久才真正发请求(连续打字不发，停手才发一次)
  timeout_ms = 5000, -- 单次 curl 超时

  -- 文件类型白名单；nil = 所有类型都启用
  filetypes = nil,

  -- 计费单价(元/百万 token)，:DeepseekUsage 用来估算费用。
  -- 截至 2026-07 官方 deepseek-v4-flash 价；会变动，切模型/涨价请在此更新。
  pricing = {
    cache_hit = 0.02, -- 输入·缓存命中
    cache_miss = 1.0, -- 输入·缓存未命中
    output = 2.0, -- 输出
  },

  -- 调试：:DeepseekDebug 开关，日志写到 log_file(可 tail)
  debug = false,
  log_file = vim.fn.stdpath("cache") .. "/deepseek_fim.log",
}

return M
