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
    ---@return rustaceanvim.Opts
    opts = function()
      local use_ra_diagnostics = vim.g.rust_diagnostics == "rust-analyzer"
      return {
        dap = {
          adapter = false,
          autoload_configurations = false,
        },
        server = {
          on_init = function(client)
            -- RA 局部高亮会漏掉外层过程宏上下文，Neovim 合并 full 结果时又会保留错误 token。
            -- 仅禁用 range 请求，保留 full/delta 语义高亮。
            client.server_capabilities.semanticTokensProvider.range = false
          end,
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
              checkOnSave = use_ra_diagnostics,
              check = {
                command = "clippy",
              },
              -- 补全文档显示完整函数签名（含 pub/泛型/where 子句；默认只给参数+返回类型）
              completion = {
                fullFunctionSignatures = {
                  enable = true,
                },
              },
              diagnostics = {
                enable = use_ra_diagnostics,
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
                -- 无字段枚举显示判别值（= 0/1/…）
                discriminantHints = {
                  enable = "fieldless",
                },
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
      }
    end,
    config = function(_, opts)
      vim.g.rustaceanvim = vim.tbl_deep_extend("keep", vim.g.rustaceanvim or {}, opts or {})
    end,
  },

  -- Correctly setup lspconfig for Rust 🚀
  {
    "neovim/nvim-lspconfig",
    opts = function(_, opts)
      opts.servers.bacon_ls = {
        enabled = vim.g.rust_diagnostics == "bacon-ls",
        cmd = { vim.fn.stdpath("data") .. "/mason/bin/bacon-ls" },
        cmd_env = { PATH = vim.fn.expand("~/.cargo/bin") .. ":" .. vim.env.PATH },
        root_dir = function(bufnr, on_dir)
          vim.lsp.config.rust_analyzer.root_dir(bufnr, on_dir)
        end,
        settings = {
          bacon_ls = {
            backend = "cargo",
            cargo = {
              command = "clippy",
              allTargets = true,
              extraArgs = { "--workspace", "--no-deps" },
            },
          },
        },
      }
      opts.servers.rust_analyzer = { enabled = false }
    end,
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
