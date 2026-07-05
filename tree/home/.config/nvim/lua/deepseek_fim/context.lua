-- 从 buffer 抽取 FIM 所需的 prompt(光标前) / suffix(光标后)。
local M = {}

--- 收集光标处的前后文。
--- @param bufnr integer
--- @param cursor integer[]  blink 的 context.cursor，形如 { row(1-indexed), col(0-indexed 字节列) }
--- @param cfg deepseek_fim.Config
--- @return string prompt  光标前文本(含当前行光标左半)
--- @return string suffix  光标后文本(含当前行光标右半)
function M.collect(bufnr, cursor, cfg)
  local row = cursor[1] -- 1-indexed 行号
  local col = cursor[2] -- 0-indexed 字节列
  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  local total = #lines
  local cur_line = lines[row] or ""

  -- 当前行按光标列切成左右两半(按字节，和 col 的语义一致)
  local before_cur = cur_line:sub(1, col)
  local after_cur = cur_line:sub(col + 1)

  -- prompt = 上文若干行 + 当前行光标左半
  local pstart = math.max(1, row - cfg.prefix_lines)
  local prefix_parts = {}
  for i = pstart, row - 1 do
    prefix_parts[#prefix_parts + 1] = lines[i]
  end
  prefix_parts[#prefix_parts + 1] = before_cur
  local prompt = table.concat(prefix_parts, "\n")

  -- suffix = 当前行光标右半 + 下文若干行
  local send = math.min(total, row + cfg.suffix_lines)
  local suffix_parts = { after_cur }
  for i = row + 1, send do
    suffix_parts[#suffix_parts + 1] = lines[i]
  end
  local suffix = table.concat(suffix_parts, "\n")

  return prompt, suffix
end

return M
