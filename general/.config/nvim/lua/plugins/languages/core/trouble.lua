return {
    "folke/trouble.nvim",
    cmd = "Trouble",
    opts = {
        modes = {
            symbols = { -- Configure symbols mode
                win = {
                    type = "split", -- split window
                    relative = "win", -- relative to current window
                    position = "left", -- right side
                    size = 0.3,
                },
            },
            diagnostics = {
                win = {
                    type = "split",
                    relative = "win",
                    position = "bottom",
                    size = 0.20,
                },
            },
            lsp = {
                win = {
                    -- type = "float", -- split window
                    -- relative = "cursor",
                    --
                    type = "split", -- split window
                    relative = "editor", -- relative to current window
                    position = "right", -- right side
                    size = 0.3,
                },
            },
        },
        keys = {
            ["l"] = "jump", -- 跳转到选中的条目
        },
    },
    keys = function()
        return {
            { "<leader>xx", "<cmd>Trouble diagnostics toggle<cr>", desc = "Diagnostics (Trouble)" },
            { "<leader>xb", "<cmd>Trouble diagnostics toggle filter.buf=0<cr>", desc = "Buffer Diagnostics (Trouble)" },
            { "<leader>cs", "<cmd>Trouble symbols toggle<cr>", desc = "Symbols (Trouble)" },
            {
                "gl",
                "<cmd>Trouble lsp toggle focus=true<cr>",
                desc = "LSP references/definitions/... (Trouble)",
            },
        }
    end,
}
