-- Formatting 核心插件
-- 提供代码格式化功能

local M = {}

--- 初始化 conform.nvim，并清理不兼容的旧配置项。
---@param opts table
function M.setup(_, opts)
    for _, key in ipairs({ "format_on_save", "format_after_save" }) do
        if opts[key] then
            local Util = require("utils")
            local msg = "Don't set `opts.%s` for `conform.nvim`.\nWill use the conform formatter automatically"
            Util.warn(msg:format(key))
            opts[key] = nil
        end
    end
    if opts.format then
        local Util = require("utils")
        Util.warn("**conform.nvim** `opts.format` is deprecated. Please use `opts.default_format_opts` instead.")
    end
    require("conform").setup(opts)
end

return {
    {
        "stevearc/conform.nvim",
        dependencies = { "mason.nvim" },
        lazy = true,
        cmd = "ConformInfo",
        keys = {
            {
                "<leader>cF",
                function()
                    require("conform").format({ formatters = { "injected" }, timeout_ms = 3000 })
                end,
                mode = { "n", "x" },
                desc = "Format Injected Langs",
            },
            -- 用户自定义快捷键：= 格式化并保存
            {
                "=",
                function()
                    require("utils.format").format({ force = true })
                    vim.cmd("w")
                end,
                mode = { "n", "x", "v" },
                desc = "Format File",
            },
        },
        init = function()
            -- 注册 conform 格式化器
            local Util = require("utils")
            Util.on_very_lazy(function()
                Util.format.register({
                    name = "conform.nvim",
                    priority = 100,
                    primary = true,
                    format = function(buf)
                        require("conform").format({ bufnr = buf })
                    end,
                    sources = function(buf)
                        local ret = require("conform").list_formatters(buf)
                        ---@param v table
                        return vim.tbl_map(function(v)
                            return v.name
                        end, ret)
                    end,
                })
            end)
        end,
        opts = function()
            local plugin = require("lazy.core.config").plugins["conform.nvim"]
            if plugin.config ~= M.setup then
                local Util = require("utils")
                Util.error({
                    "Don't set `plugin.config` for `conform.nvim`.\n",
                    "This will break formatting.\n",
                })
            end
            ---@type table
            local opts = {
                default_format_opts = {
                    timeout_ms = 3000,
                    async = false,
                    quiet = false,
                    lsp_format = "fallback",
                },
                formatters_by_ft = {
                    lua = { "stylua" },
                    fish = { "fish_indent" },
                    sh = { "shfmt" },
                },
                formatters = {
                    injected = { options = { ignore_errors = true } },
                    stylua = {
                        prepend_args = {
                            "--indent-type",
                            "Spaces",
                            "--indent-width",
                            "4",
                        },
                    },
                },
            }
            return opts
        end,
        config = M.setup,
    },
}
