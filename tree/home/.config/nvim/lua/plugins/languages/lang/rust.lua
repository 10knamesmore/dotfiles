return {
  -- Cargo.toml 补全/跳转
  {
    "Saecki/crates.nvim",
    event = { "BufRead Cargo.toml" },
    ---@module "crates"
    ---@type crates.UserConfig
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
    ---@module "nvim-treesitter"
    ---@type TSConfig | {ensure_installed: string[]}
    opts = { ensure_installed = { "rust", "ron" } },
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
    ---@module "rustaceanvim"
    --- @type rustaceanvim.Opts
    opts = {
      dap = {
        adapter = false,
        autoload_configurations = false,
      },
      server = {
        -- 从项目根目录启动 rustup 代理，避免 Mason 的 PATH 优先级改变工具链。
        cmd = function()
          return function(dispatchers, client_config)
            local cargo_bin = vim.fn.expand("~/.cargo/bin")
            return vim.lsp.rpc.start(
              { cargo_bin .. "/rust-analyzer", "--log-file", client_config.logfile },
              dispatchers,
              {
                cwd = client_config.root_dir,
                env = { PATH = cargo_bin .. ":" .. vim.env.PATH },
              }
            )
          end
        end,
        -- see https://rust-analyzer.github.io/book/configuration
        default_settings = {
          -- rust-analyzer language server configuration
          ["rust-analyzer"] = {
            -- 普通 helper 即使位于 #[cfg(test)] 模块中也不会被归为测试引用。
            references = {
              excludeTests = true,
              excludeImports = true,
            },
            cargo = {
              -- 额外 features 由项目 .nvim.lua 设置；未指定时使用 Cargo 默认 features。
              loadOutDirsFromCheck = true,
              buildScripts = {
                enable = true,
              },
            },
            checkOnSave = true,
            check = {
              command = "clippy",
            },
            diagnostics = {
              enable = true,
              -- schema 里是复数 warningsAsHint（rust-analyzer --print-config-schema）
              warningsAsHint = { "inactive-code" },
            },
            lens = {
              references = {
                adt = {
                  enable = true,
                },
                method = {
                  enable = true,
                },
                trait = {
                  enable = true,
                },
                enumVariant = {
                  enable = true,
                },
              },
            },
            inlayHints = {
              closureCaptureHints = {
                enable = true,
              },
              expressionAdjustmentHints = {
                enable = "reborrow",
              },
              lifetimeElisionHints = {
                enable = "skip_trivial",
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
