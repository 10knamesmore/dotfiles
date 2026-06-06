return {
    {
        "rachartier/tiny-inline-diagnostic.nvim",
        event = "VeryLazy",
        priority = 1000,
        opts = {
            signs = {
                left = "",
                right = "",
                diag = "●",
                arrow = "  ",
                up_arrow = "  ",
                vertical_end = " └",
            },
            blend = {
                factor = 0.22,
            },
            transparent_background = true,
            options = {
                set_arrow_to_diag_color = true,
                multilines = {
                    enabled = true,
                    always_show = true,
                },
                show_all_diags_on_cursorline = true,
                add_messages = {
                    -- display_count = true,
                },
                show_related = {
                    enabled = true,
                    max_count = 1,
                },
                show_source = {
                    enabled = true,
                },
                virt_texts = {
                    priority = 9999,
                },
                overflow = {
                    padding = 4,
                },
            },
        },
    },
}
