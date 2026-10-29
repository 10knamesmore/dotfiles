local windowActions = {}

local directions = {
  left = { x = -1, y = 0, column = "prev" },
  right = { x = 1, y = 0, column = "next" },
  up = { x = 0, y = -1 },
  down = { x = 0, y = 1 },
}

local function notifyLayoutChanged()
  hl.dispatch(hl.dsp.event("window-layout-changed"))
end

local function moveToNearestMonitor(window, direction)
  local current = window.monitor
  local centerX = current.x + current.width / 2
  local centerY = current.y + current.height / 2
  local nearest, nearestDistance

  for _, monitor in ipairs(hl.get_monitors()) do
    local dx = monitor.x + monitor.width / 2 - centerX
    local dy = monitor.y + monitor.height / 2 - centerY
    local distance = dx * dx + dy * dy
    if dx * direction.x + dy * direction.y > 0 and (not nearest or distance < nearestDistance) then
      nearest = monitor
      nearestDistance = distance
    end
  end

  if nearest then
    return hl.dispatch(hl.dsp.window.move({ monitor = nearest, window = window }))
  end
end

-- 一个定时器负责当前窗口的垂直调整；连按累加位移，切换焦点或移动窗口时结束。
local pendingResize
local resizeTimer

local function stopResize()
  pendingResize = nil
  resizeTimer:set_enabled(false)
end

local function resizeStep()
  local resize = pendingResize
  if hl.get_active_window() ~= resize.window then
    stopResize()
    return
  end

  local step = math.ceil(math.abs(resize.remaining) / resize.framesLeft)
  if resize.remaining < 0 then
    step = -step
  end
  local result = hl.dispatch(hl.dsp.window.resize({ x = 0, y = step, relative = true, window = resize.window }))
  resize.remaining = resize.remaining - step
  resize.framesLeft = resize.framesLeft - 1
  notifyLayoutChanged()
  if not result.ok or resize.framesLeft == 0 then
    stopResize()
  end
end

resizeTimer = hl.timer(resizeStep, { timeout = 8, type = "repeat" })
resizeTimer:set_enabled(false)

function windowActions.move(directionName)
  stopResize()
  local window = hl.get_active_window()
  if not window then
    return
  end

  local direction = directions[directionName]
  local layout = window.layout
  local scrolling = hl.get_config("general:layout") == "scrolling"

  if layout and layout.name == "scrolling" and direction.column then
    -- 多窗口列中拆出当前窗口；独占一列时并入相邻列。没有相邻列则跨屏。
    local result = hl.dispatch(hl.dsp.layout("consume_or_expel " .. direction.column))
    if result.ok then
      notifyLayoutChanged()
      return result
    end
    return moveToNearestMonitor(window, direction)
  end

  local before = window.at
  local monitorBefore = window.monitor
  local result = hl.dispatch(hl.dsp.window.move({ direction = directionName, window = window }))
  if not result.ok then
    return result
  end

  -- at 是动画目标坐标，dispatcher 返回后即可判断，不需要等待画面完成移动。
  local after = window.at
  if window.monitor == monitorBefore and before.x == after.x and before.y == after.y then
    if scrolling then
      return moveToNearestMonitor(window, direction)
    end
    return hl.dispatch(hl.dsp.window.swap({ direction = directionName, window = window }))
  end
  return result
end

function windowActions.resize(directionName)
  local window = hl.get_active_window()
  if not window then
    return
  end

  local direction = directions[directionName]
  local layout = window.layout
  if not layout or layout.name ~= "scrolling" then
    stopResize()
    return hl.dispatch(hl.dsp.window.resize({
      x = direction.x * 40,
      y = direction.y * 40,
      relative = true,
      window = window,
    }))
  end

  if direction.x ~= 0 then
    stopResize()
    local result = hl.dispatch(hl.dsp.layout("colresize " .. (direction.x < 0 and "-0.05" or "+0.05")))
    notifyLayoutChanged()
    return result
  end

  if #layout.column.windows == 1 then
    return
  end

  -- 顶部窗口调整下边界，其余窗口调整上边界；J 向下、K 向上。
  local pixels = direction.y * 50 * (layout.index_in_column == 0 and 1 or -1)
  if pendingResize and pendingResize.window == window then
    pendingResize.remaining = pendingResize.remaining + pixels
    pendingResize.framesLeft = 8
  else
    pendingResize = { window = window, remaining = pixels, framesLeft = 8 }
    resizeTimer:set_enabled(true)
    resizeStep()
  end
end

return windowActions
