return {
    -- Cargo.toml 补全/跳转
    {
        "Saecki/crates.nvim",
        event = { "BufRead Cargo.toml" },
        opts = {
            completion = {
                crates = {
                    enabled = true,
                },
            },
            lsp = {
                enabled = true,
                actions = true,
                completion = true,
                hover = true,
            },
        },
    },

    -- Add Rust & related to treesitter
    {
        "nvim-treesitter/nvim-treesitter",
        opts = { ensure_installed = { "rust", "ron" } },
    },

    -- Ensure Rust debugger is installed
    {
        "mason-org/mason.nvim",
        optional = true,
        opts = function(_, opts)
            opts.ensure_installed = opts.ensure_installed or {}
            vim.list_extend(opts.ensure_installed, { "codelldb", "rust-analyzer" })
        end,
    },

    {
        "mrcjkb/rustaceanvim",
        ft = { "rust" },
        keys = {
            {
                "<leader>ce",
                function()
                    vim.cmd.RustLsp("expandMacro")
                end,
                desc = "Expand Macro",
                ft = "rust",
            },
        },
        --- @type rustaceanvim.Opts
        opts = {
            server = {
                -- see https://rust-analyzer.github.io/book/configuration
                default_settings = {
                    -- rust-analyzer language server configuration
                    ["rust-analyzer"] = {
                        cargo = {
                            allFeatures = true,
                            loadOutDirsFromCheck = true,
                            buildScripts = {
                                enable = true,
                            },
                        },
                        checkOnSave = true,
                        diagnostics = {
                            enable = true,
                            warningAsHint = { "inactive-code" },
                        },
                        inlayHints = {
                            closureCaptureHints = {
                                enable = true,
                            },
                            -- genericParameterHints = { type = { enable = true } },
                        },
                        procMacro = {
                            enable = true,
                        },
                        files = {
                            exclude = {
                                ".direnv",
                                ".git",
                                ".jj",
                                ".github",
                                ".gitlab",
                                "node_modules",
                                "target",
                                "venv",
                                ".venv",
                            },
                            -- Avoid Roots Scanned hanging, see https://github.com/rust-lang/rust-analyzer/issues/12613#issuecomment-2096386344
                            watcher = "client",
                        },
                    },
                },
            },
        },
        config = function(_, opts)
            vim.g.rustaceanvim = vim.tbl_deep_extend("keep", vim.g.rustaceanvim or {}, opts or {})
            if vim.fn.executable("rust-analyzer") == 0 then
                vim.notify(
                    "**rust-analyzer** not found in PATH, please install it.\nhttps://rust-analyzer.github.io/",
                    vim.log.levels.WARN
                )
            end
        end,
    },

    -- Correctly setup lspconfig for Rust 🚀
    {
        "neovim/nvim-lspconfig",
        opts = {
            servers = {
                rust_analyzer = { enabled = false },
            },
        },
    },

    {
        "nvim-neotest/neotest",
        optional = true,
        opts = {
            adapters = {
                ["rustaceanvim.neotest"] = {},
            },
        },
    },
}
