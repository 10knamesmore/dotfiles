return {
    {
        "nvim-treesitter/nvim-treesitter",
        opts = { ensure_installed = { "vue", "css" } },
    },

    {
        "neovim/nvim-lspconfig",
        opts = function(_, opts)
            -- Vue 文件也启用 inlay hint，沿用 vtsls 的 TypeScript 类型提示配置
            opts.inlay_hints = opts.inlay_hints or {}
            opts.inlay_hints.exclude = vim.tbl_filter(function(filetype)
                return filetype ~= "vue"
            end, opts.inlay_hints.exclude or {})

            opts.servers = opts.servers or {}
            opts.servers.vue_ls = opts.servers.vue_ls or {}

            local vtsls = opts.servers.vtsls
            if not vtsls then
                return
            end

            vtsls.filetypes = vtsls.filetypes or {}
            if not vim.tbl_contains(vtsls.filetypes, "vue") then
                table.insert(vtsls.filetypes, "vue")
            end

            vtsls.settings = vtsls.settings or {}
            vtsls.settings.vtsls = vtsls.settings.vtsls or {}
            vtsls.settings.vtsls.tsserver = vtsls.settings.vtsls.tsserver or {}
            vtsls.settings.vtsls.tsserver.globalPlugins = vtsls.settings.vtsls.tsserver.globalPlugins or {}

            local vue_plugin = {
                name = "@vue/typescript-plugin",
                location = vim.fn.stdpath("data")
                    .. "/mason/packages/vue-language-server/node_modules/@vue/language-server",
                languages = { "vue" },
                configNamespace = "typescript",
                enableForWorkspaceTypeScriptVersions = true,
            }

            local has_vue_plugin = false
            for _, plugin in ipairs(vtsls.settings.vtsls.tsserver.globalPlugins) do
                if plugin.name == vue_plugin.name then
                    has_vue_plugin = true
                    break
                end
            end

            if not has_vue_plugin then
                table.insert(vtsls.settings.vtsls.tsserver.globalPlugins, vue_plugin)
            end
        end,
    },
}
