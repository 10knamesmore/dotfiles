--- 将窗口缓存和请求中的译文投影到显示层；不修改源 buffer，也不启动网络请求。
local M = {}
local api = vim.api

---@alias WindowTranslate.Mode "inline"|"window"|"split"

---@class WindowTranslate.Style
---@field inline WindowTranslate.InlineStyle
---@field window fun(source_window: integer): vim.api.keyset.win_config 浮窗配置；打开和编辑器尺寸变化时重新计算。
---@field split fun(source_window: integer): vim.api.keyset.win_config 分栏配置；只在打开时计算，之后允许手动调整尺寸。
---@field window_options vim.wo 浮窗与分栏共用的 window-local options。
---@field progress WindowTranslate.ProgressStyle
---@field highlights table<string, vim.api.keyset.highlight> 配色切换后重新应用的 highlight 组定义。

---@class WindowTranslate.StyleOptions
---@field inline? WindowTranslate.InlineOptions
---@field window? fun(source_window: integer): vim.api.keyset.win_config
---@field split? fun(source_window: integer): vim.api.keyset.win_config
---@field window_options? vim.wo
---@field progress? WindowTranslate.ProgressOptions
---@field highlights? table<string, vim.api.keyset.highlight> 每个组整项替换，避免旧 link 覆盖新 fg/bg。

---@class WindowTranslate.ProgressStyle
---@field text string 请求中的提示文字。
---@field highlight string 提示文字的 highlight 组名。

---@class WindowTranslate.ProgressOptions
---@field text? string
---@field highlight? string

--- virt_lines 不使用窗口的 wrap；这里按显示格数拆行，保留输入中的换行。
---@param text string
---@param width integer 译文可用的显示列数，至少为 1。
---@param prefix_width integer 前缀占用的显示列数，参与 tab 宽度计算。
---@return string[]
local function wrap(text, width, prefix_width)
  local result = {}
  for _, paragraph in ipairs(vim.split(text, "\n", { plain = true })) do
    local line, cells = "", 0
    for _, char in ipairs(vim.fn.split(paragraph, "\\zs")) do
      local size = vim.fn.strdisplaywidth(char, prefix_width + cells)
      if cells + size > width and line ~= "" then
        result[#result + 1] = line
        line, cells = "", 0
      end
      line, cells = line .. char, cells + size
    end
    result[#result + 1] = line
  end
  return result
end

--- 清除源 buffer 的虚拟行和渲染缓存；不影响译文窗口或翻译缓存。
---@param state WindowTranslate.Window
function M.clear_inline(state)
  if api.nvim_buf_is_valid(state.buffer) then
    api.nvim_buf_clear_namespace(state.buffer, state.namespace, 0, -1)
  end
  state.inline_marks = {}
  state.progress_line = nil
end

--- 释放当前显示资源；请求与完整译文缓存仍由上层管理。
---@param state WindowTranslate.Window
function M.close(state)
  M.clear_inline(state)
  local window = state.output_window
  -- WinClosed 用此字段判断用户关闭；程序性关闭先解除关联。
  state.output_window = nil
  if window and api.nvim_win_is_valid(window) then
    api.nvim_win_close(window, true)
  end
  state.layouts = {}
  state.output_lines = {}
  state.output_topline = nil
end

--- 在发请求前创建浮窗或分栏。inline 在可见块末行预留空位。
---@param state WindowTranslate.Window
---@param style WindowTranslate.Style
function M.open(state, style)
  if state.mode == "inline" or state.output_window then
    return
  end
  local buffer = api.nvim_create_buf(false, true)
  state.output_lines = { "" }
  vim.bo[buffer].bufhidden = "wipe"
  vim.bo[buffer].filetype = "windowtranslate"
  local options = style[state.mode](state.window)
  state.output_window = api.nvim_open_win(buffer, false, options)
  local winopts = vim.wo[state.output_window]
  for name, value in pairs(style.window_options) do
    winopts[name] = value
  end
  winopts.winfixbuf = true
end

---@class WindowTranslate.InlineStyle
---@field prefix string 每行译文的前缀；等待中的空位不显示前缀。
---@field highlight string 译文的 highlight 组名。
---@field padding_above integer 段落上方空行数。
---@field padding_below integer 段落下方空行数。
---@field reserve_lines integer 每个待翻译块的预留虚拟行数；完成后收起多余空位。
---@field wrap boolean 按源窗口可用宽度折行；关闭后超宽内容由 Neovim 截断。

---@class WindowTranslate.InlineOptions
---@field prefix? string
---@field highlight? string
---@field padding_above? integer
---@field padding_below? integer
---@field reserve_lines? integer
---@field wrap? boolean

---@alias WindowTranslate.HighlightChunk { [1]: string, [2]: string }
---@alias WindowTranslate.VirtualLine WindowTranslate.HighlightChunk[]

--- 按块 ID 缓存排版；文本变化只重建这一项，宽度变化只使折行结果失效。
---@class WindowTranslate.Layout
---@field text string
---@field plain string[] 按原始换行拆分的译文，供浮窗和分栏复用。
---@field width? integer inline 可用列数；-1 表示关闭折行。
---@field wrapped? string[]
---@field reserved? integer 上次虚拟行保留的最小高度。
---@field virtual? WindowTranslate.VirtualLine[]

---@param state WindowTranslate.Window
---@param entry WindowTranslate.Block
---@return WindowTranslate.Layout
local function layout(state, entry)
  local cached = state.layouts[entry.id]
  if not cached or cached.text ~= entry.text then
    cached = {
      text = entry.text,
      plain = entry.text ~= "" and vim.split(entry.text, "\n", { plain = true }) or {},
    }
    state.layouts[entry.id] = cached
  end
  return cached
end

---@param state WindowTranslate.Window
---@param entry WindowTranslate.Block
---@param inline WindowTranslate.InlineStyle
---@param width integer
---@param prefix_width integer
---@return WindowTranslate.VirtualLine[]
local function inline_lines(state, entry, inline, width, prefix_width)
  local cached = layout(state, entry)
  if cached.width ~= width then
    cached.width = width
    cached.wrapped = inline.wrap and entry.text ~= "" and wrap(entry.text, width, prefix_width) or cached.plain
    cached.virtual = nil
  end
  local reserved = not entry.complete and not entry.error and inline.reserve_lines or 0
  if cached.virtual and cached.reserved == reserved then
    return cached.virtual
  end
  local translated = cached.wrapped
  ---@cast translated string[]
  ---@type WindowTranslate.VirtualLine[]
  local lines = {}
  if #translated == 0 and reserved == 0 then
    cached.reserved, cached.virtual = reserved, lines
    return lines
  end
  for _ = 1, inline.padding_above do
    lines[#lines + 1] = { { "", inline.highlight } }
  end
  for index = 1, math.max(#translated, reserved) do
    local text = translated[index]
    lines[#lines + 1] = { { text and (inline.prefix .. text) or "", inline.highlight } }
  end
  for _ = 1, inline.padding_below do
    lines[#lines + 1] = { { "", inline.highlight } }
  end
  cached.reserved, cached.virtual = reserved, lines
  return lines
end

--- 只更新当前视口的 marks；屏幕外预取不创建占位，已显示的屏幕外 marks 保留原位。
--- 一批布局变更使用 Neovim 原生 view 保存/恢复，保留源首行、横向偏移与光标。
---@param state WindowTranslate.Window
---@param frame WindowTranslate.Viewport
---@param style WindowTranslate.Style
---@param loading boolean
local function render_inline(state, frame, style, loading)
  local inline = style.inline
  local prefix_width = vim.fn.strdisplaywidth(inline.prefix)
  local width = inline.wrap
      and math.max(1, api.nvim_win_get_width(state.window) - vim.fn.getwininfo(state.window)[1].textoff - prefix_width)
    or -1
  local marks = {}
  for _, entry in ipairs(frame.blocks) do
    if entry.end_line <= frame.last then
      local lines = inline_lines(state, entry, inline, width, prefix_width)
      if #lines > 0 then
        marks[entry.end_line] = marks[entry.end_line] or {}
        vim.list_extend(marks[entry.end_line], lines)
      end
    end
  end
  local updates = {}
  for _, source in ipairs(frame.lines) do
    local previous, lines = state.inline_marks[source.line], marks[source.line]
    local same = previous == lines
    if previous and lines and #previous == #lines then
      same = true
      for index, line in ipairs(lines) do
        if previous[index] ~= line then
          same = false
          break
        end
      end
    end
    if not same then
      updates[#updates + 1] = { line = source.line, lines = lines }
    end
  end
  if #updates > 0 then
    api.nvim_win_call(state.window, function()
      local saved = vim.fn.winsaveview()
      for _, update in ipairs(updates) do
        -- 偶数 ID 属于源行，1 留给进度提示。
        if update.lines then
          api.nvim_buf_set_extmark(state.buffer, state.namespace, update.line - 1, 0, {
            id = update.line * 2,
            virt_lines = update.lines,
          })
        else
          api.nvim_buf_del_extmark(state.buffer, state.namespace, update.line * 2)
        end
        state.inline_marks[update.line] = update.lines
      end
      vim.fn.winrestview(saved)
    end)
  end
  local progress_line = loading and frame.first or nil
  if state.progress_line ~= progress_line then
    state.progress_line = progress_line
    if progress_line then
      api.nvim_buf_set_extmark(state.buffer, state.namespace, progress_line - 1, 0, {
        id = 1,
        virt_text = { { style.progress.text, style.progress.highlight } },
        virt_text_pos = "right_align",
      })
    else
      api.nvim_buf_del_extmark(state.buffer, state.namespace, 1)
    end
  end
end

--- 找出前后未变化的行，只写入中间变化区；流式更新保留译文窗口的阅读位置。
---@param state WindowTranslate.Window
---@param lines string[]
---@param topline integer
local function write_output(state, lines, topline)
  local previous = state.output_lines
  local first, last, old_last = 1, #lines, #previous
  while first <= math.min(last, old_last) and lines[first] == previous[first] do
    first = first + 1
  end
  while last >= first and old_last >= first and lines[last] == previous[old_last] do
    last, old_last = last - 1, old_last - 1
  end
  local window = state.output_window
  ---@cast window integer
  api.nvim_win_call(window, function()
    local saved = vim.fn.winsaveview()
    if first <= last or first <= old_last then
      local buffer = api.nvim_win_get_buf(window)
      vim.bo[buffer].modifiable = true
      api.nvim_buf_set_lines(buffer, first - 1, old_last, false, vim.list_slice(lines, first, last))
      vim.bo[buffer].modifiable = false
    end
    if state.output_topline ~= topline then
      state.output_topline = topline
      api.nvim_win_set_cursor(window, { 1, 0 })
    else
      vim.fn.winrestview(saved)
    end
  end)
  state.output_lines = lines
end

--- 只投影当前视口涉及的块；未处理块保持空位，无译文的完整块显示原文。
--- 流式文本变化不重置译文光标，只有源视口首行变化时才移回顶部。
---@param state WindowTranslate.Window
---@param frame WindowTranslate.Viewport
---@param style WindowTranslate.Style
function M.render(state, frame, style)
  local loading = false
  for _, block in ipairs(frame.blocks) do
    loading = loading or (not block.complete and not block.error)
  end
  if state.mode == "inline" then
    render_inline(state, frame, style, loading)
    return
  end
  local window = state.output_window
  ---@cast window integer
  local lines, shown = {}, {}
  for _, source in ipairs(frame.lines) do
    local entries = state.by_line[source.line]
    if entries then
      for _, entry in ipairs(entries) do
        if not shown[entry.id] then
          local translated = layout(state, entry).plain
          if entry.error or (entry.complete and entry.text == "") then
            translated = vim.split(entry.source, "\n", { plain = true })
          end
          for index = 1, math.max(#translated, 1) do
            lines[#lines + 1] = translated[index] or ""
          end
          shown[entry.id] = true
        end
      end
    else
      lines[#lines + 1] = source.text
    end
  end
  write_output(state, lines, frame.first)
  vim.wo[window].winbar = loading and ("%#" .. style.progress.highlight .. "#" .. style.progress.text .. "%*")
    or (style.window_options.winbar or "")
end

--- 编辑器 resize 时重算浮窗布局；分栏保留用户手动调整的尺寸。
---@param state WindowTranslate.Window
---@param style WindowTranslate.Style
function M.resize(state, style)
  if state.mode == "window" and state.output_window then
    api.nvim_win_set_config(state.output_window, style.window(state.window))
  end
end

return M
