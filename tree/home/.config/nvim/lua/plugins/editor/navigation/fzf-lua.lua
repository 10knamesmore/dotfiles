-- fzf-lua：渐进式从 telescope 迁移的 fuzzy finder。
-- 当前只接管「重活」——LSP 相关（定义/引用/实现/符号）、live_grep、find_files；
-- 这些在大仓里 telescope 走 Lua entry-maker + 每击键重排最卡，fzf-lua 把过滤直接交给
-- fzf 二进制（async 不阻塞 UI），体感快很多。其余 picker 仍留 telescope（见下方注释块）。
-- 想继续迁某个 picker：取消对应键的注释，并去 telescope.lua 注释掉同名键即可。
--
-- 交互模型和 telescope 不同（这里没有 telescope 那种「normal 模式 + 裸 j/k/l/d」）：
--   · 始终在输入框打字过滤；上下用 ctrl-j/ctrl-k 或方向键，回车确认。
--   · 打开方式：ctrl-s 水平分屏 / ctrl-v 垂直分屏 / ctrl-t 新标签（= 旧 telescope 的 s/v）。
--   · files 里运行时切换：alt-i 显示被 ignore 的、alt-h 显示 hidden、alt-f 跟随软链。
--   · 预览滚动 ctrl-u/ctrl-d（保留旧手感）。
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
      return {
        "telescope",
        -- fzf-lua 默认开 --highlight-line（fzf 0.73 的「整行满宽高亮当前行」）。它在滚动长列表 +
        -- nvim 0.12 终端 + fzf 0.73.1 下重绘时会把列表少画行/渲染错乱（预览正常、只有列表崩）。
        -- 关掉它，长列表滚动就正常，fuzzy(grep_project) 也能用了。改回 true 可复现。
        fzf_opts = { ["--highlight-line"] = false },
        winopts = {
          height = 0.85,
          width = 0.85,
          -- 关掉结果列表的 treesitter 高亮（nvim≥0.10 默认开）。它异步解析/高亮整份结果、
          -- 每次打开重挂载，大列表 + 重复打开时和 fzf 终端重绘 race，导致「第一次好、第二次
          -- 列表少画行/错乱」。关掉后结果行没了 TS 语法色，但 fuzzy 匹配高亮/图标/前景色都在。
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
        -- ui-select 暂留 telescope-ui-select，故不设 ui_select（设了 setup 会自动接管 vim.ui.select）。
        -- 若哪天想让 fzf-lua 接管 ui.select，取消下面注释并去掉 telescope-ui-select：
        -- ui_select = function(fzf_opts, items)
        --   return vim.tbl_deep_extend("force", fzf_opts, {
        --     winopts = { width = 0.6, height = math.min(#items + 4, 20) + 0.01 },
        --   })
        -- end,
      }
    end,
    config = function(_, opts)
      require("fzf-lua").setup(opts)
    end,
    -- rhs 一律内联 `require("fzf-lua").xxx`：require 在函数体内、只在按键时执行，
    -- lazy 只求值 keys 拿键列表不会调 rhs，故不 eager 加载；同时 LuaLS 能静态解析
    -- require("fzf-lua") 的返回类型，给出 provider 补全与签名提示（别改回字符串索引）。
    keys = {
      -- ===== 已迁到 fzf-lua（LSP / live_grep / find_files）=====
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
      -- <leader>/ 默认 fuzzy：grep_project 把整个项目的 rg 行一次性灌进 fzf 直接 fuzzy 过滤
      -- （--nth 3.. 只 fuzzy 内容列，不含 文件:行号）。按 ctrl-g 才切回 live-rg（每击键重跑 rg，
      -- 即旧 live_grep 行为，与 zsh 的 Ctrl-S 同构）。代价：大仓 rg "" 会 dump 全量行，比 live 重。
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
          -- 旧 <leader><space> 是 find_files{hidden, no_ignore}：看到一切
          require("fzf-lua").files({ fd_opts = "--color=never --type f --hidden --no-ignore " .. fd_excludes() })
        end,
        desc = "Find Files (all)",
      },

      -- ===== 暂留 telescope（想迁就取消注释，并去 telescope.lua 注释掉同名键）=====
      -- { "f", function() require("fzf-lua").blines() end, desc = "search in current buffer" },
      -- { "<leader>s:", function() require("fzf-lua").command_history() end, desc = "Command History" },
      -- { "<leader>sw", function() require("fzf-lua").grep_cword() end, desc = "search current word" },
      -- { "<leader>sw", function() require("fzf-lua").grep_visual() end, mode = "v", desc = "search selection" },
      -- { "<leader>b", function() require("fzf-lua").buffers() end, desc = "Buffers" },
      -- { "<leader>sa", function() require("fzf-lua").autocmds() end, desc = "Auto Commands" },
      -- { "<leader>sc", function() require("fzf-lua").commands() end, desc = "Commands" },
      -- { "<leader>sk", function() require("fzf-lua").keymaps() end, desc = "Keymaps" },
      -- { "<leader>sf", function() require("fzf-lua").filetypes() end, desc = "Change Current Filetypes" },
      -- { "<leader>sm", function() require("fzf-lua").marks() end, desc = "Marks" },
      -- { "<leader>so", function() require("fzf-lua").nvim_options() end, desc = "Options" },
      -- { "<leader>sh", function() require("fzf-lua").oldfiles() end, desc = "History Files" },
      -- { "<leader>sr", function() require("fzf-lua").registers() end, desc = "registers" },
      -- { "<leader>sj", function() require("fzf-lua").jumps() end, desc = "Jump List" },
      -- { "<leader>sC", function() require("fzf-lua").colorschemes() end, desc = "Colorscheme" },
      -- { "<leader>su", function() require("fzf-lua").undotree() end, desc = "Undo History" },
    },
  },
}
