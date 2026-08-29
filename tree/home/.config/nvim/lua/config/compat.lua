-- 部分 LSP server 会在 workspace.fileOperations 中广播 `**/*.{}` 等
-- vim.glob.to_lpeg 无法解析的 glob；异常会中断文件操作或 LSP 注册。
-- 在解析边界把无效 glob 降级为空匹配，避免误匹配任何路径。
local orig = vim.glob.to_lpeg
vim.glob.to_lpeg = function(pattern)
  local ok, res = pcall(orig, pattern)
  if ok then
    return res
  end
  -- P(false) 永不匹配 → 该 filter 匹配不到任何路径，等价于「忽略这条不合规
  -- 规则」，比 P(true)（匹配一切、会误发操作）安全。
  return vim.lpeg.P(false)
end

-- 接受插件仍会调用的数字 bufnr 签名，并用 enable(false, filter) 提供 disable。
do
  local diagnostic_enable = vim.diagnostic.enable

  local function legacy_filter(bufnr, namespace)
    local filter = {}
    if bufnr ~= nil then
      filter.bufnr = bufnr
    end
    if namespace ~= nil then
      filter.ns_id = namespace
    end
    return filter
  end

  if type(diagnostic_enable) == "function" then
    vim.diagnostic.enable = function(enable, filter)
      if type(enable) == "number" then
        return diagnostic_enable(true, legacy_filter(enable, filter))
      end
      return diagnostic_enable(enable, filter)
    end
  end

  if type(vim.diagnostic.disable) ~= "function" and type(diagnostic_enable) == "function" then
    vim.diagnostic.disable = function(bufnr, namespace)
      return diagnostic_enable(false, legacy_filter(bufnr, namespace))
    end
  end
end
