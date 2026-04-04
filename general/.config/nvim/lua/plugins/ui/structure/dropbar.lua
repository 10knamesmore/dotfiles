return {
    "Bekaboo/dropbar.nvim",
    event = { "BufReadPost", "BufWritePost", "BufNewFile" },
    dependencies = {
        "nvim-tree/nvim-web-devicons",
    },
    opts = {
        bar = {
            enable = function(buf, win, _)
                if not vim.api.nvim_win_is_valid(win) then
                    return false
                end

                if vim.bo[buf].buftype ~= "" then
                    return false
                end

                local ignore_ft = {
                    "TelescopePrompt",
                    "Trouble",
                    "alpha",
                    "dashboard",
                    "help",
                    "lazy",
                    "mason",
                    "neo-tree",
                    "noice",
                    "notify",
                    "qf",
                    "snacks_dashboard",
                    "terminal",
                    "toggleterm",
                }

                return not vim.tbl_contains(ignore_ft, vim.bo[buf].filetype)
            end,
            pick = {
                pivots = "asdfghjkl;qwertyuiop"
            }
        },
        menu = {
            win_configs = {
                border = "rounded",
            },
            keymaps = {
                ["h"] = "<C-w>q",
                ["l"] = function()
                    local menu = require("dropbar.utils").menu.get_current()
                    if not menu then
                        return
                    end
                    local cursor = vim.api.nvim_win_get_cursor(menu.win)
                    local component = menu.entries[cursor[1]]:first_clickable(cursor[2])
                    if component then
                        menu:click_on(component, nil, 1, "l")
                    end
                end,
            },
        },
    },
    keys = {
        {
            "<leader>h",
            function()
                require("dropbar.api").pick()
            end,
            desc = "Dropbar Pick",
        },
        {
            "[;",
            function()
                require("dropbar.api").goto_context_start()
            end,
            desc = "Dropbar Prev Context",
        },
        {
            "];",
            function()
                require("dropbar.api").select_next_context()
            end,
            desc = "Dropbar Next Context",
        },
    },
}
