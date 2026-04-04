return {
    {
        "folke/tokyonight.nvim",
        lazy = false, -- 立即加载（默认配色）
        priority = 1000, -- 最高优先级，确保最先加载
        opts = function()
            vim.api.nvim_set_hl(0, "@keyword.import.rust", { link = "@Keyword" })
            return {
                transparent = false,
                style = "moon",
                styles = {
                    functions = { italic = true, bold = true },
                    keywords = { italic = true, bold = true },
                },
                ---@param highlights tokyonight.Highlights
                ---@param colors ColorScheme
                on_highlights = function(highlights, colors)
                    require("tokyonight")
                    highlights["String"].italic = true
                    highlights["Constant"].italic = true
                    highlights["Constant"].bold = true
                    -- Vue 内建 HTML 标签用蓝色
                    highlights["@tag.builtin.vue"] = { fg = colors.blue }
                    -- Vue 组件标签用红色，和 React 保持一致
                    highlights["@tag.vue"] = { fg = colors.red }
                    highlights["@lsp.type.component.vue"] = { fg = colors.red }
                    -- Vue 顶层块标签分别使用独立颜色
                    highlights["@tag.template.vue"] = { fg = colors.purple, bold = true }
                    highlights["@tag.script.vue"] = { fg = colors.yellow, bold = true }
                    highlights["@tag.style.vue"] = { fg = colors.green, bold = true }
                end,
            }
        end,
        config = function(_, opts)
            require("tokyonight").setup(opts)
            vim.cmd.colorscheme("tokyonight-moon") -- 应用配色
        end,
    },
    {
        "rebelot/kanagawa.nvim",
        opts = {},
    },
    {
        "bluz71/vim-moonfly-colors",
        name = "moonfly",
        lazy = false,
        priority = 1000,
    },
    {
        "catppuccin/nvim",
        lazy = true,
        name = "catppuccin",
        opts = {
            flavour = "mocha", -- latte, frappe, macchiato, mocha
            integrations = {
                aerial = true,
                alpha = true,
                cmp = true,
                dashboard = true,
                flash = false,
                fzf = true,
                grug_far = true,
                gitsigns = true,
                headlines = true,
                illuminate = true,
                indent_blankline = { enabled = true },
                leap = true,
                lsp_trouble = true,
                mason = true,
                markdown = true,
                mini = true,
                native_lsp = {
                    enabled = true,
                    underlines = {
                        errors = { "undercurl" },
                        hints = { "undercurl" },
                        warnings = { "undercurl" },
                        information = { "undercurl" },
                    },
                },
                navic = { enabled = true, custom_bg = "lualine" },
                neotest = true,
                neotree = true,
                noice = true,
                notify = true,
                semantic_tokens = true,
                snacks = true,
                telescope = true,
                treesitter = true,
                treesitter_context = true,
                which_key = true,
            },
        },
    },
}
