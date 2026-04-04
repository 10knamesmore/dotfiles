return vim.g.copilot_enabled == false and {}
    or {
        -- copilot
        {
            "zbirenbaum/copilot.lua",
            cmd = "Copilot",
            build = ":Copilot auth",
            event = "BufReadPost",
            opts = {
                suggestion = {
                    enabled = true,
                    auto_trigger = true,
                    hide_during_completion = false,
                    keymap = {
                        accept = false, -- handled by nvim-cmp / blink.cmp
                        next = "<M-]>",
                        prev = "<M-[>",
                    },
                },
                panel = { enabled = false },
                filetypes = {
                    markdown = true,
                    help = true,
                },
            },
        },

        -- copilot-language-server
        {
            "neovim/nvim-lspconfig",
            opts = {
                servers = {
                    -- copilot.lua only works with its own copilot lsp server
                    copilot = { enabled = false },
                },
            },
        },

        -- lualine
        -- {
        --     "nvim-lualine/lualine.nvim",
        --     optional = true,
        --     event = "VeryLazy",
        --     opts = function(_, opts)
        --         -- 防御性初始化
        --         opts.sections = opts.sections or {}
        --         opts.sections.lualine_x = opts.sections.lualine_x or {}
        --
        --         table.insert(
        --             opts.sections.lualine_x,
        --             2,
        --             utils.lualine.status(utils.config.icons.kinds.Copilot, function()
        --                 local clients = package.loaded["copilot"] and vim.lsp.get_clients({ name = "copilot", bufnr = 0 })
        --                     or {}
        --                 if #clients > 0 then
        --                     local status = require("copilot.status").data.status
        --                     return (status == "InProgress" and "pending") or (status == "Warning" and "error") or "ok"
        --                 end
        --             end)
        --         )
        --     end,
        -- },

        -- copilot cmp source
        {
            "hrsh7th/nvim-cmp",
            optional = true,
            dependencies = { -- this will only be evaluated if nvim-cmp is enabled
                {
                    "zbirenbaum/copilot-cmp",
                    opts = {},
                    config = function(_, opts)
                        local copilot_cmp = require("copilot_cmp")
                        copilot_cmp.setup(opts)
                        -- attach cmp source whenever copilot attaches
                        -- fixes lazy-loading issues with the copilot cmp source
                        Snacks.util.lsp.on({ name = "copilot" }, function()
                            copilot_cmp._on_insert_enter({})
                        end)
                    end,
                    specs = {
                        {
                            "hrsh7th/nvim-cmp",
                            optional = true,
                            ---@param opts cmp.ConfigSchema
                            opts = function(_, opts)
                                table.insert(opts.sources, 1, {
                                    name = "copilot",
                                    group_index = 1,
                                    priority = 100,
                                })
                            end,
                        },
                    },
                },
            },
        },
        {
            "saghen/blink.cmp",
            optional = true,
            dependencies = { "fang2hou/blink-copilot" },
            opts = {
                sources = {
                    default = { "copilot" },
                    providers = {
                        copilot = {
                            name = "copilot",
                            module = "blink-copilot",
                            score_offset = 100,
                            async = true,
                        },
                    },
                },
            },
        },
    }
