--- 路径工具模块

--- 根目录检测器规范：检测器名 / 文件名模式 / 自定义函数
---@alias PathRootSpec string|string[]|fun(buf:number):string[]

--- 单条根目录检测结果
---@class PathRoot
---@field spec PathRootSpec 命中的检测器规范
---@field paths string[] 检测到的根目录（按长度降序）

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
---@type table<string, fun(buf:number, patterns?:string|string[]):string[]>
M.detectors = {}

--- 当前工作目录作为根目录。
---@return string[]
function M.detectors.cwd()
  return { vim.uv.cwd() }
end

--- 从挂载的 LSP 客户端读取 workspace folders / root_dir，仅保留包含当前文件的根。
--- `vim.g.root_lsp_ignore` 可列出要忽略的客户端名（如 copilot）。
---@param buf number
---@return string[]
function M.detectors.lsp(buf)
  local bufpath = M.bufpath(buf)
  if not bufpath then
    return {}
  end

  local roots = {} ---@type string[]
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
    if not vim.tbl_contains(vim.g.root_lsp_ignore or {}, client.name) then
      for _, ws in ipairs(client.config.workspace_folders or {}) do
        roots[#roots + 1] = vim.uri_to_fname(ws.uri)
      end
      if client.root_dir then
        roots[#roots + 1] = client.root_dir
      end
    end
  end

  return vim.tbl_filter(function(path)
    path = M.realpath(path)
    return path ~= nil and bufpath:find(path, 1, true) == 1
  end, roots)
end

--- 根据文件名或模式检测根目录
---@param buf number
---@param patterns string[]|string 匹配模式
---@return string[]
function M.detectors.pattern(buf, patterns)
  patterns = type(patterns) == "string" and { patterns } or patterns
  ---@cast patterns string[]
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
---@param spec PathRootSpec
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
---@param opts? { buf?: number, spec?: PathRootSpec[], all?: boolean }
---@return PathRoot[]
function M.detect(opts)
  opts = opts or {}
  opts.spec = opts.spec or type(vim.g.root_spec) == "table" and vim.g.root_spec or M.spec
  opts.buf = (opts.buf == nil or opts.buf == 0) and vim.api.nvim_get_current_buf() or opts.buf

  local ret = {} ---@type PathRoot[]
  for _, spec in ipairs(opts.spec) do
    local paths = M.resolve(spec)(opts.buf)
    paths = paths or {}
    paths = type(paths) == "table" and paths or { paths }
    ---@cast paths string[]
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
