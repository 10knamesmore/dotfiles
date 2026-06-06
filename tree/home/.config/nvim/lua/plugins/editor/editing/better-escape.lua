-- 输入jk或kj时立刻输入, 匹配到后一个时撤回并Esc
return {
    "max397574/better-escape.nvim",
    event = "InsertEnter",
    config = function()
        return require("better_escape").setup({
            timeout = 500, -- after `timeout` passes, you can press the escape key and the plugin will ignore it
            default_mappings = false, -- setting this to false removes all the default mappings
            mappings = {
                i = {
                    j = {
                        k = "<Esc>",
                    },
                    k = {
                        j = "<Esc>",
                    },
                },
            },
        })
    end,
}
