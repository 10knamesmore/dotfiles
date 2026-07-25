-- animations
return {
  -- Animates many common Neovim actions, like scrolling,
  -- moving the cursor, and resizing windows.
  {
    "nvim-mini/mini.animate",
    event = "VeryLazy",
    cond = vim.g.neovide == nil,
    opts = function(_, opts)
      local animate = require("mini.animate")
      return vim.tbl_deep_extend("force", opts, {
        cursor = {
          enable = false, -- 用 kitty 自带的
        },
        resize = {
          timing = animate.gen_timing.cubic({ duration = 50, unit = "total" }),
        },
        open = {
          timing = animate.gen_timing.cubic({ duration = 50, unit = "total" }),
        },
        close = {
          timing = animate.gen_timing.cubic({ duration = 50, unit = "total" }),
        },
        -- 滚动动画交给 snacks.scroll，别在这里重新打开。
        -- mini.animate 的滚动动画是固定时长的：连按 J/K（间隔 < 动画时长）时，
        -- 新动画会接管未跑完的旧动画，且开头 winrestview 把视图拉回起点，
        -- 于是视图被反复拉回、光标每帧被夹回可视区 —— 表现为光标抽动、画面滚不动。
        -- snacks.scroll 有 animate_repeat：检测到连按时把动画压到 50ms，避开这个竞态。
        scroll = { enable = false },
      })
    end,
  },
}
