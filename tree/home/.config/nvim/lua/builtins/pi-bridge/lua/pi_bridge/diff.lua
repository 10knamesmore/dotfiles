--- Proposed edits are decorations on the source buffer, never changes to its text.
local M = {}
local api = vim.api
local namespace = api.nvim_create_namespace("pi_diff_preview")

function M.clear(buffer)
  api.nvim_buf_clear_namespace(buffer, namespace, 0, -1)
end

--- Capture the source revision so a later station cannot preview against edited text.
function M.prepare(buffer, proposed_text)
  local before = table.concat(api.nvim_buf_get_lines(buffer, 0, -1, true), "\n")
    .. (vim.bo[buffer].endofline and "\n" or "")
  return {
    buffer = buffer,
    changedtick = api.nvim_buf_get_changedtick(buffer),
    after = vim.split(proposed_text, "\n", { plain = true }),
    hunks = vim.text.diff(before, proposed_text, { result_type = "indices", algorithm = "histogram" }),
  }
end

function M.show(preview)
  M.clear(preview.buffer)
  for _, hunk in ipairs(preview.hunks) do
    local before_start, before_count, after_start, after_count = unpack(hunk)
    for row = before_start - 1, before_start + before_count - 2 do
      api.nvim_buf_set_extmark(preview.buffer, namespace, row, 0, {
        line_hl_group = "DiffDelete",
        sign_text = "-",
        sign_hl_group = "DiffDelete",
        priority = 210,
      })
    end
    if after_count > 0 then
      local lines = {}
      for row = after_start, after_start + after_count - 1 do
        lines[#lines + 1] = { { "+ " .. preview.after[row], "DiffAdd" } }
      end
      -- With no removed lines, xdiff's start is the line before the insertion.
      local anchor = before_count > 0 and before_start + before_count - 2 or before_start - 1
      api.nvim_buf_set_extmark(preview.buffer, namespace, math.max(0, anchor), 0, {
        virt_lines = lines,
        virt_lines_above = anchor < 0,
        virt_lines_overflow = "scroll",
      })
    end
  end
end

--- Screen spans include virtual additions so a tour comment does not cover the diff.
function M.screen_spans(buffer, window)
  local spans = {}
  local viewport = vim.fn.getwininfo(window)[1]
  local viewport_bottom = viewport.winrow + viewport.winbar + viewport.height - 1
  for _, mark in ipairs(api.nvim_buf_get_extmarks(buffer, namespace, 0, -1, { details = true })) do
    local row, details = mark[2] + 1, mark[4]
    local line = api.nvim_buf_get_lines(buffer, row - 1, row, true)[1]
    local first = vim.fn.screenpos(window, row, 1)
    local last = vim.fn.screenpos(window, row, math.max(1, #line))
    if first.row > 0 then
      local line_bottom = last.row > 0 and last.row or viewport_bottom
      local top, bottom = first.row, line_bottom
      if details.virt_lines then
        if details.virt_lines_above then
          top, bottom = first.row - #details.virt_lines, first.row - 1
        else
          top, bottom = line_bottom + 1, line_bottom + #details.virt_lines
        end
      end
      spans[#spans + 1] = { top = top, bottom = bottom, left = first.col }
    end
  end
  return spans
end

return M
