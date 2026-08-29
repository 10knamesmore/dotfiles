return {
  -- correctly setup lspconfig
  {
    "neovim/nvim-lspconfig",
    opts = {
      -- make sure mason installs the server
      servers = {
        --- 兼容仍使用 tsserver 标识的 nvim-lspconfig。
        tsserver = {
          enabled = false,
        },
        ts_ls = {
          enabled = false,
        },
        vtsls = {
          -- explicitly add default filetypes, so that we can extend
          -- them in related extras
          filetypes = {
            "javascript",
            "javascriptreact",
            "javascript.jsx",
            "typescript",
            "typescriptreact",
            "typescript.tsx",
          },
          settings = {
            complete_function_calls = true,
            vtsls = {
              enableMoveToFileCodeAction = true,
              autoUseWorkspaceTsdk = true,
              experimental = {
                maxInlayHintLength = 30,
                completion = {
                  enableServerSideFuzzyMatch = true,
                },
              },
            },
            typescript = {
              updateImportsOnFileMove = { enabled = "always" },
              suggest = {
                completeFunctionCalls = true,
              },
              inlayHints = {
                enumMemberValues = { enabled = true },
                functionLikeReturnTypes = { enabled = true },
                parameterNames = { enabled = "literals" },
                parameterTypes = { enabled = true },
                propertyDeclarationTypes = { enabled = true },
                variableTypes = { enabled = true },
              },
            },
          },
          keys = {
            {
              "gD",
              function()
                local win = vim.api.nvim_get_current_win()
                local params = vim.lsp.util.make_position_params(win, "utf-16")
                utils.lsp.execute({
                  command = "typescript.goToSourceDefinition",
                  arguments = { params.textDocument.uri, params.position },
                  open = true,
                })
              end,
              desc = "Goto Source Definition",
            },
            {
              "gR",
              function()
                utils.lsp.execute({
                  command = "typescript.findAllFileReferences",
                  arguments = { vim.uri_from_bufnr(0) },
                  open = true,
                })
              end,
              desc = "File References",
            },
            {
              "<leader>co",
              utils.lsp.action["source.organizeImports"],
              desc = "Organize Imports",
            },
            {
              "<leader>cM",
              utils.lsp.action["source.addMissingImports.ts"],
              desc = "Add missing imports",
            },
            {
              "<leader>cu",
              utils.lsp.action["source.removeUnused.ts"],
              desc = "Remove unused imports",
            },
            {
              "<leader>cD",
              utils.lsp.action["source.fixAll.ts"],
              desc = "Fix all diagnostics",
            },
            {
              "<leader>cV",
              function()
                utils.lsp.execute({ command = "typescript.selectTypeScriptVersion" })
              end,
              desc = "Select TS workspace version",
            },
          },
        },
      },
      setup = {
        --- 兼容仍使用 tsserver 标识的 nvim-lspconfig。
        tsserver = function()
          -- disable tsserver
          return true
        end,
        ts_ls = function()
          -- disable tsserver
          return true
        end,
        vtsls = function(_, opts)
          if vim.lsp.config.denols and vim.lsp.config.vtsls then
            --- 为 denols 与 vtsls 生成互斥的根目录解析逻辑。
            ---@param server string
            local resolve = function(server)
              local markers, root_dir = vim.lsp.config[server].root_markers, vim.lsp.config[server].root_dir
              vim.lsp.config(server, {
                root_dir = function(bufnr, on_dir)
                  local is_deno = vim.fs.root(bufnr, { "deno.json", "deno.jsonc" }) ~= nil
                  if is_deno == (server == "denols") then
                    if root_dir then
                      return root_dir(bufnr, on_dir)
                    elseif type(markers) == "table" then
                      local root = vim.fs.root(bufnr, markers)
                      return root and on_dir(root)
                    end
                  end
                end,
              })
            end
            resolve("denols")
            resolve("vtsls")
          end

          Snacks.util.lsp.on({ name = "vtsls" }, function(buffer, client)
            client.commands["_typescript.moveToFileRefactoring"] = function(command, ctx)
              ---@type string, string, lsp.Range
              local action, uri, range = unpack(command.arguments)

              local function move(newf)
                client:request("workspace/executeCommand", {
                  command = command.command,
                  arguments = { action, uri, range, newf },
                })
              end

              local fname = vim.uri_to_fname(uri)
              client:request("workspace/executeCommand", {
                command = "typescript.tsserverRequest",
                arguments = {
                  "getMoveToRefactoringFileSuggestions",
                  {
                    file = fname,
                    startLine = range.start.line + 1,
                    startOffset = range.start.character + 1,
                    endLine = range["end"].line + 1,
                    endOffset = range["end"].character + 1,
                  },
                },
              }, function(_, result)
                ---@type string[]
                local files = result.body.files
                table.insert(files, 1, "Enter new path...")
                vim.ui.select(files, {
                  prompt = "Select move destination:",
                  format_item = function(f)
                    return vim.fn.fnamemodify(f, ":~:.")
                  end,
                }, function(f)
                  if f and f:find("^Enter new path") then
                    vim.ui.input({
                      prompt = "Enter move destination:",
                      default = vim.fn.fnamemodify(fname, ":h") .. "/",
                      completion = "file",
                    }, function(newf)
                      return newf and move(newf)
                    end)
                  elseif f then
                    move(f)
                  end
                end)
              end)
            end
          end)

          -- tsserver 的关键字补全不带尾随空格（rust-analyzer 那种 "pub $0" 是服务端 snippet，
          -- tsserver 只回纯文本），所以 accept 后光标紧贴关键字。这里在 accept 之后补一个空格。
          -- 别改成 blink 的 transform_items 去改 textEdit.newText：blink accept 前必发
          -- completionItem/resolve，vtsls 会重建 item 把改过的文本原样打回，改了等于没改。
          local keyword_space = {}
          for kw in
            ([[const let var function class interface type enum namespace declare abstract
            readonly static public private protected export import new extends implements in of as
            satisfies keyof typeof instanceof async await yield return throw case delete void
            if else do while for switch catch try finally]]):gmatch("%S+")
          do
            keyword_space[kw] = true
          end
          vim.api.nvim_create_autocmd("User", {
            pattern = "BlinkCmpAccept",
            callback = function(ev)
              local item = ev.data and ev.data.item
              if
                not item
                or item.kind ~= vim.lsp.protocol.CompletionItemKind.Keyword
                or not keyword_space[item.label]
                or not vim.tbl_contains(opts.filetypes, vim.bo.filetype)
              then
                return
              end
              -- 事件可能被 schedule；若用户已 Esc 回 normal mode，col 会被钳位导致空格插错字节
              if not vim.api.nvim_get_mode().mode:find("^i") then
                return
              end
              local row, col = unpack(vim.api.nvim_win_get_cursor(0))
              -- 光标后已有空格（比如在行中间替换补全）就不重复加
              if vim.api.nvim_get_current_line():sub(col + 1, col + 1) == " " then
                return
              end
              vim.api.nvim_buf_set_text(0, row - 1, col, row - 1, col, { " " })
              vim.api.nvim_win_set_cursor(0, { row, col + 1 })
            end,
          })

          -- copy typescript settings to javascript
          opts.settings.javascript =
            vim.tbl_deep_extend("force", {}, opts.settings.typescript, opts.settings.javascript or {})
        end,
      },
    },
  },

  {
    "mfussenegger/nvim-dap",
    optional = true,
    dependencies = {
      {
        "mason-org/mason.nvim",
        opts = function(_, opts)
          opts.ensure_installed = opts.ensure_installed or {}
          table.insert(opts.ensure_installed, "js-debug-adapter")
        end,
      },
    },
    config = function()
      local dap = require("dap")

      for _, adapterType in ipairs({ "node", "chrome", "msedge" }) do
        local pwaType = "pwa-" .. adapterType

        if not dap.adapters[pwaType] then
          dap.adapters[pwaType] = {
            type = "server",
            host = "localhost",
            port = "${port}",
            executable = {
              command = "js-debug-adapter",
              args = { "${port}" },
            },
          }
        end

        -- Define adapters without the "pwa-" prefix for VSCode compatibility
        if not dap.adapters[adapterType] then
          dap.adapters[adapterType] = function(cb, config)
            local nativeAdapter = dap.adapters[pwaType]

            config.type = pwaType

            if type(nativeAdapter) == "function" then
              nativeAdapter(cb, config)
            else
              cb(nativeAdapter)
            end
          end
        end
      end

      local js_filetypes = { "typescript", "javascript", "typescriptreact", "javascriptreact" }

      local vscode = require("dap.ext.vscode")
      vscode.type_to_filetypes["node"] = js_filetypes
      vscode.type_to_filetypes["pwa-node"] = js_filetypes

      for _, language in ipairs(js_filetypes) do
        if not dap.configurations[language] then
          local runtimeExecutable = nil
          if language:find("typescript") then
            runtimeExecutable = vim.fn.executable("tsx") == 1 and "tsx" or "ts-node"
          end
          dap.configurations[language] = {
            {
              type = "pwa-node",
              request = "launch",
              name = "Launch file",
              program = "${file}",
              cwd = "${workspaceFolder}",
              sourceMaps = true,
              runtimeExecutable = runtimeExecutable,
              skipFiles = {
                "<node_internals>/**",
                "node_modules/**",
              },
              resolveSourceMapLocations = {
                "${workspaceFolder}/**",
                "!**/node_modules/**",
              },
            },
            {
              type = "pwa-node",
              request = "attach",
              name = "Attach",
              processId = require("dap.utils").pick_process,
              cwd = "${workspaceFolder}",
              sourceMaps = true,
              runtimeExecutable = runtimeExecutable,
              skipFiles = {
                "<node_internals>/**",
                "node_modules/**",
              },
              resolveSourceMapLocations = {
                "${workspaceFolder}/**",
                "!**/node_modules/**",
              },
            },
          }
        end
      end
    end,
  },

  {
    "jay-babu/mason-nvim-dap.nvim",
    optional = true,
    opts = {
      -- chrome adapter is deprecated, use js-debug-adapter instead
      automatic_installation = { exclude = { "chrome" } },
    },
  },

  -- 格式化。没有这段的话 = 会落到 lsp_format="fallback"，由 vtsls/jsonls 的内置
  -- 格式化器接手 —— 它们不读项目的 .prettierrc / biome.json，团队项目里必然和 CI 打架。
  -- stop_after_first：biome 优先（有 biome.json 的项目），否则 prettierd（常驻进程更快），
  -- 最后退到 prettier。三者都不可用时 conform 静默跳过，不会中断保存。
  {
    "stevearc/conform.nvim",
    optional = true,
    ---@module "conform"
    ---@type conform.setupOpts
    opts = {
      formatters = {
        -- conform 内置的 biome 只设了 cwd 没设 require_cwd（默认 false），于是**没有
        -- biome.json 的项目也会跑 biome**，用它自带的默认风格（双引号 + trailingCommas
        -- all）盖掉项目的 .prettierrc —— 排在 stop_after_first 第一位更是让 prettier
        -- 永远轮不到。加这个开关，让 biome 只在真有 biome 配置的项目里接管。
        biome = { require_cwd = true },
      },
      formatters_by_ft = {
        javascript = { "biome", "prettierd", "prettier", stop_after_first = true },
        javascriptreact = { "biome", "prettierd", "prettier", stop_after_first = true },
        typescript = { "biome", "prettierd", "prettier", stop_after_first = true },
        typescriptreact = { "biome", "prettierd", "prettier", stop_after_first = true },
        json = { "biome", "prettierd", "prettier", stop_after_first = true },
        jsonc = { "biome", "prettierd", "prettier", stop_after_first = true },
        css = { "prettierd", "prettier", stop_after_first = true },
        scss = { "prettierd", "prettier", stop_after_first = true },
        html = { "prettierd", "prettier", stop_after_first = true },
      },
    },
  },
  {
    "mason-org/mason.nvim",
    ---@module "mason"
    ---@type MasonSettings | {ensure_installed: string[]}
    opts = { ensure_installed = { "prettier", "prettierd", "biome" } },
  },

  -- Filetype icons
  {
    "nvim-mini/mini.icons", -- URL 必须与 ui/core/mini-icons.lua 一致，否则 lazy 会重新 clone
    opts = {
      file = {
        [".eslintrc.js"] = { glyph = "󰱺", hl = "MiniIconsYellow" },
        [".node-version"] = { glyph = "", hl = "MiniIconsGreen" },
        [".prettierrc"] = { glyph = "", hl = "MiniIconsPurple" },
        [".yarnrc.yml"] = { glyph = "", hl = "MiniIconsBlue" },
        ["eslint.config.js"] = { glyph = "󰱺", hl = "MiniIconsYellow" },
        ["package.json"] = { glyph = "", hl = "MiniIconsGreen" },
        ["tsconfig.json"] = { glyph = "", hl = "MiniIconsAzure" },
        ["tsconfig.build.json"] = { glyph = "", hl = "MiniIconsAzure" },
        ["yarn.lock"] = { glyph = "", hl = "MiniIconsBlue" },
      },
    },
  },
}
