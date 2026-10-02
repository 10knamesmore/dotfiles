local width_ratio = 0.8

return {
  "shortcuts/no-neck-pain.nvim",
  version = "*",
  lazy = false,
  opts = {
    -- 编辑区域占总宽度的比例，包含行号和 sign column。
    width = math.floor(vim.o.columns * width_ratio),
    autocmds = {
      -- 等 dashboard 或启动命令完成后再居中；新标签页也保持居中。
      enableOnVimEnter = "safe",
      enableOnTabEnter = true,
    },
    integrations = {
      dashboard = { enabled = true },
    },
  },
  config = function(_, opts)
    local no_neck_pain = require("no-neck-pain")
    no_neck_pain.setup(opts)

    vim.api.nvim_create_autocmd("VimResized", {
      group = vim.api.nvim_create_augroup("CenteredEditorWidth", { clear = true }),
      callback = function()
        -- 同步 setup 参数，避免切换配色时恢复旧宽度。
        opts.width = math.floor(vim.o.columns * width_ratio)
        if no_neck_pain.state and no_neck_pain.state.enabled then
          no_neck_pain.resize(opts.width)
        else
          no_neck_pain.config.width = opts.width
        end
      end,
    })
  end,
  keys = {
    { "<leader>uC", "<cmd>NoNeckPain<cr>", desc = "Toggle centered editor" },
  },
}
