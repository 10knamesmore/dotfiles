--- 打开大文件时禁用某些功能以提高性能
return {
    "LunarVim/bigfile.nvim",
    lazy = true,
    opts = {
        filesize = 5, -- size of the file in MiB, the plugin round file sizes to the closest MiB
        pattern = { "*" }, -- autocmd pattern or function see <### Overriding the detection of big files>
        features = { -- features to disable
            "indent_blankline",
            "illuminate",
            "lsp",
            "treesitter",
            "syntax",
            "matchparen",
            "vimopts",
            "filetype",
        },
    },
}
