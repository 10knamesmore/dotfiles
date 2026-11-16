rex.bind("f2", "client.tab.rename")
rex.bind("ctrl+shift+r", "client.tab.rename")
rex.bind("ctrl+shift+enter", "pane.split", { direction = "auto" })
rex.bind("ctrl+shift+t", "client.tab.new")
rex.bind("ctrl+shift+left", "client.tab.previous")
rex.bind("ctrl+shift+right", "client.tab.next")
rex.bind("ctrl+shift+[", "pane.focus_previous")
rex.bind("ctrl+shift+]", "pane.focus_next")
rex.bind("ctrl+shift+n", "client.window.new")
rex.bind("alt+backspace", "pane.send_key", { key = "delete" })

local function active_block(view)
  for _, window in ipairs(view.windows or {}) do
    if window.window_id == view.active_window_id then
      return window.focused_block_id
    end
  end
end

local function focus_or_yabai(pane_direction, window_direction)
  return function()
    local before, err = rex.session.view{}
    if err ~= nil then
      error(err, 0)
    end

    _, err = rex.session.focus_direction{
      direction = pane_direction,
      wrap = false,
    }
    if err ~= nil then
      error(err, 0)
    end

    local after, err = rex.session.view{}
    if err ~= nil then
      error(err, 0)
    end

    if active_block(before) ~= active_block(after) then
      return true
    end

    local result, exec_error = rex.exec({
      "/opt/homebrew/bin/yabai",
      "-m",
      "window",
      "--focus",
      window_direction,
    })
    if result == nil then
      error(exec_error, 0)
    end
    if not result.ok then
      rex.log("warn", result.stderr)
    end

    return true
  end
end

rex.bind("cmd+h", focus_or_yabai("left", "west"))
rex.bind("cmd+j", focus_or_yabai("down", "south"))
rex.bind("cmd+k", focus_or_yabai("up", "north"))
rex.bind("cmd+l", focus_or_yabai("right", "east"))
