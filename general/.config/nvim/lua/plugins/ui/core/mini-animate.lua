-- animations
return {
    -- Animates many common Neovim actions, like scrolling,
    -- moving the cursor, and resizing windows.
    {
        "nvim-mini/mini.animate",
        event = "VeryLazy",
        cond = vim.g.neovide == nil,
        opts = function(_, opts)
            -- don't use animate when scrolling with the mouse
            local mouse_scrolled = false
            for _, scroll in ipairs({ "Up", "Down" }) do
                local key = "<ScrollWheel" .. scroll .. ">"
                vim.keymap.set({ "", "i" }, key, function()
                    mouse_scrolled = true
                    return key
                end, { expr = true })
            end

            local animate = require("mini.animate")
            return vim.tbl_deep_extend("force", opts, {
                cursor = {
                    enable = false, -- 用 kitty 自带的
                },
                resize = {
                    timing = animate.gen_timing.cubic({ duration = 50, unit = "total" }),
                },
                open = {
                    timing = animate.gen_timing.cubic({ duration = 50, unit = "total" }),
                },
                close = {
                    timing = animate.gen_timing.cubic({ duration = 50, unit = "total" }),
                },
                scroll = {
                    timing = animate.gen_timing.cubic({ duration = 150, unit = "total" }),
                    subscroll = animate.gen_subscroll.equal({
                        predicate = function(total_scroll)
                            if mouse_scrolled then
                                mouse_scrolled = false
                                return false
                            end
                            return total_scroll > 1
                        end,
                    }),
                },
            })
        end,
    },
}
