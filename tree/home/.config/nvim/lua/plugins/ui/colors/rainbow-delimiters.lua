return {
  "HiPhish/rainbow-delimiters.nvim",
  -- 必须有触发器：defaults.lazy=true 下没有 event/ft/cmd/keys 又没人 require 的插件
  -- 永远不加载，彩虹括号从来不会生效。
  event = { "BufReadPost", "BufNewFile" },
  main = "rainbow-delimiters.setup",
  submodules = false,
  ---@module "rainbow-delimiters"
  ---@type rainbow_delimiters.config
  opts = {},
}
