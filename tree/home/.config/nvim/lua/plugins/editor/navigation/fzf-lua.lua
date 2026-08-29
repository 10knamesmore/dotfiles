-- fzf-lua 负责 LSP 定位、grep 与文件查找；其他 picker 由 telescope 提供。
-- 过滤由 fzf 进程执行，不阻塞 Neovim UI。
--
-- 交互模型：
--   · 始终在输入框打字过滤；上下用 ctrl-j/ctrl-k 或方向键，回车确认。
--   · ctrl-s 水平分屏 / ctrl-v 垂直分屏 / ctrl-t 新标签。
--   · files 里运行时切换：alt-i 显示被 ignore 的、alt-h 显示 hidden、alt-f 跟随软链。
--   · 预览滚动 ctrl-u/ctrl-d。
local ignore_globs = {
  "!.git",
  "!node_modules",
  "!target",
  "!.venv",
  "!__pycache__",
  "!.DS_Store",
}

-- 把 ignore_globs 拼成 fd 的 --exclude 串与 rg 的 -g '!…' 串
local function fd_excludes()
  local parts = {}
  for _, g in ipairs(ignore_globs) do
    parts[#parts + 1] = "--exclude " .. vim.fn.shellescape(g:sub(2)) -- 去掉前导 '!'
  end
  return table.concat(parts, " ")
end

local function rg_excludes()
  local parts = {}
  for _, g in ipairs(ignore_globs) do
    parts[#parts + 1] = "-g " .. vim.fn.shellescape(g)
  end
  return table.concat(parts, " ")
end

return {
  {
    "ibhagwan/fzf-lua",
    cmd = "FzfLua",
    dependencies = { "nvim-tree/nvim-web-devicons" },
    opts = function()
      ---@type fzf-lua.Config
      return {
        "telescope",
        -- highlight-line 会在长列表滚动时造成终端列表重绘缺行；禁用后不影响 fuzzy 匹配高亮。
        fzf_opts = { ["--highlight-line"] = false },
        winopts = {
          height = 0.85,
          width = 0.85,
          -- treesitter 异步高亮会与 fzf 终端重绘竞争并导致列表缺行；关闭后仍保留 fuzzy 高亮和图标。
          treesitter = { enabled = false },
          preview = {
            layout = "flex",
            scrollbar = "float",
            delay = 50,
          },
        },
        keymap = {
          -- 内置 previewer（neovim buffer 预览）里的键
          builtin = {
            ["<C-u>"] = "preview-page-up",
            ["<C-d>"] = "preview-page-down",
          },
          -- 传给 fzf 进程的键
          fzf = {
            ["ctrl-u"] = "preview-page-up",
            ["ctrl-d"] = "preview-page-down",
            -- 多选：ctrl-q 把选中项灌进 quickfix
            ["ctrl-q"] = "select-all+accept",
          },
        },
        files = {
          -- 默认尊重 .gitignore；额外排除重目录。alt-i/alt-h 可运行时反悔。
          fd_opts = "--color=never --type f --hidden --follow " .. fd_excludes(),
        },
        grep = {
          rg_opts = "--column --line-number --no-heading --color=always --smart-case "
            .. "--max-columns=4096 "
            .. rg_excludes(),
        },
        -- 不配置 ui_select，由 telescope-ui-select 保持 vim.ui.select ownership。
      }
    end,
    config = function(_, opts)
      require("fzf-lua").setup(opts)
    end,
    -- rhs 内联 require，只在按键时加载；静态属性访问同时保留 LuaLS provider 补全。
    keys = {
      -- ===== fzf-lua providers =====
      {
        "gd",
        function()
          require("fzf-lua").lsp_definitions()
        end,
        desc = "Goto Definition",
      }, -- jump1 默认单结果直跳
      {
        "gi",
        function()
          require("fzf-lua").lsp_implementations()
        end,
        desc = "Goto Implementations",
      },
      {
        "gr",
        function()
          require("fzf-lua").lsp_references()
        end,
        desc = "References",
        nowait = true,
      },
      {
        "<leader>ss",
        function()
          require("fzf-lua").lsp_document_symbols()
        end,
        desc = "Symbol",
      },
      {
        "<leader>sS",
        function()
          require("fzf-lua").lsp_live_workspace_symbols()
        end,
        desc = "Symbol (Workspace)",
      },
      -- grep_project 一次读取项目匹配行，再由 fzf 过滤内容列；ctrl-g 切换为每次输入重跑 rg 的 live-rg。
      -- 大仓库的初始空查询会读取大量行。
      {
        "<leader>/",
        function()
          require("fzf-lua").grep_project()
        end,
        desc = "Grep (fuzzy)",
      },
      {
        "<leader><space>",
        function()
          -- 包含 hidden 和 ignored 文件，但继续排除 ignore_globs 中的重目录。
          require("fzf-lua").files({ fd_opts = "--color=never --type f --hidden --no-ignore " .. fd_excludes() })
        end,
        desc = "Find Files (all)",
      },
    },
  },
}
