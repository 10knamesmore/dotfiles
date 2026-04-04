--- 路径工具模块
local M = {}

--- 获取指定 buffer 的真实文件路径
---@param buf number buffer 句柄
---@return string|nil
function M.bufpath(buf)
    return M.realpath(vim.api.nvim_buf_get_name(assert(buf)))
end

--- 获取当前工作目录的真实路径
---@return string|nil
function M.cwd()
    return M.realpath(vim.uv.cwd()) or ""
end

--- 获取路径的真实路径 (win32 兼容)
---@param path string|nil
---@return string|nil
function M.realpath(path)
    if path == "" or path == nil then
        return nil
    end
    path = vim.fn.has("win32") == 0 and vim.uv.fs_realpath(path) or path
    return path
end

--- 根目录检测器集合
---@type table<string, fun(buf:number):string[]>
M.detectors = {}

--- 根据文件名或模式检测根目录
---@param buf number
---@param patterns string[]|string 匹配模式
---@return string[]
function M.detectors.pattern(buf, patterns)
    patterns = type(patterns) == "string" and { patterns } or patterns
    local path = M.bufpath(buf) or vim.uv.cwd()
    local pattern = vim.fs.find(function(name)
        for _, p in ipairs(patterns) do
            if name == p then
                return true
            end
            if p:sub(1, 1) == "*" and name:find(vim.pesc(p:sub(2)) .. "$") then
                return true
            end
        end
        return false
    end, { path = path, upward = true })[1]
    return pattern and { vim.fs.dirname(pattern) } or {}
end

--- 解析根目录检测器规范
---@param spec string|fun(buf:number):string[]|string[]
---@return fun(buf:number):string[]
function M.resolve(spec)
    if M.detectors[spec] then
        return M.detectors[spec]
    elseif type(spec) == "function" then
        return spec
    end
    return function(buf)
        return M.detectors.pattern(buf, spec)
    end
end

--- 检测所有可能的根目录
---@param opts? { buf?: number, spec?: LazyRootSpec[], all?: boolean }
function M.detect(opts)
    opts = opts or {}
    opts.spec = opts.spec or type(vim.g.root_spec) == "table" and vim.g.root_spec or M.spec
    opts.buf = (opts.buf == nil or opts.buf == 0) and vim.api.nvim_get_current_buf() or opts.buf

    local ret = {} ---@type LazyRoot[]
    for _, spec in ipairs(opts.spec) do
        local paths = M.resolve(spec)(opts.buf)
        paths = paths or {}
        paths = type(paths) == "table" and paths or { paths }
        local roots = {} ---@type string[]
        for _, p in ipairs(paths) do
            local pp = M.realpath(p)
            if pp and not vim.tbl_contains(roots, pp) then
                roots[#roots + 1] = pp
            end
        end
        table.sort(roots, function(a, b)
            return #a > #b
        end)
        if #roots > 0 then
            ret[#ret + 1] = { spec = spec, paths = roots }
            if opts.all == false then
                break
            end
        end
    end
    return ret
end

--- 缓存 buffer 到根目录的映射
---@type table<number, string>
M.cache = {} -- bunr -> dir

--- 获取指定 buffer 的根目录（优先级：LSP workspace > LSP root_dir > 文件名模式 > cwd）
---@return string
function M.get_root()
    local buf = vim.api.nvim_get_current_buf()
    local ret = M.cache[buf]

    if not ret then
        local roots = M.detect({ all = false, buf = buf })
        ret = roots[1] and roots[1].paths[1] or vim.uv.cwd()
        M.cache[buf] = ret
    end

    return ret
end

return M
