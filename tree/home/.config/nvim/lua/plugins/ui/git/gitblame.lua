return {
  "f-person/git-blame.nvim",
  -- BufReadPost 而非 VeryLazy：它对每个 buffer 起独立 git 子进程，
  -- dashboard / 空 buffer 场景没有 blame 可算，没必要跟着启动一起拉起来。
  event = "BufReadPost",
  ---@module "gitblame"
  ---@type SetupOptions
  opts = {
    -- your configuration comes here
    -- for example
    enabled = true, -- if you want to enable the plugin
    message_template = " <summary> • <date> • <author> • <<sha>>", -- template for the blame message, check the Message template section for more options
    date_format = "%m-%d-%Y %H:%M:%S", -- template for the date, check Date format section for more options
    virtual_text_column = 1, -- virtual text start column, check Start virtual text at column section for more options
    delay = 0, -- delay in milliseconds
  },
}
