-- Util 工具插件
-- persistence - Session 管理
-- plenary - 其他插件使用的库

return {
    -- persistence - Session 管理
    {
        "folke/persistence.nvim",
        event = "BufReadPre",
        opts = {},
    },

    -- plenary - 其他插件使用的库
    { "nvim-lua/plenary.nvim", lazy = true },
}
