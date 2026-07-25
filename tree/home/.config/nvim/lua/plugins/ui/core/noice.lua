-- noice.nvim - UI 增强（命令行、通知等）
return {
  {
    "folke/noice.nvim",
    event = "VeryLazy",
    dependencies = {
      "MunifTanjim/nui.nvim",
      "rcarriga/nvim-notify",
    },
    -- enabled = false,
    ---@type NoiceConfig
    opts = {
      messages = {
        enabled = true,
        view = "notify",
        view_error = "notify",
        view_warn = "notify",
      },
      notify = {
        enabled = true,
        view = "notify",
      },
      lsp = {
        override = {
          ["vim.lsp.util.convert_input_to_markdown_lines"] = true,
          ["vim.lsp.util.stylize_markdown"] = true,
        },
      },
      routes = {},
      presets = {
        bottom_search = true,
        command_palette = true,
        long_message_to_split = true,
      },
      commands = {
        all = {
          view = "popup",
          opts = { enter = true, format = "details" },
          filter = {},
        },
      },
    },
    keys = {
      {
        "<S-Enter>",
        function()
          require("noice").redirect(vim.fn.getcmdline())
        end,
        mode = "c",
        desc = "Redirect Cmdline",
      },
      {
        "<leader>sn",
        function()
          require("noice").cmd("telescope")
        end,
        desc = "Noice All",
      },
      {
        "<leader>n",
        function()
          require("noice").cmd("all")
        end,
        desc = "Noice All",
      },
      {
        "<c-f>",
        function()
          if not require("noice.lsp").scroll(4) then
            return "<c-f>"
          end
        end,
        silent = true,
        expr = true,
        desc = "Scroll Forward",
        mode = { "i", "n", "s" },
      },
      {
        "<c-b>",
        function()
          if not require("noice.lsp").scroll(-4) then
            return "<c-b>"
          end
        end,
        silent = true,
        expr = true,
        desc = "Scroll Backward",
        mode = { "i", "n", "s" },
      },
    },
    config = function(_, opts)
      -- HACK: noice shows messages from before it was enabled,
      -- but this is not ideal when Lazy is installing plugins,
      -- so clear the messages in this case.
      if vim.o.filetype == "lazy" then
        vim.cmd([[messages clear]])
      end
      require("noice").setup(opts)
    end,
  },
  {
    "rcarriga/nvim-notify",
    ---@module "notify"
    -- notify.Config 的 merge_duplicates 是必填字段，而这里是交给 lazy 深合并的
    -- 部分配置，缺字段属于设计如此。注意 disable-next-line 必须紧贴 opts 那行
    -- （诊断报在表上，不是报在 ---@type 上）——摆法同 bqf.lua:5-8。
    ---@type notify.Config
    ---@diagnostic disable-next-line: missing-fields
    opts = {
      fps = 60,
      stages = "slide",
      timeout = 4000,
      top_down = true,
    },
  },
}
