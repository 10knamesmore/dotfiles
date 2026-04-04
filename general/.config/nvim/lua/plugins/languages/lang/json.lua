return {
    -- add json to treesitter
    {
        "nvim-treesitter/nvim-treesitter",
        opts = { ensure_installed = { "json5" } },
    },

    -- yaml schema support
    {
        "b0o/SchemaStore.nvim",
        lazy = true,
        version = false, -- last release is way too old
    },

    -- correctly setup lspconfig
    {
        "neovim/nvim-lspconfig",
        opts = {
            -- make sure mason installs the server
            servers = {
                jsonls = {
                    -- lazy-load schemastore when needed
                    before_init = function(_, new_config)
                        new_config.settings.json.schemas = new_config.settings.json.schemas or {}
                        vim.list_extend(new_config.settings.json.schemas, require("schemastore").json.schemas())
                    end,
                    settings = {
                        json = {
                            format = {
                                enable = true,
                            },
                            validate = { enable = true },
                        },
                    },
                },
            },
        },
    },
    -- {
    --     "10knamesmore/json-to-types.nvim",
    --     build = "sh install.sh pnpm",
    --     ft = "json",
    --     keys = {
    --         {
    --             "<leader>cj",
    --             function()
    --                 local choices = require("json-to-types.utils").language_map
    --                 vim.ui.select(choices, { prompt = "choose a file type" }, function(choice)
    --                     if choice then
    --                         require("json-to-types").convertTypesBuffer(choice)
    --                     end
    --                 end)
    --             end,
    --             -- "<CMD>ConvertJSONtoLang<CR>",
    --             desc = "Convert JSON Lang",
    --         },
    --     },
    -- },
}
