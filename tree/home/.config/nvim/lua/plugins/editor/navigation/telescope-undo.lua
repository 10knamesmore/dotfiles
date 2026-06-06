-- telescope-undo - 在 Telescope 中浏览和恢复 undo 历史分支
return {
    "debugloop/telescope-undo.nvim",
    dependencies = { "nvim-telescope/telescope.nvim" },
    keys = {
        {
            "<leader>su",
            function()
                require("telescope").extensions.undo.undo({ initial_mode = "normal" })
            end,
            desc = "Undo History",
        },
    },
    config = function()
        require("telescope").load_extension("undo")
    end,
}
