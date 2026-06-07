return {
  "folke/which-key.nvim",
  event = "VeryLazy",
  opts_extend = { "spec" },
  opts = function()
    return {
      preset = "helix",
      defaults = {},
      -- 禁用 which-key 的场景：它的 g trigger（裸 g 的 buffer-local nowait 映射）
      -- 接管按键转发，树里查不到的序列会被静默丢弃（实测吞 minifiles 的 gY）。
      -- 仅靠 ft 名单有时序竞态：mini.files 先建 buffer 后设 filetype，wk 可能在
      -- ft 为空时就挂上 trigger。buftype=nofile 在 buffer 创建瞬间即生效（无竞态），
      -- 由它兜底拦截所有 scratch buffer（mini.files / 各类插件浮窗 UI）。
      disable = {
        ft = { "minifiles", "minifiles-help" },
        bt = { "nofile" },
      },
      spec = {
        {
          mode = { "n", "x" },
          { "<leader>c", group = "code" },
          { "<leader>d", group = "debug" },
          { "<leader>dp", group = "profiler" },
          { "<leader>f", group = "file/find" },
          { "<leader>g", group = "git" },
          { "<leader>q", group = "quit/session" },
          { "<leader>r", group = "rest" },
          { "<leader>s", group = "search" },
          { "<leader>t", group = "tabs/todo" },
          { "<leader>u", group = "ui" },
          { "<leader>x", group = "diagnostics/quickfix" },
          { "<leader>y", group = "yank" },
          { "[", group = "prev" },
          { "]", group = "next" },
          { "g", group = "goto" },
          { "z", group = "fold" },
          {
            "<leader>w",
            group = "windows",
            proxy = "<c-w>",
            expand = function()
              return require("which-key.extras").expand.win()
            end,
          },
          -- better descriptions
          { "gx", desc = "Open url" },
        },
      },
    }
  end,
  keys = {
    {
      "<leader>?",
      function()
        require("which-key").show({ global = false })
      end,
      desc = "Buffer Keymaps (which-key)",
    },
    {
      "<c-w><space>",
      function()
        require("which-key").show({ keys = "<c-w>", loop = true })
      end,
      desc = "Window Hydra Mode (which-key)",
    },
  },
}
