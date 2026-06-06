return {
    "nvim-mini/mini.align",
    version = false,
    event = { "BufReadPost", "BufWritePost", "BufNewFile" },
    opts = function()
        -- No need to copy this inside `setup()`. Will be used automatically.
        local opts = {
            -- Module mappings. Use `''` (empty string) to disable one.
            mappings = {
                start = "",
                start_with_preview = "ga",
            },

            modifiers = {
                j = function(_, opts)
                    local next_side = {
                        left = "center",
                        center = "right",
                        right = "none",
                        none = "left",
                    }
                    opts.justify_side = next_side[opts.justify_side] or "left"
                end,

                [":"] = function(steps, opts)
                    opts.split_pattern = ":"
                    opts.justify_side = "right"
                    table.insert(steps.pre_justify, require("mini.align").gen_step.trim("both"))
                    opts.merge_delimiter = " "
                end,
            },
            -- Modifiers changing alignment steps and/or options
            -- modifiers = {
            --   -- Main option modifiers
            --   ['s'] = --<function: enter split pattern>,
            --   ['j'] = --<function: choose justify side>,
            --   ['m'] = --<function: enter merge delimiter>,
            --
            --   -- Modifiers adding pre-steps
            --   ['f'] = --<function: filter parts by entering Lua expression>,
            --   ['i'] = --<function: ignore some split matches>,
            --   ['p'] = --<function: pair parts>,
            --   ['t'] = --<function: trim parts>,
            --
            --   -- Delete some last pre-step
            --   ['<BS>'] = --<function: delete some last pre-step>,
            --
            --   -- Special configurations for common splits
            --   ['='] = --<function: enhanced setup for '='>,
            --   [','] = --<function: enhanced setup for ','>,
            --   ['|'] = --<function: enhanced setup for '|'>,
            --   [' '] = --<function: enhanced setup for ' '>,
            -- },

            -- Default options controlling alignment process
            options = {
                split_pattern = "",
                justify_side = "left",
                merge_delimiter = "",
            },

            -- Default steps performing alignment (if `nil`, default is used)
            steps = {
                pre_split = {},
                split = nil,
                pre_justify = {},
                justify = nil,
                pre_merge = {},
                merge = nil,
            },

            -- Whether to disable showing non-error feedback
            -- This also affects (purely informational) helper messages shown after
            -- idle time if user input is required.
            silent = false,
        }

        return opts
    end,
}
