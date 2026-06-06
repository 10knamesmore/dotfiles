-- LSP 工具模块
-- 提供 LSP 相关的辅助功能

---@class util.lsp
local M = {}

--- 创建一个基于当前 LSP 客户端的格式化器描述。
---@param opts? {filter?: (string|vim.lsp.get_clients.Filter)}
function M.formatter(opts)
    opts = opts or {}
    local filter = opts.filter or {}
    filter = type(filter) == "string" and { name = filter } or filter
    ---@cast filter vim.lsp.get_clients.Filter

    ---@type Formatter
    local ret = {
        name = "LSP",
        primary = true,
        priority = 1,
        format = function(buf)
            M.format(vim.tbl_deep_extend("force", {}, filter, { bufnr = buf }))
        end,
        sources = function(buf)
            local clients = vim.lsp.get_clients(vim.tbl_deep_extend("force", {}, filter, { bufnr = buf }))
            ---@param client vim.lsp.Client
            local ret = vim.tbl_filter(function(client)
                return client:supports_method("textDocument/formatting")
                    or client:supports_method("textDocument/rangeFormatting")
            end, clients)
            ---@param client vim.lsp.Client
            return vim.tbl_map(function(client)
                return client.name
            end, ret)
        end,
    }
    return vim.tbl_deep_extend("force", ret, opts)
end

--- 使用 `conform.nvim` 或原生 LSP 执行格式化。
---@param opts? {timeout_ms?: number, format_options?: table, bufnr?: number}
function M.format(opts)
    local Util = require("utils")
    opts = vim.tbl_deep_extend(
        "force",
        {},
        opts or {},
        Util.opts("nvim-lspconfig").format or {},
        Util.opts("conform.nvim").format or {}
    )

    local ok, conform = pcall(require, "conform")
    -- 优先使用 conform 进行 LSP 格式化（更好的 diff）
    if ok then
        opts.formatters = {}
        conform.format(opts)
    else
        vim.lsp.buf.format(opts)
    end
end

--- 代码操作快捷方式
--- 用法: Util.lsp.action.source() 执行 source action
--- 用法: Util.lsp.action["source.organizeImports"]() 组织导入
M.action = setmetatable({}, {
    __index = function(_, action)
        return function()
            vim.lsp.buf.code_action({
                apply = true,
                context = {
                    only = { action },
                    diagnostics = {},
                },
            })
        end
    end,
})

--- 执行工作区级 LSP 命令，可选通过 Trouble 打开结果。
---@param opts {command: string, arguments?: any[], open?: boolean, handler?: function}
function M.execute(opts)
    local params = {
        command = opts.command,
        arguments = opts.arguments,
    }
    if opts.open then
        local ok, trouble = pcall(require, "trouble")
        if ok then
            trouble.open({
                mode = "lsp_command",
                params = params,
            })
        else
            return vim.lsp.buf_request(0, "workspace/executeCommand", params, opts.handler)
        end
    else
        return vim.lsp.buf_request(0, "workspace/executeCommand", params, opts.handler)
    end
end

--- LSP keymaps 管理
M.keymaps = {}

---@type LazyKeysLspSpec[]|nil
M.keymaps._keys = {}

--- 为满足过滤条件的 LSP 客户端批量注册按键。
---@param filter vim.lsp.get_clients.Filter
---@param spec LazyKeysLspSpec[]
function M.keymaps.set(filter, spec)
    local Keys = require("lazy.core.handler.keys")
    for _, keys in pairs(Keys.resolve(spec)) do
        ---@cast keys LazyKeysLsp
        local filters = {} ---@type vim.lsp.get_clients.Filter[]
        if keys.has then
            local methods = type(keys.has) == "string" and { keys.has } or keys.has --[[@as string[] ]]
            for _, method in ipairs(methods) do
                method = method:find("/") and method or ("textDocument/" .. method)
                filters[#filters + 1] = vim.tbl_extend("force", vim.deepcopy(filter), { method = method })
            end
        else
            filters[#filters + 1] = filter
        end

        for _, f in ipairs(filters) do
            local opts = Keys.opts(keys)
            ---@cast opts snacks.keymap.set.Opts
            opts.lsp = f
            opts.enabled = keys.enabled
            Snacks.keymap.set(keys.mode or "n", keys.lhs, keys.rhs, opts)
        end
    end
end

return M
