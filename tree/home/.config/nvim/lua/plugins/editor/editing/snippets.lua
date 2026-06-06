-- nvim-snippets - 代码片段
return {
    "garymjr/nvim-snippets",
    lazy = true,
    opts = {
        friendly_snippets = false,
        search_paths = { vim.fn.stdpath("config") .. "/snippets" },
    },
}
