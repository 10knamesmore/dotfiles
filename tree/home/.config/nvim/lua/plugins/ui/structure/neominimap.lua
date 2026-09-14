---@module "neominimap.config.meta"
return {
  "Isrothy/neominimap.nvim",
  version = "v3.x.x",
  lazy = false, -- 作者推荐不使用 lazy load，以便正确挂载 buffer/window 监听
  keys = {
    { "<leader>m", "<cmd>Neominimap Toggle<cr>", desc = "Toggle Global Minimap" },
  },
  init = function()
    ---@type Neominimap.UserConfig
    vim.g.neominimap = {
      auto_enable = true, -- 打开文件时默认启用小地图
      log_level = vim.log.levels.OFF,
      notification_level = vim.log.levels.INFO,

      -- 排除不需要小地图的浮窗和文件类型
      exclude_filetypes = {
        "help",
        "bigfile",
        "dashboard",
        "snacks_dashboard",
        "alpha",
        "lazy",
        "mason",
        "trouble",
        "qf",
        "minifiles",
        "minifiles-help",
        "snacks_picker_input",
      },

      -- 排除非普通文件的 buftype
      exclude_buftypes = {
        "nofile",
        "nowrite",
        "quickfix",
        "terminal",
        "prompt",
      },

      x_multiplier = 2, -- 横向压缩比例（1 个字符代表的列数）
      y_multiplier = 1, -- 纵向压缩比例

      -- 当前行在小地图视口居中
      current_line_position = "percent",

      --- 可选 "float"（浮动跟随）或 "split"（分屏固定侧边栏）
      layout = "float",

      split = {
        minimap_width = 20,
        fix_width = false,
        direction = "right",
        close_if_last_window = false,
        persist = true,
      },

      float = {
        minimap_width = 25,
        max_minimap_height = nil,
        margin = {
          right = 0,
          top = 0,
          bottom = 0,
        },
        z_index = 30,
        window_border = "rounded",
        persist = true,
      },

      delay = 200, -- 文本变化后防抖延时 (ms)
      sync_cursor = true, -- 光标移动时同步滚动小地图

      click = {
        enabled = false,
      },

      diagnostic = {
        enabled = true,
        mode = "line",
        priority = {
          ERROR = 100,
          WARN = 90,
          INFO = 80,
          HINT = 70,
        },
      },

      git = {
        enabled = true,
        mode = "icon",
        priority = 6,
      },

      search = {
        enabled = true,
        mode = "sign",
        priority = 20,
      },

      treesitter = {
        enabled = true,
        priority = 200,
      },

      fold = {
        enabled = true, -- 感知代码折叠
      },
    }
  end,
}
