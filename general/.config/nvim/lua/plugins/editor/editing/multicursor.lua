-- multicursor.nvim - 纯 Lua 多光标编辑
return {
    "jake-stewart/multicursor.nvim",
    event = "VeryLazy",
    config = function()
        local mc = require("multicursor-nvim")
        mc.setup()

        local map = vim.keymap.set

        -- 添加/跳过光标（匹配当前词）
        map({ "n", "x" }, "<C-n>", function() mc.matchAddCursor(1) end, { desc = "Add cursor on next match" })
        map({ "n", "x" }, "<C-p>", function() mc.matchAddCursor(-1) end, { desc = "Add cursor on prev match" })
        map({ "n", "x" }, "<C-S-n>", function() mc.matchAllAddCursors() end, { desc = "Add cursors on all matches" })
        map({ "n", "x" }, "q", function() mc.matchSkipCursor(1) end, { desc = "Skip match, select next" })
        map({ "n", "x" }, "Q", function() mc.matchSkipCursor(-1) end, { desc = "Skip match, select prev" })

        -- 上下添加光标
        map({ "n", "x" }, "<C-j>", function() mc.lineAddCursor(1) end, { desc = "Add cursor below" })
        map({ "n", "x" }, "<C-Up>", function() mc.lineAddCursor(-1) end, { desc = "Add cursor above" })

        -- 多光标模式内操作
        map("n", "<Esc>", function()
            if not mc.cursorsEnabled() then
                mc.enableCursors()
            elseif mc.hasCursors() then
                mc.clearCursors()
            else
                vim.cmd("nohlsearch")
            end
        end)

        -- 对齐光标列
        map("n", "<leader>ma", function() mc.alignCursors() end, { desc = "Align cursors" })

        -- 在每个选中行添加光标（visual 模式）
        map("x", "I", mc.insertVisual, { desc = "Insert at each line" })
        map("x", "A", mc.appendVisual, { desc = "Append at each line" })

        -- 高亮颜色
        vim.api.nvim_set_hl(0, "MultiCursorCursor", { link = "Cursor" })
        vim.api.nvim_set_hl(0, "MultiCursorVisual", { link = "Visual" })
        vim.api.nvim_set_hl(0, "MultiCursorSign", { link = "SignColumn" })
        vim.api.nvim_set_hl(0, "MultiCursorMatchPreview", { link = "Search" })
        vim.api.nvim_set_hl(0, "MultiCursorDisabledCursor", { link = "Visual" })
        vim.api.nvim_set_hl(0, "MultiCursorDisabledVisual", { link = "Visual" })
        vim.api.nvim_set_hl(0, "MultiCursorDisabledSign", { link = "SignColumn" })
    end,
}
