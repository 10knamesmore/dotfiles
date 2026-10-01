return {
  "shortcuts/no-neck-pain.nvim",
  version = "*",
  lazy = false,
  opts = {
    -- 限制整个编辑窗口的宽度，行号和 sign column 也在这 160 列之内。
    width = 160,
    autocmds = {
      -- 等 dashboard 或启动命令完成后再居中；新标签页也保持居中。
      enableOnVimEnter = "safe",
      enableOnTabEnter = true,
    },
    integrations = {
      dashboard = { enabled = true },
    },
  },
  keys = {
    { "<leader>uC", "<cmd>NoNeckPain<cr>", desc = "Toggle centered editor" },
  },
}
