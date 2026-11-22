local display = {
  ascii = false, -- 只替换图形线条，保留原有标签文字。
  color = true,
  border_corners = "rounded", -- auto / square / rounded，或依序指定左上、右上、左下、右下。
  source = "cursor", -- "cursor"：光标进块时显示；"below"：阅读时源码也留在图下。
  max_width = nil, -- 默认使用各窗口的可用列数，可设为固定上限。
}

local M = {}
local api = vim.api
local colors = require("utils.markdown_mermaid_colors")
local buffers = {}
local scratch_ns = api.nvim_create_namespace("markdown-mermaid-scratch")
local scratch_palettes = {}
local queue = {}
local running = 0
local max_running = 2

local fence_query = [[
  (fenced_code_block
    (fenced_code_block_delimiter) @open
    (info_string (language) @_language)
    (code_fence_content) @content
    (fenced_code_block_delimiter) @close
    (#eq? @_language "mermaid"))
]]

local function collect(buf, parser)
  local query = vim.treesitter.query.parse("markdown", fence_query)
  local blocks = {}
  for _, tree in ipairs(parser:parse()) do
    for _, match in query:iter_matches(tree:root(), buf) do
      local nodes = {}
      for id, captures in pairs(match) do
        nodes[query.captures[id]] = captures[1]
      end
      local first, col = nodes.open:start()
      local last = nodes.close:start()
      local line = api.nvim_buf_get_lines(buf, first, first + 1, false)[1]
      local content = nodes.content
      local lines = vim.split(vim.treesitter.get_node_text(content, buf), "\n", { plain = true })
      local start = content:start()
      -- 续行节点记录引文和列表的 Markdown 前缀。
      for child in content:iter_children() do
        if child:type() == "block_continuation" then
          local row, prefix = child:end_()
          if row > start then
            lines[row - start + 1] = lines[row - start + 1]:sub(prefix + 1)
          end
        end
      end
      blocks[#blocks + 1] = {
        first = first,
        last = last,
        col = col,
        end_col = #line,
        indent = vim.fn.strdisplaywidth(line:sub(1, col)),
        source = table.concat(lines, "\n"),
      }
    end
  end
  return blocks
end

local function clip(line, width, indent)
  if vim.fn.strdisplaywidth(line, indent) <= width then
    return line
  end
  local first, last = 0, vim.fn.strcharlen(line)
  while first < last do
    local count = math.floor((first + last + 1) / 2)
    if vim.fn.strdisplaywidth(vim.fn.strcharpart(line, 0, count, true), indent) <= width then
      first = count
    else
      last = count - 1
    end
  end
  return vim.fn.strcharpart(line, 0, first, true)
end

local function decorate(buf, ns, block, lines, width, palette)
  local virtual = {}
  local padding = (" "):rep(block.indent)
  local keep_source = display.source == "below"
  for i = keep_source and 1 or 2, #lines do
    local line = lines[i]
    local chunks = colors.chunks(line, clip(line.text, width, block.indent), palette)
    if block.indent > 0 then
      table.insert(chunks, 1, { padding })
    end
    virtual[#virtual + 1] = chunks
  end
  if keep_source then
    api.nvim_buf_set_extmark(buf, ns, block.first, 0, {
      virt_lines = virtual,
      virt_lines_above = true,
    })
    return
  end
  -- 保留首行供光标进入；overlay 避免满宽预览触发换行。
  local first = lines[1]
  api.nvim_buf_set_extmark(buf, ns, block.first, block.col, {
    end_col = block.end_col,
    conceal = "",
    virt_text = colors.chunks(first, clip(first.text, width, block.indent), palette),
    virt_text_pos = "overlay",
    virt_lines = virtual,
  })
  api.nvim_buf_set_extmark(buf, ns, block.first + 1, 0, {
    end_row = block.last,
    end_col = 0,
    conceal_lines = "",
  })
end

local function write_scratch(buf, result)
  local lines = result.lines and vim.tbl_map(function(line)
    return line.text
  end, result.lines) or result.failure or { "正在渲染 Mermaid…" }
  vim.bo[buf].modifiable = true
  api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false
  vim.bo[buf].modified = false
  if not result.lines then
    return
  end
  local palette = {}
  for row, line in ipairs(result.lines) do
    for _, span in ipairs(line.spans) do
      api.nvim_buf_set_extmark(buf, scratch_ns, row - 1, span.start_col, {
        end_col = span.end_col,
        hl_group = colors.highlight(palette, span),
      })
    end
  end
  scratch_palettes[buf] = next(palette) and palette or nil
end

local function open_scratch(result)
  local buf = result.scratch or api.nvim_create_buf(false, true)
  vim.bo[buf].bufhidden = "wipe"
  local win = vim.fn.bufwinid(buf)
  if win == -1 then
    win = api.nvim_open_win(buf, true, { split = "below" })
  else
    api.nvim_set_current_win(win)
  end
  local options = vim.wo[win]
  options.wrap = false
  options.number = false
  options.relativenumber = false
  options.signcolumn = "no"
  options.foldcolumn = "0"
  options.list = false
  write_scratch(buf, result)
  return buf
end

local refresh

local function run_jobs()
  while running < max_running and #queue > 0 do
    local job = table.remove(queue, 1)
    job()
  end
end

local function queue_render(buf, view, key, source, width)
  local result = {}
  local function finish(output, launch_error)
    if output and output.code == 0 and output.stdout and output.stdout:find("%S") then
      result.lines = vim.tbl_map(colors.parse, vim.split(output.stdout:gsub("\n$", ""), "\n", { plain = true }))
    else
      local reason = launch_error
      if output then
        local stderr = colors.parse((output.stderr or ""):gsub("\n+$", "")).text
        if stderr:find("%S") then
          reason = stderr
        elseif output.code == 0 then
          reason = "mermaid-text 未输出字符图"
        else
          reason = "mermaid-text 退出码 " .. output.code
        end
      end
      result.failure = vim.split(reason .. "\n\n" .. source, "\n", { plain = true })
    end
    if result.scratch and api.nvim_buf_is_loaded(result.scratch) then
      write_scratch(result.scratch, result)
    end
    result.scratch = nil
    if buffers[buf] == view and view.active and view.tick == api.nvim_buf_get_changedtick(buf) then
      refresh(buf, view)
    end
  end
  local function start()
    if not (result.scratch and api.nvim_buf_is_loaded(result.scratch)) then
      if buffers[buf] ~= view or view.requests[key] ~= result then
        return
      end
      if not view.active or not view.wanted[key] or view.tick ~= api.nvim_buf_get_changedtick(buf) then
        view.requests[key] = nil
        return
      end
    end
    local cmd = { "mermaid-text", "--width", tostring(width) }
    if display.border_corners then
      vim.list_extend(cmd, { "--border-corners", display.border_corners })
    end
    if display.ascii then
      cmd[#cmd + 1] = "--ascii"
    end
    if display.color == false then
      cmd[#cmd + 1] = "--no-color"
    end
    local function on_exit(output)
      result.job = nil
      running = running - 1
      finish(output)
      run_jobs()
    end
    local ok, job = pcall(vim.system, cmd, { stdin = source, text = true }, vim.schedule_wrap(on_exit))
    if ok then
      result.job = job
      running = running + 1
    else
      finish(nil, tostring(job))
    end
  end
  -- 等窗口尺寸稳定后再启动；窗口缩放动画会连续改变可用宽度。
  vim.defer_fn(function()
    queue[#queue + 1] = start
    run_jobs()
  end, 100)
  return result
end

local function text_width(info)
  return info.width - info.textoff
end

local function target_width(available)
  return math.max(1, math.min(available, display.max_width or available))
end

local function clear_window(buf, displayed)
  api.nvim_buf_clear_namespace(buf, displayed.ns, 0, -1)
  colors.release(displayed.palette)
end

local function clear_windows(buf, view)
  for _, displayed in pairs(view.windows) do
    clear_window(buf, displayed)
  end
  view.windows = {}
end

refresh = function(buf, view)
  local used, wanted, windows = {}, {}, {}
  for _, win in ipairs(vim.fn.win_findbuf(buf)) do
    local info = vim.fn.getwininfo(win)[1]
    local columns = text_width(info)
    local cursor = api.nvim_win_get_cursor(win)[1] - 1
    local top = info.topline - 1
    local bottom = info.botline == 0 and top + info.height - 1 or info.botline - 1
    local shown = {}
    for _, block in ipairs(view.blocks) do
      local available = columns - block.indent
      local width = target_width(available)
      local key = width .. ":" .. block.source
      used[key] = true
      if available > 0 and (cursor < block.first or cursor > block.last) then
        local result = view.requests[key]
        if block.last >= top and block.first <= bottom then
          wanted[key] = true
          if not result and vim.fn.executable("mermaid-text") == 1 then
            view.requests[key] = queue_render(buf, view, key, block.source, width)
          end
        end
        if result and result.lines then
          shown[block] = result.lines
        end
      end
    end
    local previous = view.windows[win]
    if next(shown) then
      if not previous or previous.width ~= columns or not vim.deep_equal(shown, previous.shown) then
        local ns = previous and previous.ns or api.nvim_create_namespace("markdown-mermaid-" .. win)
        if not previous then
          api.nvim__ns_set(ns, { wins = { win } })
        end
        api.nvim_buf_clear_namespace(buf, ns, 0, -1)
        local palette = {}
        api.nvim_win_call(win, function()
          for block, lines in pairs(shown) do
            decorate(buf, ns, block, lines, columns - block.indent, palette)
          end
        end)
        if previous then
          colors.release(previous.palette)
        end
        windows[win] = { ns = ns, width = columns, shown = shown, palette = palette }
      else
        windows[win] = previous
      end
    end
  end
  for win, displayed in pairs(view.windows) do
    if not windows[win] then
      clear_window(buf, displayed)
    end
  end
  for key, result in pairs(view.requests) do
    if not used[key] and not result.job and not (result.scratch and api.nvim_buf_is_loaded(result.scratch)) then
      view.requests[key] = nil
    end
  end
  view.wanted, view.windows = wanted, windows
end

local function get_view(buf)
  local view = buffers[buf]
  local tick = api.nvim_buf_get_changedtick(buf)
  if not view or view.tick ~= tick then
    local parser = vim.treesitter.get_parser(buf, "markdown")
    if view then
      clear_windows(buf, view)
    else
      view = { requests = {}, windows = {} }
    end
    view.tick = tick
    view.wanted = {}
    view.blocks = collect(buf, parser)
    buffers[buf] = view
  end
  return view
end

function M.attach(ctx)
  local buf = ctx.buf
  if vim.bo[buf].filetype ~= "markdown" then
    return
  end

  api.nvim_create_autocmd({ "TextChanged", "TextChangedI", "TextChangedP" }, {
    buffer = buf,
    callback = function()
      local view = buffers[buf]
      if view and view.active and #view.blocks > 0 and view.tick ~= api.nvim_buf_get_changedtick(buf) then
        M.render(ctx)
      end
    end,
  })

  api.nvim_buf_create_user_command(buf, "MermaidView", function()
    local view = get_view(buf)
    local row = api.nvim_win_get_cursor(0)[1] - 1
    for _, block in ipairs(view.blocks) do
      if row >= block.first and row <= block.last then
        local columns = text_width(vim.fn.getwininfo(api.nvim_get_current_win())[1])
        local width = target_width(columns - block.indent)
        local key = width .. ":" .. block.source
        local result = view.requests[key]
        if not result then
          if vim.fn.executable("mermaid-text") ~= 1 then
            vim.notify("找不到 mermaid-text", vim.log.levels.WARN)
            return
          end
          result = queue_render(buf, view, key, block.source, width)
          view.requests[key] = result
        end
        if result.lines or result.failure then
          open_scratch(result)
        else
          if result.scratch and not api.nvim_buf_is_loaded(result.scratch) then
            result.scratch = nil
          end
          result.scratch = open_scratch(result)
        end
        return
      end
    end
    vim.notify("请把光标移到 Mermaid 围栏内", vim.log.levels.WARN)
  end, { desc = "Open Mermaid diagram in a horizontally scrollable split" })

  api.nvim_buf_create_user_command(buf, "MermaidImage", function()
    local row = api.nvim_win_get_cursor(0)[1] - 1
    for _, block in ipairs(collect(buf, vim.treesitter.get_parser(buf, "markdown"))) do
      if row >= block.first and row <= block.last then
        require("utils.markdown_mermaid_image").open(block.source, display.color)
        return
      end
    end
    vim.notify("请把光标移到 Mermaid 围栏内", vim.log.levels.WARN)
  end, { desc = "Open Mermaid diagram as an image" })
end

function M.render(ctx)
  local buf = ctx.buf
  if vim.bo[buf].filetype ~= "markdown" then
    return
  end
  local view = get_view(buf)
  view.active = true
  if #view.blocks > 0 or next(view.windows) then
    refresh(buf, view)
  end
end

function M.clear(ctx)
  local buf = ctx.buf
  local view = buffers[buf]
  if not view or not view.active then
    return
  end
  view.active = false
  -- render-markdown 每次重解析会先调用 clear，再同步调用 render。
  vim.schedule(function()
    if buffers[buf] == view and not view.active then
      clear_windows(buf, view)
    end
  end)
end

function M.disable()
  for buf, view in pairs(buffers) do
    view.active = false
    clear_windows(buf, view)
  end
end

api.nvim_create_autocmd({ "BufWinEnter", "WinClosed" }, {
  callback = function(event)
    local closed = event.event == "WinClosed" and tonumber(event.match)
    for buf, view in pairs(buffers) do
      for win, displayed in pairs(view.windows) do
        if win == closed or not api.nvim_win_is_valid(win) or api.nvim_win_get_buf(win) ~= buf then
          clear_window(buf, displayed)
          view.windows[win] = nil
        end
      end
    end
  end,
})

api.nvim_create_autocmd("BufUnload", {
  callback = function(event)
    if scratch_palettes[event.buf] then
      api.nvim_buf_clear_namespace(event.buf, scratch_ns, 0, -1)
      colors.release(scratch_palettes[event.buf])
      scratch_palettes[event.buf] = nil
    end
    local view = buffers[event.buf]
    if view then
      clear_windows(event.buf, view)
    end
    buffers[event.buf] = nil
    for _, result in pairs(view and view.requests or {}) do
      local preview = result.scratch and api.nvim_buf_is_loaded(result.scratch)
      if result.job and not result.job:is_closing() and not preview then
        result.job:kill(15)
      end
    end
  end,
})

return M
