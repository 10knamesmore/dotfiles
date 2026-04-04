return {
    "lewis6991/gitsigns.nvim",
    event = { "BufReadPost", "BufNewFile", "BufWritePre" },
    opts = {
        signs_staged_enable = true,
        signcolumn = false, -- Toggle with `:Gitsigns toggle_signs`
        numhl = true, -- Toggle with `:Gitsigns toggle_numhl`
        linehl = false, -- Toggle with `:Gitsigns toggle_linehl`
        word_diff = false, -- Toggle with `:Gitsigns toggle_word_diff`
        watch_gitdir = {
            follow_files = true,
        },
        on_attach = function(buffer)
            local gs = require("gitsigns")
            local gitsigns_cache = require("gitsigns.cache").cache

            local function map(mode, l, r, desc)
                vim.keymap.set(mode, l, r, { buffer = buffer, desc = desc, silent = true })
            end

            local function get_staged_hunks(bufnr)
                local bcache = gitsigns_cache[bufnr]
                return bcache and bcache:get_hunks(false, true) or {}
            end

            --- @param direction 'first'|'last'|'next'|'prev'
            local function nav_hunk_with_staged_fallback(direction)
                local unstaged_hunks = gs.get_hunks(buffer) or {}
                local target = vim.tbl_isempty(unstaged_hunks) and "staged" or "unstaged"
                gs.nav_hunk(direction, { target = target })
            end

            map("n", "<leader>gd", function()
                if vim.wo.diff then
                    vim.cmd("diffoff!")
                    for _, buf in ipairs(vim.api.nvim_list_bufs()) do
                        local name = vim.api.nvim_buf_get_name(buf)
                        if name:match("^gitsigns:") then
                            vim.api.nvim_buf_delete(buf, { force = true })
                        end
                    end
                else
                    local bufnr = vim.api.nvim_get_current_buf()
                    local unstaged_hunks = gs.get_hunks(bufnr) or {}
                    if not vim.tbl_isempty(unstaged_hunks) then
                        gs.diffthis()
                        return
                    end

                    local staged_hunks = get_staged_hunks(bufnr)
                    if vim.tbl_isempty(staged_hunks) then
                        vim.notify("No changes to diff", vim.log.levels.INFO)
                        return
                    end

                    gs.diffthis("HEAD")
                end
            end, "Diff This")

            map("n", "<leader>gk", function()
                gs.preview_hunk()
            end, "diff line")

            map("n", "]h", function()
                if vim.wo.diff then
                    vim.cmd.normal({ "]c", bang = true })
                else
                    nav_hunk_with_staged_fallback("next")
                end
            end, "Next Hunk")

            map("n", "[h", function()
                if vim.wo.diff then
                    vim.cmd.normal({ "[c", bang = true })
                else
                    nav_hunk_with_staged_fallback("prev")
                end
            end, "Prev Hunk")

            map({ "n", "v" }, "<leader>ga", function()
                if vim.fn.mode() == "v" or vim.fn.mode() == "V" then
                    local start_line = vim.fn.line("v")
                    local end_line = vim.fn.line(".")
                    if start_line > end_line then
                        start_line, end_line = end_line, start_line
                    end
                    gs.stage_hunk({ start_line, end_line })
                else
                    gs.stage_hunk()
                end
            end, "Stage Hunk")

            map("n", "]H", function()
                nav_hunk_with_staged_fallback("last")
            end, "Last Hunk")

            map("n", "[H", function()
                nav_hunk_with_staged_fallback("first")
            end, "First Hunk")
        end,
    },
}
