-- 状态栏显示项目根目录名。
local function dirname()
  return vim.fs.basename(utils.path.get_root())
end

return {
  "nvim-lualine/lualine.nvim",
  -- event = "VeryLazy",
  event = function()
    return { "BufReadPost", "BufWritePost", "BufNewFile" }
  end,
  init = function()
    vim.g.lualine_laststatus = vim.o.laststatus
    if vim.fn.argc(-1) > 0 then
      -- set an empty statusline till lualine loads
      vim.o.statusline = " "
    else
      -- hide the statusline on the starter page
      vim.o.laststatus = 0
    end
  end,
  opts = function()
    vim.o.laststatus = vim.g.lualine_laststatus

    local opts = {
      options = {
        theme = require("config.disco.lualine").theme,
        globalstatus = vim.o.laststatus == 3,
        disabled_filetypes = {
          statusline = { "dashboard", "alpha", "ministarter", "snacks_dashboard" },
        },

        section_separators = { left = "", right = "" },
        component_separators = { left = "", right = "" },
      },

      sections = {
        lualine_a = { { "mode" } },

        lualine_b = {
          {
            function()
              return "󱉭 " .. dirname()
            end,
            color = function()
              return { fg = Snacks.util.color("Directory") }
            end,
          },

          {
            "branch",
            color = function()
              return { fg = Snacks.util.color("Identifier") }
            end,
          },
        },

        lualine_c = {
          { utils.lualine.pretty_path(), separator = "" },

          { "filetype", icon_only = true, padding = { left = 0, right = 0 } },

          {
            "diagnostics",

            symbols = {
              error = " ",
              warn = " ",
              hint = " ",
              info = " ",
            },
          },
          {
            function()
              local parts = {}
              local names = {}
              for _, c in ipairs(vim.lsp.get_clients({ bufnr = 0 })) do
                names[#names + 1] = c.name
              end
              if #names > 0 then
                parts[#parts + 1] = " " .. table.concat(names, ",")
              end
              local ok, conform = pcall(require, "conform")
              if ok then
                local fs = {}
                for _, f in ipairs(conform.list_formatters_to_run(0)) do
                  fs[#fs + 1] = f.name
                end
                if #fs > 0 then
                  parts[#parts + 1] = " " .. table.concat(fs, ",")
                end
              end
              local ok2, lint = pcall(require, "lint")
              if ok2 then
                local ls = lint.linters_by_ft[vim.bo.filetype]
                if ls and #ls > 0 then
                  parts[#parts + 1] = "󱉶 " .. table.concat(ls, ",")
                end
              end
              return table.concat(parts, "  ")
            end,
            color = function()
              return { fg = Snacks.util.color("Comment") }
            end,
          },
        },

        lualine_x = {

          {
            "searchcount",
            color = function()
              return { fg = Snacks.util.color("Special") }
            end,
          },
          -- 当前命令提示
          {
            function()
              return require("noice").api.status.command.get()
            end,
            cond = function()
              return package.loaded["noice"] and require("noice").api.status.command.has()
            end,
            color = function()
              return { fg = Snacks.util.color("Statement") }
            end,
          },

          {
            "diff",
            symbols = {
              added = " ",
              modified = " ",
              removed = " ",
            },
            -- 使用 gitsigns 插件提供的缓存 diff 数据源
            source = function()
              local gitsigns = vim.b.gitsigns_status_dict
              if gitsigns then
                return {
                  added = gitsigns.added,
                  modified = gitsigns.changed,
                  removed = gitsigns.removed,
                }
              end
            end,
          },
        },

        lualine_y = {
          {
            "encoding",
            separator = "",
            padding = { left = 1, right = 1 },
            color = function()
              return { fg = Snacks.util.color("Type"), gui = "italic,bold" }
            end,
          },
          {
            "filesize",
            color = function()
              return { fg = Snacks.util.color("Keyword"), gui = "italic,bold" }
            end,
          },
        },

        lualine_z = {
          {
            "selectioncount",
            separator = " ",
            padding = { left = 1, right = 1 },
            color = { gui = "bold" },
          },
          {
            "progress",
            separator = " ",
            padding = { left = 0, right = 1 },
            color = { gui = "italic,bold" },
          },
          {
            "location",
            padding = { left = 0, right = 1 },
            color = { gui = "italic,bold" },
          },
        },
      },
      extensions = { "trouble", "mason", "lazy", "fzf" },
    }

    return opts
  end,
}
