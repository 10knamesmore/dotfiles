-- Linting 核心插件
-- 提供代码检查功能

return {
    -- nvim-lint - 异步调用语言特定的 linter
    {
        "mfussenegger/nvim-lint",
        event = { "BufReadPost", "BufNewFile", "BufWritePre" },
        opts = {
            -- 触发 linter 的事件
            events = { "BufWritePost", "BufReadPost", "InsertLeave" },
            linters_by_ft = {
                fish = { "fish" },
                -- 使用 "*" 对所有文件类型运行 linter
                -- ['*'] = { 'global linter' },
                -- 使用 "_" 作为没有配置 linter 的文件类型的fallback
                -- ['_'] = { 'fallback linter' },
            },
            -- 自定义 linter 配置
            ---@type table<string,table>
            linters = {
                -- 示例：只在有 selene.toml 文件时使用 selene
                -- selene = {
                --   condition = function(ctx)
                --     return vim.fs.find({ "selene.toml" }, { path = ctx.filename, upward = true })[1]
                --   end,
                -- },
            },
        },
        config = function(_, opts)
            local M = {}

            local lint = require("lint")
            for name, linter in pairs(opts.linters) do
                if type(linter) == "table" and type(lint.linters[name]) == "table" then
                    lint.linters[name] = vim.tbl_deep_extend("force", lint.linters[name], linter)
                    if type(linter.prepend_args) == "table" then
                        lint.linters[name].args = lint.linters[name].args or {}
                        vim.list_extend(lint.linters[name].args, linter.prepend_args)
                    end
                else
                    lint.linters[name] = linter
                end
            end
            lint.linters_by_ft = opts.linters_by_ft

            --- 为函数创建防抖包装，避免频繁触发 lint。
            ---@param ms integer
            ---@param fn function
            ---@return function
            function M.debounce(ms, fn)
                local timer = vim.uv.new_timer()
                return function(...)
                    local argv = { ... }
                    timer:start(ms, 0, function()
                        timer:stop()
                        vim.schedule_wrap(fn)(unpack(argv))
                    end)
                end
            end

            --- 根据当前文件类型解析并执行可用的 linter。
            function M.lint()
                -- 解析文件类型对应的 linters
                local names = lint._resolve_linter_by_ft(vim.bo.filetype)
                names = vim.list_extend({}, names)

                -- 添加 fallback linters
                if #names == 0 then
                    vim.list_extend(names, lint.linters_by_ft["_"] or {})
                end

                -- 添加全局 linters
                vim.list_extend(names, lint.linters_by_ft["*"] or {})

                -- 过滤不存在或不匹配条件的 linters
                local ctx = { filename = vim.api.nvim_buf_get_name(0) }
                ctx.dirname = vim.fn.fnamemodify(ctx.filename, ":h")
                names = vim.tbl_filter(function(name)
                    local linter = lint.linters[name]
                    if not linter then
                        local Util = require("utils")
                        Util.warn("Linter not found: " .. name, { title = "nvim-lint" })
                    end
                    return linter and not (type(linter) == "table" and linter.condition and not linter.condition(ctx))
                end, names)

                -- 运行 linters
                if #names > 0 then
                    lint.try_lint(names)
                end
            end

            vim.api.nvim_create_autocmd(opts.events, {
                group = vim.api.nvim_create_augroup("nvim-lint", { clear = true }),
                callback = M.debounce(100, M.lint),
            })
        end,
    },
}
