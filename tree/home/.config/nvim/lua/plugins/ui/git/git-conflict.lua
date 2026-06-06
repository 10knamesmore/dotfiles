return {
    "akinsho/git-conflict.nvim",
    version = "*",
    event = "BufReadPre",
    opts = {
        default_mappings = false,
        disable_diagnostics = true,
        highlights = {
            incoming = "DiffAdd",
            current = "DiffText",
        },
    },
    keys = {
        -- { "<leader>gco", "<cmd>GitConflictChooseOurs<cr>", desc = "Conflict Choose Ours" },
        -- { "<leader>gct", "<cmd>GitConflictChooseTheirs<cr>", desc = "Conflict Choose Theirs" },
        -- { "<leader>gcb", "<cmd>GitConflictChooseBoth<cr>", desc = "Conflict Choose Both" },
        -- { "<leader>gc0", "<cmd>GitConflictChooseNone<cr>", desc = "Conflict Choose None" },
        { "[x", "<cmd>GitConflictPrevConflict<cr>", desc = "Conflict Prev" },
        { "]x", "<cmd>GitConflictNextConflict<cr>", desc = "Conflict Next" },
    },
}
