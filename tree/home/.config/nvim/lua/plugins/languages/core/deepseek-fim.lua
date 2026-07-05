-- DeepSeek FIM inline 补全：作为 blink.cmp 的一个 source 注入补全菜单。
-- 逻辑代码在 lua/deepseek_fim/(rtp 天然可 require)，本 spec 只负责：
--   1. 调 setup(注册 :DeepseekToggle、初始化缓存/节流状态)
--   2. 把 deepseek provider 挂到 blink 的 sources
-- 用 opts 函数追加，不动 languages/core/completion.lua 的主 blink 配置。
return {
  "saghen/blink.cmp",
  optional = true,
  -- setup 放 init：启动即注册 :DeepseekToggle、初始化缓存/节流状态。
  -- 若放 opts，会随 blink 的 InsertEnter 懒加载才执行，导致进插入模式前命令不存在。
  init = function()
    require("deepseek_fim").setup({
      -- 覆盖默认见 lua/deepseek_fim/config.lua，例如：
      -- model = "deepseek-v4-pro",
      -- filetypes = { "lua", "python", "rust" },
    })
  end,
  opts = function(_, opts)
    opts.sources = opts.sources or {}
    opts.sources.default = opts.sources.default or {}
    if not vim.tbl_contains(opts.sources.default, "deepseek") then
      table.insert(opts.sources.default, "deepseek")
    end

    opts.sources.providers = opts.sources.providers or {}
    opts.sources.providers.deepseek = {
      module = "deepseek_fim.source",
      name = "DeepSeek",
      async = true, -- 不阻塞菜单：先出 LSP，FIM 到了再补进来
      timeout_ms = 5000,
      min_keyword_length = 0, -- FIM 纯插入、无 keyword 也要能触发
      score_offset = 100, -- 优先级拉满：碾压 LSP(=3) 等，DeepSeek 项永远置顶
    }
  end,
}
