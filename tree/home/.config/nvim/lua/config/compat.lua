-- neovim 0.12 收紧了 `vim.glob.to_lpeg` 的文法（PR neovim/neovim#37161），
-- 以前能容忍的 glob 现在直接 assert 报错。部分 LSP server 仍会在
-- `workspace.fileOperations` 里广播这类不合规 glob（实测有空 brace 的
-- `**/*.{}`，另见 `**/*.{mdx}` / `**.dart` / `bundled:///libs/**/*`），
-- 而调用方（mini.files 的 fs-actions、原生 LSP 注册、noice）都是裸调
-- to_lpeg，一炸就把整个文件操作/注册中断。
-- 上游认定是 server 的锅、短期不放松 to_lpeg：neovim/neovim#37204。
-- 这里在边界兜底：无法解析的 glob 降级为「匹配空」而非抛错。
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

-- neovim 0.12 将 `vim.diagnostic.enable(bufnr)` 旧签名改成
-- `vim.diagnostic.enable(true, { bufnr = bufnr })`，并移除了
-- `vim.diagnostic.disable(bufnr)`。部分插件仍在 autocommand 中使用旧签名。
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
