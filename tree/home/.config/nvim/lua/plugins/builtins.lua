return {
  {
    dir = vim.fn.stdpath("config") .. "/lua/builtins/pi-bridge",
    name = "pi-bridge",
    main = "pi_bridge",
    lazy = false,
    opts = {},
  },
  {
    dir = vim.fn.stdpath("config") .. "/lua/builtins/window-translate",
    name = "window-translate",
    main = "window_translate",
    cmd = { "TranslateToggle", "TranslateMode" },
    keys = {
      {
        "<M-q>",
        function()
          require("window_translate").toggle()
        end,
        mode = { "n", "x", "i" },
        desc = "Toggle window translation",
      },
    },
    ---@type WindowTranslate.Options
    opts = {
      mode = "inline", -- inline | window（浮窗）| split（左右分栏）
      api = "responses", -- responses | deepseek
      endpoint = "https://api.openai.com/v1/responses",
      model = "gpt-6-luna",
      api_key_env = "OPENAI_API_KEY",
      fast = true, -- Responses 服务档位；两种 API 都关闭思考
      target_language = "中文",
      debounce_ms = 50,
      max_concurrent = 6, -- 每个源窗口的并发请求上限，可见块优先
      chunks_per_request = 3, -- 每个请求最多翻译的块数，全部返回译文
      prefetch = {
        ahead = 2, -- 滚动方向前方，按源窗口高度逐屏请求
        behind = 1,
      },
      style = {
        inline = {
          prefix = "  │ ",
          highlight = "WindowTranslate",
          padding_above = 0,
          padding_below = 0,
          reserve_lines = 1, -- 每个待翻译块预留的虚拟行数，完成后收起空位
          wrap = true,
        },
        -- 返回 nvim_open_win() 的原生配置，尺寸在打开时计算。
        window = function()
          local width = math.floor(vim.o.columns * 0.45)
          return {
            relative = "editor",
            row = 2,
            col = vim.o.columns - width - 2,
            width = width,
            height = math.floor(vim.o.lines * 0.6),
            border = "rounded",
            title = " 翻译 ",
            title_pos = "center",
            style = "minimal",
          }
        end,
        split = function(source_window)
          return {
            split = "right",
            win = source_window,
            width = math.floor(vim.api.nvim_win_get_width(source_window) / 2),
          }
        end,
        -- 浮窗与分栏共用的 window-local options。
        window_options = {
          wrap = true,
          linebreak = true,
          number = false,
          relativenumber = false,
          signcolumn = "no",
          foldcolumn = "0",
          winblend = 0,
          winhighlight = "Normal:WindowTranslateNormal,NormalFloat:WindowTranslateNormal,FloatBorder:WindowTranslateBorder",
        },
        progress = { text = "translating…", highlight = "WindowTranslateProgress" },
        -- 原生 nvim_set_hl() 属性；自定义颜色时把 link 整项替换掉。
        highlights = {
          WindowTranslate = { fg = "#a6c9a5", italic = true },
          WindowTranslateNormal = { link = "NormalFloat" },
          WindowTranslateBorder = { link = "FloatBorder" },
          WindowTranslateProgress = { link = "Comment" },
        },
      },
    },
  },
}
