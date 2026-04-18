-- Formatting 核心插件：conform.nvim
-- 通过 utils.format 注册为 primary formatter，由 `=` 键触发
return {
    {
        "stevearc/conform.nvim",
        dependencies = { "mason.nvim" },
        lazy = true,
        cmd = "ConformInfo",
        keys = {
            {
                "<leader>cF",
                function()
                    require("conform").format({ formatters = { "injected" }, timeout_ms = 3000 })
                end,
                mode = { "n", "x" },
                desc = "Format Injected Langs",
            },
            {
                "=",
                function()
                    require("utils.format").format({ force = true })
                    vim.cmd("w")
                end,
                mode = { "n", "x", "v" },
                desc = "Format File",
            },
        },
        init = function()
            local Util = require("utils")
            Util.on_very_lazy(function()
                Util.format.register({
                    name = "conform.nvim",
                    priority = 100,
                    primary = true,
                    format = function(buf)
                        require("conform").format({ bufnr = buf })
                    end,
                    sources = function(buf)
                        return vim.tbl_map(function(v)
                            return v.name
                        end, require("conform").list_formatters(buf))
                    end,
                })
            end)
        end,
        opts = {
            default_format_opts = {
                timeout_ms = 3000,
                async = false,
                quiet = false,
                lsp_format = "fallback",
            },
            formatters_by_ft = {
                lua = { "stylua" },
                fish = { "fish_indent" },
                sh = { "shfmt" },
            },
            formatters = {
                injected = { options = { ignore_errors = true } },
                stylua = {
                    prepend_args = { "--indent-type", "Spaces", "--indent-width", "4" },
                },
            },
        },
    },
}
