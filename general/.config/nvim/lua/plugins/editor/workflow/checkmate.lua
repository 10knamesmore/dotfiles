-- 项目 TODO 管理
return {
    {
        "folke/which-key.nvim",
        opts = {
            spec = {
                { "<leader>t", group = "TODO Manage", icon = "" },
            },
        },
        keys = {
            {
                "t",
                function()
                    local project_dir = utils.path.get_root()
                    local file = project_dir .. "/" .. ".todo.md"
                    local current_file_path = vim.api.nvim_buf_get_name(0)
                    if current_file_path == file then
                        return
                    end

                    Snacks.scratch.open({
                        file = file,
                        name = "TODO: " .. file,
                    })
                end,
                desc = "Project TODO",
            },
        },
    },
    {
        "bngarren/checkmate.nvim",
        ft = "markdown", -- Lazy loads for Markdown files matching patterns in 'files'
        opts = function(_, _)
            ---@type checkmate.Config
            return {
                enabled = true,
                notify = true,
                -- Default file matching:
                --  - Any `todo` or `TODO` file, including with `.md` extension
                --  - Any `.todo` extension (can be ".todo" or ".todo.md")
                -- To activate Checkmate, the filename must match AND the filetype must be "markdown"
                files = {
                    "todo",
                    "TODO",
                    "todo.md",
                    "TODO.md",
                    "*.todo",
                    "*.todo.md",
                },
                log = {
                    level = "warn",
                    use_file = true,
                },
                -- Default keymappings
                keys = {
                    ["<leader>tn"] = {
                        rhs = function()
                            require("checkmate").create()
                        end,
                        desc = "New",
                        modes = { "n" },
                    },
                    ["<leader>ta"] = {
                        rhs = function()
                            require("checkmate").archive()
                        end,
                        desc = "Archive",
                        modes = { "n" },
                    },
                    ["<leader>tt"] = {
                        rhs = function()
                            require("checkmate").select_todo()
                        end,
                        desc = "Set State",
                        modes = { "n", "v" },
                    },
                    ["<C-a>"] = {
                        rhs = function()
                            require("checkmate").toggle_metadata("done")
                        end,
                        desc = "Cycle State",
                        modes = { "n", "v" },
                    },
                },
                default_list_marker = "-",
                ui = {
                    picker = "telescope",
                },
                todo_states = {
                    -- we don't need to set the `markdown` field for `unchecked` and `checked` as these can't be overriden
                    ---@diagnostic disable-next-line: missing-fields
                    unchecked = {
                        marker = "□",
                        order = 1,
                    },
                    ---@diagnostic disable-next-line: missing-fields
                    checked = {
                        marker = "✔",
                        order = 2,
                    },
                },
                style = {}, -- override defaults
                enter_insert_after_new = true, -- Should enter INSERT mode after `:Checkmate create` (new todo)
                list_continuation = {
                    enabled = true,
                    split_line = true,
                    keys = {
                        ["<CR>"] = function()
                            require("checkmate").create({
                                position = "below",
                                indent = false,
                            })
                        end,
                        ["<S-CR>"] = function()
                            require("checkmate").create({
                                position = "below",
                                indent = true,
                            })
                        end,
                    },
                },
                smart_toggle = {
                    enabled = true,
                    include_cycle = false,
                    check_down = "direct_children",
                    uncheck_down = "none",
                    check_up = "direct_children",
                    uncheck_up = "direct_children",
                },
                show_todo_count = true,
                todo_count_position = "eol",
                todo_count_recursive = true,
                use_metadata_keymaps = true,
                metadata = {
                    -- Example: A @priority tag that has dynamic color based on the priority value
                    priority = {
                        style = function(context)
                            local value = context.value:lower()
                            if value == "high" then
                                return { fg = "#ff5555", bold = true }
                            elseif value == "medium" then
                                return { fg = "#ffb86c" }
                            elseif value == "low" then
                                return { fg = "#8be9fd" }
                            else -- fallback
                                return { fg = "#8be9fd" }
                            end
                        end,
                        get_value = function()
                            return "medium" -- Default priority
                        end,
                        choices = function()
                            return { "low", "medium", "high" }
                        end,
                        key = "<leader>tp",
                        sort_order = 10,
                        jump_to_on_insert = "value",
                        select_on_insert = true,
                    },
                    -- Example: A @started tag that uses a default date/time string when added
                    started = {
                        aliases = { "init" },
                        style = { fg = "#9fd6d5" },
                        get_value = function()
                            return tostring(os.date("%m/%d/%y %H:%M"))
                        end,
                        key = "<leader>ts",
                        sort_order = 20,
                    },
                    -- Example: A @done tag that also sets the todo item state when it is added and removed
                    done = {
                        aliases = { "completed", "finished" },
                        style = { fg = "#96de7a" },
                        get_value = function()
                            return tostring(os.date("%m/%d/%y %H:%M"))
                        end,
                        -- key = "<leader>td",
                        on_add = function(todo)
                            require("checkmate").set_todo_state(todo, "checked")
                            require("checkmate").remove_metadata("blocked")
                        end,
                        on_remove = function(todo)
                            require("checkmate").set_todo_state(todo, "unchecked")
                        end,
                        sort_order = 100,
                    },
                    blocked = {
                        style = { fg = "#ff5555", underline = true, bold = true },
                        get_value = function()
                            return "By:"
                        end,
                        key = "<leader>tb",
                        sort_order = 70,
                    },
                },
                archive = {
                    heading = {
                        title = "Archive",
                        level = 2, -- e.g. ##
                    },
                    parent_spacing = 0, -- no extra lines between archived todos
                    newest_first = true,
                },
                linter = {
                    enabled = true,
                },
            }
        end,
    },
}
