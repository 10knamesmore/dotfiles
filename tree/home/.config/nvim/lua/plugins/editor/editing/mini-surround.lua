return {
    "echasnovski/mini.surround",
    event = { "BufReadPost", "BufNewFile" },
    opts = {
        mappings = {
            add = "ys",
            delete = "ds",
            replace = "cs",
            find = "",
            find_left = "",
            highlight = "",
            update_n_lines = "",
        },
        search_method = "cover_or_next",
    },
    keys = {
        { "ys", desc = "Add surrounding", mode = { "n", "v" } },
        { "yss", desc = "Add surrounding to current line" },
        { "ds", desc = "Delete surrounding" },
        { "cs", desc = "Change surrounding" },
    },
}
