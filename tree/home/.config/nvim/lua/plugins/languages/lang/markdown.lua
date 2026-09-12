vim.api.nvim_create_autocmd("User", {
  pattern = "VeryLazy",
  callback = function()
    vim.filetype.add({
      extension = { mdx = "markdown.mdx" },
    })
  end,
})

vim.api.nvim_set_hl(0, "@spell", { italic = true })

-- 绝对路径 markdown link 不是 marksman 的 definition 输入；链接上直接打开文件，其他位置保留 LSP fallback。
local function decode_uri_component(value)
  return (value:gsub("%%(%x%x)", function(hex)
    return string.char(tonumber(hex, 16))
  end))
end

local function normalize_markdown_anchor(value)
  value = decode_uri_component(vim.trim(value)):lower()
  value = value:gsub("`([^`]*)`", "%1")
  value = value:gsub("%[([^%]]-)%]%([^)]-%)", "%1")
  value = value:gsub("[%*_~]", "")
  value = value:gsub("[.,!?;:'\"()]", "")
  value = value:gsub("%s+", "-"):gsub("%-+", "-")
  return value:gsub("^%-+", ""):gsub("%-+$", "")
end

local function find_markdown_anchor(anchor)
  local wanted = normalize_markdown_anchor(anchor:gsub("^#", ""))
  if wanted == "" then
    return
  end

  local seen = {}
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  for row, line in ipairs(lines) do
    local heading = line:match("^%s*#+%s+(.+)$")
    if heading then
      heading = heading:gsub("%s+#+%s*$", "")
      local slug = normalize_markdown_anchor(heading)
      local duplicate = seen[slug] or 0
      seen[slug] = duplicate + 1
      local candidate = duplicate == 0 and slug or (slug .. "-" .. duplicate)
      if candidate == wanted then
        vim.api.nvim_win_set_cursor(0, { row, 0 })
        vim.cmd("normal! zz")
        return true
      end
    end
  end

  vim.notify("未找到 Markdown 锚点: #" .. anchor:gsub("^#", ""), vim.log.levels.WARN)
  return false
end

local function markdown_link_at_cursor()
  local row, column = unpack(vim.api.nvim_win_get_cursor(0))
  local line = vim.api.nvim_buf_get_lines(0, row - 1, row, false)[1] or ""
  local cursor = column + 1

  local from = 1
  while true do
    local start_pos, end_pos, _, target = line:find("%[([^%]]-)%]%((.-)%)", from)
    if not start_pos then
      break
    end
    if cursor >= start_pos and cursor <= end_pos then
      return target
    end
    from = end_pos + 1
  end

  from = 1
  while true do
    local start_pos, end_pos, target = line:find("%[%[([^%]]-)%]%]", from)
    if not start_pos then
      break
    end
    if cursor >= start_pos and cursor <= end_pos then
      return target
    end
    from = end_pos + 1
  end
end

local function resolve_markdown_target(raw_target)
  local target = vim.trim(raw_target)
  if target == "" then
    return
  end

  if target:sub(1, 1) == "<" then
    target = target:match("^<([^>]*)>") or target
  else
    target = target:match("^(%S+)") or target
  end

  local path_part, anchor = target:match("^(.-)#(.*)$")
  if path_part == nil then
    path_part = target
  end
  if path_part:sub(1, 1) == "~" then
    path_part = vim.fn.expand(path_part)
  end

  local source = vim.api.nvim_buf_get_name(0)
  if path_part:match("^%a[%w+.-]*://") then
    if path_part:lower():sub(1, 7) ~= "file://" then
      return nil, "external"
    end
    path_part = vim.uri_to_fname(path_part)
  else
    path_part = decode_uri_component(path_part)
    if path_part == "" then
      path_part = source
    elseif path_part:sub(1, 1) ~= "/" then
      path_part = vim.fs.joinpath(vim.fs.dirname(source), path_part)
    end
  end

  return vim.fs.normalize(path_part), anchor
end

local function goto_markdown_link()
  local raw_target = markdown_link_at_cursor()
  if not raw_target then
    return require("fzf-lua").lsp_definitions()
  end

  local path, anchor = resolve_markdown_target(raw_target)
  if path == nil then
    return require("fzf-lua").lsp_definitions()
  end

  if not vim.uv.fs_stat(path) then
    vim.notify("Markdown 链接目标不存在: " .. path, vim.log.levels.WARN)
    return
  end

  local current = vim.fs.normalize(vim.api.nvim_buf_get_name(0))
  if path ~= current then
    local ok = pcall(vim.cmd, "edit " .. vim.fn.fnameescape(path))
    if not ok then
      vim.notify("无法打开 Markdown 链接目标: " .. path, vim.log.levels.WARN)
      return
    end
  end

  if anchor and anchor ~= "" then
    find_markdown_anchor(anchor)
  end
end

vim.api.nvim_create_autocmd("FileType", {
  pattern = { "markdown", "markdown.mdx" },
  callback = function(event)
    vim.keymap.set("n", "gd", goto_markdown_link, {
      buffer = event.buf,
      silent = true,
      desc = "Go to Markdown Link or Definition",
    })
  end,
})

---@type render.md.UserConfig
local render_md_opt = {
  render_modes = true,
  anti_conceal = {
    -- normal 模式整页保持渲染（含光标行），只在 insert 等模式下取消渲染光标行
    disabled_modes = { "n" },
    -- Number of lines above cursor to show.
    above = 0,
    -- Number of lines below cursor to show.
    below = 0,
  },
  completions = {
    lsp = {
      enabled = true,
    },
  },
  code = {
    sign = true,
    width = "block",
    right_pad = 4,
  },
    -- stylua: ignore
    callout = {
        note      = { quote_icon = "█"},
        tip       = { quote_icon = "█"},
        important = { quote_icon = "█"},
        warning   = { quote_icon = "█"},
        caution   = { quote_icon = "█"},
        abstract  = { quote_icon = "█"},
        summary   = { quote_icon = "█"},
        tldr      = { quote_icon = "█"},
        info      = { quote_icon = "█"},
        todo      = { quote_icon = "█"},
        hint      = { quote_icon = "█"},
        success   = { quote_icon = "█"},
        check     = { quote_icon = "█"},
        done      = { quote_icon = "█"},
        question  = { quote_icon = "█"},
        help      = { quote_icon = "█"},
        faq       = { quote_icon = "█"},
        attention = { quote_icon = "█"},
        failure   = { quote_icon = "█"},
        fail      = { quote_icon = "█"},
        missing   = { quote_icon = "█"},
        danger    = { quote_icon = "█"},
        error     = { quote_icon = "█"},
        bug       = { quote_icon = "█"},
        example   = { quote_icon = "█"},
        quote     = { quote_icon = "█"},
        cite      = { quote_icon = "█"},
    },
  heading = {
    sign = false,
    position = "inline",
    width = "full",
  },
  checkbox = {
    enabled = true,
    bullet = false,
    left_pad = 0,
    right_pad = 1,
    unchecked = {
      icon = "󰄱 ",
      highlight = "RenderMarkdownUnchecked",
      scope_highlight = nil,
    },
    checked = {
      icon = "󰱒 ",
      highlight = "RenderMarkdownChecked",
      scope_highlight = nil,
    },
    custom = {
      todo = {
        raw = "[-]",
        rendered = "󰥔 ",
        highlight = "RenderMarkdownTodo",
        scope_highlight = nil,
      },
      important = {
        raw = "[~]",
        rendered = "󰓎 ",
        highlight = "DiagnosticWarn",
      },
    },
    scope_priority = nil,
  },
  latex = {
    enabled = true,
    converter = { "utftex", "latex2text" }, -- NOTE: 需要安装 cli 工具
    highlight = "RenderMarkdownMath",
    position = "above",
    top_pad = 0,
    bottom_pad = 1,
  },
  sign = {
    enabled = false,
  },
  paragraph = { left_margin = 0 },
}

return {
  {
    "stevearc/conform.nvim",
    optional = true,
    ---@module "conform"
    ---@type conform.setupOpts
    opts = {
      formatters = {
        ["markdown-toc"] = {
          condition = function(_, ctx)
            for _, line in ipairs(vim.api.nvim_buf_get_lines(ctx.buf, 0, -1, false)) do
              if line:find("<!%-%- toc %-%->") then
                return true
              end
            end
            -- 上游把 condition 标成 fun(...):boolean（不是 boolean?），
            -- 落到函数末尾隐式返回 nil 会触发 missing-return。行为等价。
            return false
          end,
        },
        ["markdownlint-cli2"] = {
          condition = function(_, ctx)
            local diag = vim.tbl_filter(function(d)
              return d.source == "markdownlint"
            end, vim.diagnostic.get(ctx.buf))
            return #diag > 0
          end,
        },
      },
      formatters_by_ft = {
        ["markdown"] = { "prettier", "markdownlint-cli2", "markdown-toc" },
        ["markdown.mdx"] = { "prettier", "markdownlint-cli2", "markdown-toc" },
      },
    },
  },
  {
    "mason-org/mason.nvim",
    -- prettier 是上面 formatters_by_ft 里 markdown 链的第一环，必须一起装：
    -- conform 对不可用的 formatter 是静默跳过、其余照跑，换机后只会「排版无声消失」。
    ---@module "mason"
    ---@type MasonSettings | {ensure_installed: string[]}
    opts = { ensure_installed = { "prettier", "markdownlint-cli2", "markdown-toc" } },
  },
  -- markdown 不lint
  -- {
  --     "mfussenegger/nvim-lint",
  --     optional = true,
  --     opts = {
  --         linters_by_ft = {
  --             markdown = { "markdownlint-cli2" },
  --         },
  --     },
  -- },
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        marksman = {},
      },
    },
  },
  -- Markdown preview
  {
    "iamcco/markdown-preview.nvim",
    cmd = { "MarkdownPreviewToggle", "MarkdownPreview", "MarkdownPreviewStop" },
    build = function()
      require("lazy").load({ plugins = { "markdown-preview.nvim" } })
      vim.fn["mkdp#util#install"]()
    end,
    keys = {
      {
        "<leader>cp",
        ft = "markdown",
        "<cmd>MarkdownPreviewToggle<cr>",
        desc = "Markdown Preview",
      },
    },
    config = function()
      vim.cmd([[do FileType]])
    end,
  },

  {
    "MeanderingProgrammer/render-markdown.nvim",
    opts = render_md_opt,
    ft = { "markdown", "norg", "rmd", "org" },
    config = function(_, opts)
      local render_md = require("render-markdown")
      vim.b.md_left_margin = 0
      render_md.setup(opts)
      Snacks.toggle({
        name = "Render Markdown",
        get = render_md.get,
        set = render_md.set,
      }):map("<leader>um")

      -- 只在 markdown 下切换 paragraph left_margin
      vim.api.nvim_create_autocmd("FileType", {
        pattern = "markdown",
        callback = function()
          vim.b.md_left_margin = vim.b.md_left_margin or 0.5
          vim.keymap.set("n", "<leader>cm", function()
            local cfg = vim.deepcopy(render_md_opt)

            if vim.b.md_left_margin == 0.5 then
              cfg.latex.position = "center"
              vim.b.md_left_margin = 0
            else
              cfg.latex.position = "above"
              vim.b.md_left_margin = 0.5
            end

            cfg.paragraph.left_margin = vim.b.md_left_margin

            render_md.setup(cfg)
          end, { buffer = true, desc = "切换 Paragraph Left Margin" })
        end,
      })
    end,
  },
  {
    "nvim-treesitter/nvim-treesitter",
    ---@module "nvim-treesitter"
    ---@type TSConfig | {ensure_installed: string[]}
    opts = {
      ensure_installed = { "latex" },
    },
  },
}
