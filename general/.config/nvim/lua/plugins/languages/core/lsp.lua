return {
    -- lspconfig
    --
    {
        "neovim/nvim-lspconfig",
        event = { "BufReadPost", "BufNewFile", "BufWritePre" },
        dependencies = {
            "mason.nvim",
            { "mason-org/mason-lspconfig.nvim", config = function() end },
            {
                "folke/lazydev.nvim",
                ft = "lua",
                cmd = "LazyDev",
                opts = {
                    library = {
                        { path = "${3rd}/luv/library", words = { "vim%.uv" } },
                        { path = "snacks.nvim", words = { "Snacks" } },
                        { path = "lazy.nvim", words = { "LazyVim" } },
                    },
                },
            },
        },
        opts_extend = { "servers.*.keys" },
        opts = function()
            local Util = require("utils")
            ---@class PluginLspOpts
            local ret = {
                -- options for vim.diagnostic.config()
                ---@type vim.diagnostic.Opts
                diagnostics = {
                    underline = true,
                    update_in_insert = false,
                    virtual_text = false, -- 用户自定义：关闭 virtual_text
                    severity_sort = true,
                    signs = {
                        text = {
                            [vim.diagnostic.severity.ERROR] = Util.icons.diagnostics.Error,
                            [vim.diagnostic.severity.WARN] = Util.icons.diagnostics.Warn,
                            [vim.diagnostic.severity.HINT] = Util.icons.diagnostics.Hint,
                            [vim.diagnostic.severity.INFO] = Util.icons.diagnostics.Info,
                        },
                    },
                    globals = {
                        "vim",
                    },
                },
                inlay_hints = {
                    enabled = true,
                    exclude = { "vue" },
                },
                codelens = {
                    enabled = true,
                },
                folds = {
                    enabled = true,
                },
                format = {
                    formatting_options = nil,
                    timeout_ms = nil,
                },
                ---@type table<string, lazyvim.lsp.Config|boolean>
                servers = {
                    ["*"] = {
                        capabilities = {
                            workspace = {
                                fileOperations = {
                                    didRename = true,
                                    willRename = true,
                                },
                            },
                        },
                        keys = {
                            {
                                "<leader>cl",
                                function()
                                    Snacks.picker.lsp_config()
                                end,
                                desc = "Lsp Info",
                            },
                            -- { "<c-k>", function() return vim.lsp.buf.signature_help() end, mode = "i", desc = "Signature Help", has = "signatureHelp" }, //有了pretty hover
                            -- <leader>ca 由 actions-preview.nvim 处理 (codeaction.lua)
                            -- {
                            --     "<leader>ca",
                            --     vim.lsp.buf.code_action,
                            --     desc = "Code Action",
                            --     mode = { "n", "x" },
                            --     has = "codeAction",
                            -- },
                            {
                                "<leader>cc",
                                vim.lsp.codelens.run,
                                desc = "Run Codelens",
                                mode = { "n", "x" },
                                has = "codeLens",
                            },
                            {
                                "<leader>cC",
                                vim.lsp.codelens.refresh,
                                desc = "Refresh & Display Codelens",
                                mode = { "n" },
                                has = "codeLens",
                            },
                            {
                                "<leader>cR",
                                function()
                                    Snacks.rename.rename_file()
                                end,
                                desc = "Rename File",
                                mode = { "n" },
                                has = { "workspace/didRenameFiles", "workspace/willRenameFiles" },
                            },
                            { "<leader>cr", vim.lsp.buf.rename, desc = "Rename", has = "rename" },
                            { "<leader>cA", Util.lsp.action.source, desc = "Source Action", has = "codeAction" },
                            {
                                "]]",
                                function()
                                    Snacks.words.jump(vim.v.count1)
                                end,
                                has = "documentHighlight",
                                desc = "Next Reference",
                                enabled = function()
                                    return Snacks.words.is_enabled()
                                end,
                            },
                            {
                                "[[",
                                function()
                                    Snacks.words.jump(-vim.v.count1)
                                end,
                                has = "documentHighlight",
                                desc = "Prev Reference",
                                enabled = function()
                                    return Snacks.words.is_enabled()
                                end,
                            },
                            {
                                "<a-n>",
                                function()
                                    Snacks.words.jump(vim.v.count1, true)
                                end,
                                has = "documentHighlight",
                                desc = "Next Reference",
                                enabled = function()
                                    return Snacks.words.is_enabled()
                                end,
                            },
                            {
                                "<a-p>",
                                function()
                                    Snacks.words.jump(-vim.v.count1, true)
                                end,
                                has = "documentHighlight",
                                desc = "Prev Reference",
                                enabled = function()
                                    return Snacks.words.is_enabled()
                                end,
                            },
                        },
                    },
                    stylua = { enabled = false },
                    systemd_ls = { enabled = false }, -- 用户自定义：禁用 systemd_ls
                    lua_ls = {
                        settings = {
                            Lua = {
                                workspace = {
                                    checkThirdParty = false,
                                },
                                codeLens = {
                                    enable = true,
                                },
                                completion = {
                                    callSnippet = "Replace",
                                },
                                doc = {
                                    privateName = { "^_" },
                                },
                                hint = {
                                    enable = true,
                                    setType = false,
                                    paramType = true,
                                    paramName = "Disable",
                                    semicolon = "Disable",
                                    arrayIndex = "Disable",
                                },
                            },
                        },
                    },
                },
                ---@type table<string, fun(server:string, opts: vim.lsp.Config):boolean?>
                setup = {},
            }
            return ret
        end,
        ---@param opts PluginLspOpts
        config = vim.schedule_wrap(function(_, opts)
            local Util = require("utils")

            -- setup autoformat
            Util.format.register(Util.lsp.formatter())

            -- setup keymaps
            for server, server_opts in pairs(opts.servers) do
                if type(server_opts) == "table" and server_opts.keys then
                    Util.lsp.keymaps.set({ name = server ~= "*" and server or nil }, server_opts.keys)
                end
            end

            -- Inlay hints need a direct LspAttach check; relying only on capability
            -- observers can miss buffers that were already attached.
            if opts.inlay_hints.enabled then
                local function enable_inlay_hints(buffer, client)
                    if
                        vim.api.nvim_buf_is_valid(buffer)
                        and vim.bo[buffer].buftype == ""
                        and not vim.tbl_contains(opts.inlay_hints.exclude, vim.bo[buffer].filetype)
                        and client
                        and client:supports_method("textDocument/inlayHint")
                    then
                        vim.lsp.inlay_hint.enable(true, { bufnr = buffer })
                    end
                end

                vim.api.nvim_create_autocmd("LspAttach", {
                    callback = function(event)
                        local client = vim.lsp.get_client_by_id(event.data.client_id)
                        enable_inlay_hints(event.buf, client)
                    end,
                })

                for _, client in ipairs(vim.lsp.get_clients()) do
                    for buffer in pairs(client.attached_buffers or {}) do
                        enable_inlay_hints(buffer, client)
                    end
                end
            end

            -- folds
            if opts.folds.enabled then
                Snacks.util.lsp.on({ method = "textDocument/foldingRange" }, function()
                    if Util.set_default("foldmethod", "expr") then
                        Util.set_default("foldexpr", "v:lua.vim.lsp.foldexpr()")
                    end
                end)
            end

            -- code lens
            if opts.codelens.enabled and vim.lsp.codelens then
                Snacks.util.lsp.on({ method = "textDocument/codeLens" }, function(buffer)
                    vim.lsp.codelens.refresh()
                    vim.api.nvim_create_autocmd({ "BufEnter", "CursorHold", "InsertLeave" }, {
                        buffer = buffer,
                        callback = vim.lsp.codelens.refresh,
                    })
                end)
            end

            -- diagnostics
            if type(opts.diagnostics.virtual_text) == "table" and opts.diagnostics.virtual_text.prefix == "icons" then
                opts.diagnostics.virtual_text.prefix = function(diagnostic)
                    local icons = Util.config.icons.diagnostics
                    for d, icon in pairs(icons) do
                        if diagnostic.severity == vim.diagnostic.severity[d:upper()] then
                            return icon
                        end
                    end
                    return "●"
                end
            end
            vim.diagnostic.config(vim.deepcopy(opts.diagnostics))

            if opts.capabilities then
                opts.servers["*"] = vim.tbl_deep_extend("force", opts.servers["*"] or {}, {
                    capabilities = opts.capabilities,
                })
            end

            if opts.servers["*"] then
                vim.lsp.config("*", opts.servers["*"])
            end

            -- get all the servers that are available through mason-lspconfig
            local have_mason = Util.has("mason-lspconfig.nvim")
            local mason_all = have_mason
                    and vim.tbl_keys(require("mason-lspconfig.mappings").get_mason_map().lspconfig_to_package)
                or {} --[[ @as string[] ]]
            local mason_exclude = {} ---@type string[]

            ---@return boolean? exclude automatic setup
            local function configure(server)
                if server == "*" then
                    return false
                end
                local sopts = opts.servers[server]
                sopts = sopts == true and {} or (not sopts) and { enabled = false } or sopts --[[@as lazyvim.lsp.Config]]

                if sopts.enabled == false then
                    mason_exclude[#mason_exclude + 1] = server
                    return
                end

                local use_mason = sopts.mason ~= false and vim.tbl_contains(mason_all, server)
                local setup = opts.setup[server] or opts.setup["*"]
                if setup and setup(server, sopts) then
                    mason_exclude[#mason_exclude + 1] = server
                else
                    vim.lsp.config(server, sopts)
                    if not use_mason then
                        vim.lsp.enable(server)
                    end
                end
                return use_mason
            end

            local install = vim.tbl_filter(configure, vim.tbl_keys(opts.servers))
            if have_mason then
                require("mason-lspconfig").setup({
                    ensure_installed = vim.list_extend(
                        install,
                        Util.opts("mason-lspconfig.nvim").ensure_installed or {}
                    ),
                    automatic_enable = { exclude = mason_exclude },
                })
            end
        end),
    },

    -- cmdline tools and lsp servers
    {
        "mason-org/mason.nvim",
        cmd = "Mason",
        build = ":MasonUpdate",
        opts_extend = { "ensure_installed" },
        opts = {
            ensure_installed = {
                "stylua",
                "shfmt",
            },
        },
        ---@param opts MasonSettings | {ensure_installed: string[]}
        config = function(_, opts)
            require("mason").setup(opts)
            local mr = require("mason-registry")
            mr:on("package:install:success", function()
                vim.defer_fn(function()
                    require("lazy.core.handler.event").trigger({
                        event = "FileType",
                        buf = vim.api.nvim_get_current_buf(),
                    })
                end, 100)
            end)

            mr.refresh(function()
                for _, tool in ipairs(opts.ensure_installed) do
                    local p = mr.get_package(tool)
                    if not p:is_installed() then
                        p:install()
                    end
                end
            end)
        end,
    },

    -- neoconf.nvim
    {
        "folke/neoconf.nvim",
        cmd = "Neoconf",
        opts = {},
    },

    -- SchemaStore
    {
        "b0o/SchemaStore.nvim",
        lazy = true,
        version = false,
    },
}
