--- 给rename 添加一些更好的预览功能
return {
    -- Rename with cmdpreview
    desc = "Incremental LSP renaming based on Neovim's command-preview feature",
    {
        "smjonas/inc-rename.nvim",
        cmd = "IncRename",
        opts = {},
    },

    -- LSP Keymaps
    {
        "neovim/nvim-lspconfig",
        opts = {
            servers = {
                ["*"] = {
                    keys = {
                        {
                            "<leader>cr",
                            function()
                                local inc_rename = require("inc_rename")
                                return ":" .. inc_rename.config.cmd_name .. " " .. vim.fn.expand("<cword>")
                            end,
                            expr = true,
                            desc = "Rename (inc-rename.nvim)",
                            has = "rename",
                        },
                    },
                },
            },
        },
    },

    --- Noice integration
    {
        "folke/noice.nvim",
        optional = true,
        opts = {
            presets = { inc_rename = true },
        },
    },
}
