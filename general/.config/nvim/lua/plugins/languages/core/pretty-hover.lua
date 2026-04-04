-- hover功能
-- <ctrl>k 开启hover
return {
    "Fildo7525/pretty_hover",
    event = "LspAttach",
    opts = {},
    keys = {
        {
            "<C-k>",
            function()
                require("pretty_hover").hover()
            end,
            desc = "lsp hover",
        },
    },
}
