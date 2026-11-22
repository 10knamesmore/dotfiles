local M = {}
local api = vim.api

function M.parse(line)
  local parts, spans = {}, {}
  local offset, length = 1, 0
  local fg, bg
  while true do
    local first, last, sgr = line:find("\27%[([%d;]*)m", offset)
    local text = line:sub(offset, first and first - 1 or #line)
    parts[#parts + 1] = text
    if #text > 0 and (fg or bg) then
      spans[#spans + 1] = { start_col = length, end_col = length + #text, fg = fg, bg = bg }
    end
    if not first then
      break
    end
    if sgr == "0" then
      fg, bg = nil, nil
    else
      local layer, r, g, b = sgr:match("^([34]8);2;(%d+);(%d+);(%d+)$")
      if layer then
        local rgb = tonumber(r) * 65536 + tonumber(g) * 256 + tonumber(b)
        if layer == "38" then
          fg = rgb
        else
          bg = rgb
        end
      end
    end
    length = length + #text
    offset = last + 1
  end
  return { text = table.concat(parts), spans = spans }
end

local active, free = {}, {}
local slots = 0

-- Neovim retains highlight names for the session. Reuse a slot once its last extmark is gone.
function M.highlight(palette, span)
  local key = (span.fg or -1) .. ":" .. (span.bg or -1)
  local group = palette[key]
  if not group then
    group = active[key]
    if not group then
      local slot = table.remove(free)
      if not slot then
        slots = slots + 1
        slot = slots
      end
      group = {
        name = "MarkdownMermaidColor" .. slot,
        slot = slot,
        style = { fg = span.fg, bg = span.bg },
        users = 0,
      }
      api.nvim_set_hl(0, group.name, group.style)
      active[key] = group
    end
    palette[key] = group
    group.users = group.users + 1
  end
  return group.name
end

function M.release(palette)
  for key, group in pairs(palette) do
    group.users = group.users - 1
    if group.users == 0 then
      api.nvim_set_hl(0, group.name, {})
      active[key] = nil
      free[#free + 1] = group.slot
    end
  end
end

function M.chunks(line, prefix, palette)
  local chunks = {}
  local col = 0
  for _, span in ipairs(line.spans) do
    if span.start_col >= #prefix then
      break
    end
    if col < span.start_col then
      chunks[#chunks + 1] = { prefix:sub(col + 1, span.start_col) }
    end
    col = math.min(span.end_col, #prefix)
    chunks[#chunks + 1] = { prefix:sub(span.start_col + 1, col), M.highlight(palette, span) }
  end
  if col < #prefix or #chunks == 0 then
    chunks[#chunks + 1] = { prefix:sub(col + 1) }
  end
  return chunks
end

api.nvim_create_autocmd("ColorScheme", {
  callback = function()
    for _, group in pairs(active) do
      api.nvim_set_hl(0, group.name, group.style)
    end
  end,
})

return M
