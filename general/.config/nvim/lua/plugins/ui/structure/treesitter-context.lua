return {
    "nvim-treesitter/nvim-treesitter-context",
    event = { "BufReadPost", "BufWritePost", "BufNewFile" },
    opts = function()
        return { enable = true, multiwindow = true, mode = "cursor", max_lines = 3 }
    end,
}
