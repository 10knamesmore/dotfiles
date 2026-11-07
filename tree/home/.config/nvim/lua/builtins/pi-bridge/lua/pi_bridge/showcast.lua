--- A connected Pi publishes a tour in the current window; Neovim owns navigation and comments.
local M = {}
local api = vim.api
local diff = require("pi_bridge.diff")
local namespace = api.nvim_create_namespace("pi_showcast")
local active
local finish
local visit
local render_comment
local position_comment

---@class PiShowcastStop
---@field buffer integer|string Buffer ID or absolute file path
---@field position integer[] {1-based row, 0-based UTF-8 byte column}
---@field title string
---@field comment string Markdown
---@field highlights? {start: integer[], ["end"]: integer[]}[] Half-open ranges, same units as position
---@field proposed_text? string Complete proposed buffer contents, separated by LF

local function close_comment(tour)
  local window = tour.comment_window
  tour.comment_window = nil
  if window then
    api.nvim_win_close(window, true)
  end
end

local function clear_station(tour)
  if tour.buffer and api.nvim_buf_is_loaded(tour.buffer) then
    diff.clear(tour.buffer)
    api.nvim_buf_clear_namespace(tour.buffer, namespace, 0, -1)
    api.nvim_buf_call(tour.buffer, function()
      for key, previous in pairs(tour.mappings) do
        vim.keymap.del("n", key, { buffer = tour.buffer })
        if previous.buffer == 1 then
          vim.fn.mapset("n", false, previous)
        end
      end
    end)
  end
  tour.mappings = {}
  tour.buffer = nil
end

local function refresh_diff(tour)
  diff.clear(tour.buffer)
  local stop = tour.stops[tour.index]
  if stop.preview and api.nvim_buf_get_changedtick(tour.buffer) ~= stop.preview.changedtick then
    stop.preview = nil
  end
  if stop.preview and tour.diff_visible then
    diff.show(stop.preview)
  end
end

local function toggle_diff()
  local tour = active
  if not tour or not tour.stops[tour.index].preview then
    return
  end
  tour.diff_visible = not tour.diff_visible
  refresh_diff(tour)
  position_comment(tour)
end

local function toggle_comment()
  if not active then
    return
  end
  active.comment_visible = not active.comment_visible
  if active.comment_visible and not active.comment_window then
    render_comment(active)
  else
    position_comment(active)
  end
end

local function focus_comment()
  if not active then
    return
  end
  if not active.comment_visible then
    toggle_comment()
  end
  if not api.nvim_win_get_config(active.comment_window).hide then
    api.nvim_set_current_win(active.comment_window)
  end
end

local function bind_keys(tour, buffer, remember)
  local bindings = {
    {
      "n",
      function()
        visit(tour.index + 1)
      end,
      "Showcast: next stop",
    },
    {
      "N",
      function()
        visit(tour.index - 1)
      end,
      "Showcast: previous stop",
    },
    { "<Tab>", toggle_comment, "Showcast: toggle comment" },
    { "<leader>up", toggle_diff, "Showcast: toggle diff preview" },
    { "<CR>", focus_comment, "Showcast: focus comment" },
    { "q", finish, "Showcast: end tour" },
  }
  api.nvim_buf_call(buffer, function()
    for _, binding in ipairs(bindings) do
      if remember then
        tour.mappings[binding[1]] = vim.fn.maparg(binding[1], "n", false, true)
      end
      vim.keymap.set("n", binding[1], binding[2], { buffer = buffer, silent = true, desc = binding[3] })
    end
  end)
end

--- Floating-window rows start below the code window's winbar.
--- Use live extmarks and screenpos so wrapping, folds and UTF-8 columns follow Neovim.
local function highlight_bounds(tour, viewport)
  local top, bottom, left
  local function include(first_row, first_col, last_row, last_col)
    local first = vim.fn.screenpos(tour.window, first_row, first_col)
    local last = vim.fn.screenpos(tour.window, last_row, last_col)
    if (first.row == 0 and first_row > viewport.botline) or (last.row == 0 and last_row < viewport.topline) then
      return
    end
    local first_visible = first.row > 0 and first.row - viewport.winrow - viewport.winbar or 0
    local last_visible = last.row > 0 and last.row - viewport.winrow - viewport.winbar or viewport.height - 1
    local column = first.col > 0 and first.col - viewport.wincol or viewport.textoff
    top = math.min(top or first_visible, first_visible)
    bottom = math.max(bottom or last_visible, last_visible)
    left = math.min(left or column, column)
  end
  local marks = api.nvim_buf_get_extmarks(tour.buffer, namespace, 0, -1, { details = true })
  for _, mark in ipairs(marks) do
    local first_row, first_col, details = mark[2] + 1, mark[3] + 1, mark[4]
    local last_row, last_col = details.end_row + 1, details.end_col
    -- An exclusive end at column zero belongs to the previous line.
    if last_col == 0 and last_row > first_row then
      last_row = last_row - 1
      last_col = #api.nvim_buf_get_lines(tour.buffer, last_row - 1, last_row, true)[1]
    end
    include(first_row, first_col, last_row, math.max(1, last_col))
  end
  if #marks == 0 then
    local location = tour.stops[tour.index].position
    include(location[1], location[2] + 1, location[1], location[2] + 1)
  end
  for _, span in ipairs(diff.screen_spans(tour.buffer, tour.window)) do
    local first = span.top - viewport.winrow - viewport.winbar
    local last = span.bottom - viewport.winrow - viewport.winbar
    if last >= 0 and first < viewport.height then
      top = math.min(top or math.huge, math.max(0, first))
      bottom = math.max(bottom or -1, math.min(viewport.height - 1, last))
      left = math.min(left or viewport.textoff, span.left - viewport.wincol)
    end
  end
  if top then
    return { top = top, bottom = bottom, left = left }
  end
end

local function hide_comment(tour)
  if api.nvim_get_current_win() == tour.comment_window then
    api.nvim_set_current_win(tour.window)
  end
  api.nvim_win_set_config(tour.comment_window, { hide = true })
end

position_comment = function(tour)
  if not tour.comment_window then
    return
  end
  if not tour.comment_visible or api.nvim_win_get_buf(tour.window) ~= tour.buffer then
    hide_comment(tour)
    return
  end
  -- screenpos reads rendered line offsets; refresh after buffer/layout changes before measuring.
  vim.cmd.redraw()
  local viewport = vim.fn.getwininfo(tour.window)[1]
  local bounds = highlight_bounds(tour, viewport)
  if not bounds or viewport.width < 3 then
    hide_comment(tour)
    return
  end
  local width = math.min(76, viewport.width - 2)
  local config = api.nvim_win_get_config(tour.comment_window)
  if config.width ~= width then
    api.nvim_win_set_width(tour.comment_window, width)
  end
  local wanted_height =
    math.min(math.max(1, math.floor(viewport.height * 0.4)), api.nvim_win_text_height(tour.comment_window, {}).all)
  local above = bounds.top
  local below = viewport.height - bounds.bottom - 1
  local use_below = below >= wanted_height + 2 or (above < wanted_height + 2 and below >= above)
  local available = use_below and below or above
  -- Keep both borders outside the highlight. With no room, wait for the viewport to move.
  if available < 3 then
    hide_comment(tour)
    return
  end
  local height = math.min(wanted_height, available - 2)
  local row = use_below and bounds.bottom + 1 or bounds.top - height - 2
  local col = math.max(0, math.min(bounds.left - 1, viewport.width - width - 2))
  if config.hide or config.row ~= row or config.col ~= col or config.height ~= height then
    api.nvim_win_set_config(tour.comment_window, {
      relative = "win",
      win = tour.window,
      anchor = "NW",
      row = row,
      col = col,
      width = width,
      height = height,
      hide = false,
    })
  end
end

render_comment = function(tour)
  local stop = tour.stops[tour.index]
  local buffer = api.nvim_create_buf(false, true)
  vim.bo[buffer].bufhidden = "wipe"
  api.nvim_buf_set_lines(buffer, 0, -1, false, vim.split("# " .. stop.title .. "\n\n" .. stop.comment, "\n"))
  vim.bo[buffer].filetype = "markdown"
  vim.bo[buffer].modifiable = false
  local window = api.nvim_open_win(buffer, false, {
    relative = "win",
    win = tour.window,
    anchor = "NW",
    row = 0,
    col = 0,
    width = math.max(1, math.min(76, api.nvim_win_get_width(tour.window) - 2)),
    height = 1,
    hide = true,
    style = "minimal",
    border = "rounded",
    title = string.format(" %s · %d/%d ", tour.title, tour.index, #tour.stops),
    title_pos = "center",
    footer = " n/N · Tab comment · <leader>up diff · q end ",
    footer_pos = "center",
  })
  tour.comment_window = window
  api.nvim_create_autocmd("WinClosed", {
    group = tour.events,
    pattern = tostring(window),
    once = true,
    callback = function()
      if tour.comment_window == window then
        tour.comment_window = nil
        tour.comment_visible = false
      end
    end,
  })
  vim.wo[window].wrap = true
  vim.wo[window].linebreak = true
  vim.wo[window].conceallevel = 2
  bind_keys(tour, buffer, false)
  vim.keymap.set("n", "<Esc>", function()
    api.nvim_set_current_win(tour.window)
  end, { buffer = buffer, desc = "Showcast: return to code" })
  position_comment(tour)
end

visit = function(index)
  local tour = active
  if not tour or index < 1 or index > #tour.stops then
    return
  end
  close_comment(tour)
  clear_station(tour)
  tour.index = index
  local stop = tour.stops[index]
  api.nvim_set_current_win(tour.window)
  api.nvim_win_set_buf(tour.window, stop.buffer)
  tour.buffer = stop.buffer
  bind_keys(tour, tour.buffer, true)
  refresh_diff(tour)
  api.nvim_win_set_cursor(tour.window, stop.position)
  vim.cmd("normal! zvzz")
  for _, range in ipairs(stop.highlights or {}) do
    api.nvim_buf_set_extmark(tour.buffer, namespace, range.start[1] - 1, range.start[2], {
      end_row = range["end"][1] - 1,
      end_col = range["end"][2],
      hl_group = "PiShowcastRange",
      priority = 200,
      strict = true,
    })
  end
  vim.wo[tour.window].winbar = string.format(
    "Showcast %d/%d · %s · n/N stops · Tab comment · <leader>up diff · q end",
    index,
    #tour.stops,
    tour.title:gsub("%%", "%%%%"):gsub("[%c]", " ")
  )
  render_comment(tour)
end

finish = function()
  local tour = active
  if not tour then
    return
  end
  active = nil
  api.nvim_del_augroup_by_id(tour.events)
  close_comment(tour)
  clear_station(tour)
  if api.nvim_win_is_valid(tour.window) then
    local original_buffer_exists = api.nvim_buf_is_loaded(tour.origin.buffer)
    if original_buffer_exists then
      api.nvim_win_set_buf(tour.window, tour.origin.buffer)
    end
    vim.wo[tour.window].winbar = tour.origin.winbar
    if original_buffer_exists then
      api.nvim_win_call(tour.window, function()
        vim.fn.winrestview(tour.origin.view)
      end)
    end
  end
end

--- Reuse the current window and restore its buffer, view and winbar when the tour ends.
---@param channel integer
---@param title string
---@param stops PiShowcastStop[]
function M.start(channel, title, stops)
  require("pi_bridge").require_owner(channel)
  assert(#stops > 0, "Showcast needs at least one stop")
  stops = vim.deepcopy(stops)
  for _, stop in ipairs(stops) do
    if type(stop.buffer) == "string" then
      assert(stop.buffer:sub(1, 1) == "/", "Showcast file paths must be absolute")
      stop.buffer = vim.fn.bufadd(stop.buffer)
    end
    vim.fn.bufload(stop.buffer)
    if stop.proposed_text ~= nil then
      stop.preview = diff.prepare(stop.buffer, stop.proposed_text)
    end
  end
  finish()
  local tour = {
    title = title,
    stops = stops,
    index = 1,
    window = api.nvim_get_current_win(),
    origin = {
      buffer = api.nvim_get_current_buf(),
      view = vim.fn.winsaveview(),
      winbar = vim.wo.winbar,
    },
    mappings = {},
    comment_visible = true,
    diff_visible = true,
    events = api.nvim_create_augroup("PiShowcast", { clear = true }),
  }
  active = tour
  api.nvim_set_hl(0, "PiShowcastRange", { default = true, link = "Visual" })
  local watched = {}
  for _, stop in ipairs(stops) do
    if stop.preview and not watched[stop.buffer] then
      local buffer = stop.buffer
      watched[buffer] = true
      api.nvim_create_autocmd({ "TextChanged", "TextChangedI", "TextChangedP", "BufUnload" }, {
        group = tour.events,
        buffer = buffer,
        callback = function(event)
          for _, station in ipairs(tour.stops) do
            if
              station.buffer == buffer
              and station.preview
              and (event.event == "BufUnload" or api.nvim_buf_get_changedtick(buffer) ~= station.preview.changedtick)
            then
              station.preview = nil
            end
          end
          if tour.buffer == buffer then
            diff.clear(buffer)
            if event.event ~= "BufUnload" then
              refresh_diff(tour)
              position_comment(tour)
            end
          end
        end,
      })
    end
  end
  api.nvim_create_autocmd("WinClosed", {
    group = tour.events,
    pattern = tostring(tour.window),
    callback = function()
      vim.schedule(function()
        if active == tour then
          finish()
        end
      end)
    end,
  })
  api.nvim_create_autocmd("WinScrolled", {
    group = tour.events,
    callback = function()
      if vim.v.event[tostring(tour.window)] and api.nvim_win_is_valid(tour.window) then
        position_comment(tour)
      end
    end,
  })
  local ok, err = pcall(visit, 1)
  if not ok then
    finish()
    error(err, 0)
  end
end

function M.stop(channel)
  require("pi_bridge").require_owner(channel)
  finish()
end

function M.toggle_comment(channel)
  require("pi_bridge").require_owner(channel)
  toggle_comment()
end

function M.toggle_diff(channel)
  require("pi_bridge").require_owner(channel)
  toggle_diff()
end

--- Called when the owner leaves; user navigation never requires a Python round trip.
function M.release()
  finish()
end

return M
