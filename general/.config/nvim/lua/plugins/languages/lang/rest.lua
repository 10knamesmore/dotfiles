vim.filetype.add({
    extension = {
        ["http"] = "http",
    },
})
return {
    {
        "mistweaverco/kulala.nvim",
        ft = "http",
        keys = {
            { "<leader>r", "", desc = "+Rest" },
            {
                "<leader>rb",
                function()
                    require("kulala").scratchpad()
                end,
                desc = "scratchpad",
            },
            {
                "<leader>rc",
                function()
                    require("kulala").copy()
                end,
                desc = "Copy as cURL",
                ft = "http",
            },
            {
                "<leader>rC",
                function()
                    require("kulala").from_curl()
                end,
                desc = "Paste from curl",
                ft = "http",
            },
            {
                "<leader>re",
                function()
                    require("kulala").set_selected_env()
                end,
                desc = "Set environment",
                ft = "http",
            },
            {
                "<leader>rg",
                function()
                    require("kulala").download_graphql_schema()
                end,
                desc = "Download GraphQL schema",
                ft = "http",
            },
            {
                "<leader>ri",
                function()
                    require("kulala").inspect()
                end,
                desc = "Inspect current request",
                ft = "http",
            },
            {
                "<leader>rn",
                function()
                    require("kulala").jump_next()
                end,
                desc = "Jump to next request",
                ft = "http",
            },
            {
                "<leader>rp",
                function()
                    require("kulala").jump_prev()
                end,
                desc = "Jump to previous request",
                ft = "http",
            },
            {
                "<leader>rq",
                function()
                    require("kulala").close()
                end,
                desc = "Close window",
                ft = "http",
            },
            {
                "<leader>rr",
                function()
                    require("kulala").replay()
                end,
                desc = "Replay the last request",
            },
            {
                "<leader>rs",
                function()
                    require("kulala").run()
                end,
                desc = "Send the request",
                ft = "http",
            },
            {
                "<leader>rS",
                function()
                    require("kulala").show_stats()
                end,
                desc = "Show stats",
                ft = "http",
            },
            {
                "<leader>rt",
                function()
                    require("kulala").toggle_view()
                end,
                desc = "Toggle headers/body",
                ft = "http",
            },
        },
        opts = {
            contenttypes = {
                ["application/json"] = {
                    ft = "json",
                    formatter = function(body)
                        local ok, decoded = pcall(vim.json.decode, body)
                        if ok and type(decoded) == "string" then
                            return decoded
                        end

                        if vim.fn.executable("jq") == 1 then
                            local out = vim.fn.system({ "jq", "." }, body)
                            if vim.v.shell_error == 0 and out ~= "" then
                                return out
                            end
                        end

                        return body
                    end,
                },
            },
        },
    },
    {
        "nvim-treesitter/nvim-treesitter",
        opts = {
            ensure_installed = { "http", "graphql" },
        },
    },
}
