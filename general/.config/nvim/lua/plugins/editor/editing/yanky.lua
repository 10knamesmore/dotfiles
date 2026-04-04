-- <leader>sp 打开复制的历史
-- p 复制到光标后
-- P 复制到光标前
return {
    {
        "gbprod/yanky.nvim",
        desc = "Better Yank/Paste",
        event = { "BufReadPost", "BufWritePost", "BufNewFile" },
        opts = {
            highlight = { timer = 150 },
            picker = {
                select = {
                    action = nil,
                },
                telescope = {
                    use_default_mappings = true,
                    mappings = nil,
                },
            },
        },
        config = function(_, opts)
            require("yanky").setup(opts)
            vim.keymap.set({ "n", "v" }, "<leader>sy", function()
                require("telescope").extensions.yank_history.yank_history({ initial_mode = "normal" })
            end, { noremap = true, silent = true, desc = "Yank History" })
        end,
        keys = {
            { "y", "<Plug>(YankyYank)", mode = { "n", "x" }, desc = "Yank Text" },
            { "p", "<Plug>(YankyPutAfter)", mode = { "n", "x" }, desc = "Put Text After Cursor" },
            { "P", "<Plug>(YankyPutBefore)", mode = { "n", "x" }, desc = "Put Text Before Cursor" },
        },
    },
    {
        "folke/which-key.nvim",
        opts = {
            spec = {
                {
                    { "<leader>y", group = "yank", icon = "" },
                },
            },
        },
    },
    {
        "10knamesmore/yank-more.nvim",
        event = { "BufReadPost", "BufWritePost", "BufNewFile" },
        opts = {
            highlight = {
                timer = 150,
            },
            diagnostic = {
                no_diag_action = "notify", -- "yank" 或 "notify"
            },
            mappings = {
                yank_filename = { key = "gy", mode = "n" },
                yank_filepath = { key = "gY", mode = "n" },
                yank_location = { key = "<leader>yl", mode = { "n", "x" } },
                yank_all = { key = "<leader>yy", mode = "n" },
                yank_filepath_and_content = {
                    key = "<leader>yY",
                    mode = "n",
                    desc = "Yank Filepath + Content",
                },
            },
        },
    },
}
