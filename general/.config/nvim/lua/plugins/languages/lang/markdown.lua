vim.api.nvim_create_autocmd("User", {
    pattern = "VeryLazy",
    callback = function()
        vim.filetype.add({
            extension = { mdx = "markdown.mdx" },
        })
    end,
})

vim.api.nvim_set_hl(0, "@spell", { italic = true })

---@type render.md.UserConfig
local render_md_opt = {
    render_modes = true,
    anti_conceal = {
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
        sign = false,
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
        opts = {
            formatters = {
                ["markdown-toc"] = {
                    condition = function(_, ctx)
                        for _, line in ipairs(vim.api.nvim_buf_get_lines(ctx.buf, 0, -1, false)) do
                            if line:find("<!%-%- toc %-%->") then
                                return true
                            end
                        end
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
        opts = { ensure_installed = { "markdownlint-cli2", "markdown-toc" } },
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
        ft = { "markdown", "norg", "rmd", "org", "codecompanion" },
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
        opts = {
            ensure_installed = { "latex" },
        },
    },
}
