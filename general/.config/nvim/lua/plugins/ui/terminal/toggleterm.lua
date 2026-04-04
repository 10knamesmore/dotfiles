return {
    -- amongst your other plugins
    {
        "akinsho/toggleterm.nvim",
        version = "*",
        opts = {
            dir = vim.fn.getcwd(),
            direction = "float",
            open_mapping = [[<C-\>]],
            terminal_mapping = true,
            insert_mapping = true,
            auto_chdir = true,
            float_opts = {
                border = "rounded",
            },
        },
        keys = {
            { "<Esc>", [[<C-\><C-n>]], mode = "t", noremap = true, silent = true, desc = "Terminal normal mode" },
            {
                "<localleader><localleader>",
                function()
                    -- 获取当前缓冲区所在目录或工作目录
                    local term = require("toggleterm.terminal").Terminal:new({
                        direction = "float",
                        display_name = vim.fn.getcwd(),
                        on_open = function(term)
                            local cwd = vim.fn.getcwd()
                            local cmdline = term.cmd or ""
                            local name
                            if cmdline ~= "" then
                                name = cmdline
                            else
                                name = cwd
                            end
                            term.display_name = name
                        end,
                    })
                    term:toggle()
                end,
                mode = "n",
                desc = "new Terminal",
            },
            { "<localleader>s", "<cmd>TermSelect<CR>", mode = "n", desc = "choose Terminal" },
            { "<localleader>n", "<cmd>ToggleTermSetName<CR>", mode = "n", desc = "set Terminal Name" },
        },
    },
}
