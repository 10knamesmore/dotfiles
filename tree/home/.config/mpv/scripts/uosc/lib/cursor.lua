---@alias CursorEventHandler fun(shortcut: Shortcut)

local cursor = {
	x = math.huge,
	y = math.huge,
	hidden = true,
	distance = 0, -- 当前移动累计的位移距离。由 `cursor.distance_reset_timer` 重置。
	last_hover = false, -- 保存上一次鼠标事件的 `mouse.hover` 布尔值，用于进入/离开检测。
	-- 仅在渲染循环期间定义的区域(zone)上触发的事件处理器。
	---@type {event: string, hitbox: Hitbox; handler: CursorEventHandler}[]
	zones = {},
	handlers = {
		primary_down = {},
		primary_up = {},
		secondary_down = {},
		secondary_up = {},
		wheel_down = {},
		wheel_up = {},
		move = {},
	},
	first_real_mouse_move_received = false,
	history = CircularBuffer:new(10),
	autohide_fs_only = nil,
	-- 跟踪每个事件当前的按键绑定级别。0: 禁用, 1: 启用, 2: 启用 + 阻止窗口拖动
	binding_levels = {
		mbtn_left = 0,
		mbtn_left_dbl = 0,
		mbtn_right = 0,
		wheel = 0,
	},
	is_dragging_prevented = false,
	event_forward_map = {
		primary_down = 'MBTN_LEFT',
		primary_up = 'MBTN_LEFT',
		secondary_down = 'MBTN_RIGHT',
		secondary_up = 'MBTN_RIGHT',
		wheel_down = 'WHEEL_DOWN',
		wheel_up = 'WHEEL_UP',
	},
	event_binding_map = {
		primary_down = 'mbtn_left',
		primary_up = 'mbtn_left',
		primary_click = 'mbtn_left',
		secondary_down = 'mbtn_right',
		secondary_up = 'mbtn_right',
		secondary_click = 'mbtn_right',
		wheel_down = 'wheel',
		wheel_up = 'wheel',
	},
	window_dragging_blockers = create_set({'primary_click', 'primary_down'}),
	event_propagation_blockers = {
		primary_down = 'primary_click',
		primary_click = 'primary_down',
		secondary_down = 'secondary_click',
		secondary_click = 'secondary_down',
	},
	event_meta = {
		primary_down = {is_start = true, trigger_event = 'primary_click'},
		primary_up = {is_end = true, start_event = 'primary_down', trigger_event = 'primary_click'},
		secondary_down = {is_start = true, trigger_event = 'secondary_click'},
		secondary_up = {is_end = true, start_event = 'secondary_down', trigger_event = 'secondary_click'},
	},
	-- 保存起始事件的位置和时间(即开启复合事件的事件，如 click)。
	---@type {[string]: {x: number, y: number, time: number, zone_handled: boolean}}
	last_events = {},
}

cursor.autohide_timer = mp.add_timeout(1, function() cursor:autohide() end)
cursor.autohide_timer:kill()
mp.observe_property('cursor-autohide', 'number', function(_, val)
	cursor.autohide_timer.timeout = (val or 1000) / 1000
end)

cursor.distance_reset_timer = mp.add_timeout(0.2, function()
	cursor.distance = 0
	request_render()
end)
cursor.distance_reset_timer:kill()

-- 在每次渲染开始时调用
function cursor:clear_zones()
	itable_clear(self.zones)
end

---@param hitbox Hitbox
function cursor:collides_with(hitbox)
	return point_collides_with(self, hitbox)
end

-- 返回当前光标位置上该事件对应的区域(zone)。
---@param event string
function cursor:find_zone(event)
	-- 提前优化：忽略一个目前不需要作为区域处理的高频事件。
	if event == 'move' then return end

	for i = #self.zones, 1, -1 do
		local zone = self.zones[i]
		local is_blocking_only = zone.event == self.event_propagation_blockers[event]
		if (zone.event == event or is_blocking_only) and self:collides_with(zone.hitbox) then
			return not is_blocking_only and zone or nil
		end
	end
end

-- 为当前已渲染屏幕上某个 hitbox 定义一个事件区域(zone)。可用事件:
-- - primary_down, primary_up, primary_click, secondary_down, secondary_up, secondary_click, wheel_down, wheel_up
--
-- 注意:
-- - 区域在每次 `render()` 开始时被清空，需要重新绑定。
-- - 每个区域只能对应一种事件类型: 同一事件只有最后绑定的区域会被触发。
-- - 在当前实现中，你必须在 `_click` 或 `_down` 之间二选一。两者都绑定时只有最后绑定的会触发。
-- - 主键(primary)的 `_down` 和 `_click` 会禁用拖动。在 hitbox 上定义 `window_drag = true` 可重新启用。
-- - 任何禁用拖动的行为也会隐式禁用光标自动隐藏。
-- - `move` 事件区域会被忽略，因为它是高频事件，目前不需要作为区域处理。
---@param event string
---@param hitbox Hitbox
---@param callback CursorEventHandler
function cursor:zone(event, hitbox, callback)
	self.zones[#self.zones + 1] = {event = event, hitbox = hitbox, handler = callback}
end

-- 绑定一个永久的光标事件处理器，直到用 `cursor:off()` 手动解绑前一直有效。
-- `_click` 事件不能作为永久全局事件，只能作为区域(zone)使用。
---@param event string
---@param callback CursorEventHandler
---@return fun() disposer 解绑该事件。
function cursor:on(event, callback)
	if self.handlers[event] and not itable_index_of(self.handlers[event], callback) then
		self.handlers[event][#self.handlers[event] + 1] = callback
		self:decide_keybinds()
	end
	return function() self:off(event, callback) end
end

-- 解绑一个光标事件处理器。
---@param event string
function cursor:off(event, callback)
	if self.handlers[event] then
		local index = itable_index_of(self.handlers[event], callback)
		if index then
			table.remove(self.handlers[event], index)
			self:decide_keybinds()
		end
	end
end

-- 绑定一个只调用一次的光标事件处理器。
---@param event string
function cursor:once(event, callback)
	local function callback_wrap()
		callback()
		self:off(event, callback_wrap)
	end
	return self:on(event, callback_wrap)
end

-- 触发该事件。
---@param event string
---@param shortcut? Shortcut
function cursor:trigger(event, shortcut)
	local forward, zone_handled = true, false

	-- 调用原始事件处理器。
	local zone = self:find_zone(event)
	local callbacks = self.handlers[event]
	if zone or #callbacks > 0 then
		forward = false
		if zone and shortcut then
			zone.handler(shortcut)
			zone_handled = true
		end
		for _, callback in ipairs(callbacks) do callback(shortcut) end
	end

	if event ~= 'move' then
		-- 如果起始事件和结束事件都落在 `parent_zone.hitbox` 内，则调用复合/父级(click)事件处理器。
		local meta = self.event_meta[event]
		if meta then
			-- 触发复合事件
			local parent_zone = self:find_zone(meta.trigger_event)
			if parent_zone then
				forward = false -- 在此取消，这样当 down 事件可能导向 click 时就不会向下转发。
				if meta.is_end then
					local start_event = self.last_events[meta.start_event]
					if start_event and point_collides_with(start_event, parent_zone.hitbox) and shortcut then
						parent_zone.handler(create_shortcut('primary_click', shortcut.modifiers))
					end
				end
			end
		end

		-- 转发未被处理的事件。
		if forward then
			local forward_name = self.event_forward_map[event]
			local last_down = meta and meta.is_end and self.last_events[meta.start_event]
			local down_zone_handled = last_down and last_down.zone_handled
			if forward_name and not down_zone_handled then
				-- 如果没有处理器，则转发事件。
				local active = find_active_keybindings(forward_name)
				if active and active.cmd then
					local is_wheel = event:find('wheel', 1, true)
					local is_up = event:sub(-3) == '_up'
					if active.owner then
						-- 绑定属于其它脚本，因此让它看起来像普通按键事件。
						-- 鼠标绑定很简单，其它按键则需要 repeat 和 pressed 处理，
						-- 这无法通过 mp.set_key_bindings() 实现，但可以用 mp.add_key_binding() 做到。
						local state = is_wheel and 'pm' or is_up and 'um' or 'dm'
						local name = active.cmd:sub(active.cmd:find('/') + 1, -1)
						mp.commandv('script-message-to', active.owner, 'key-binding', name, state, forward_name)
					elseif is_wheel or is_up then
						-- input.conf 绑定，对鼠标按键的释放(release)做出反应
						mp.command(active.cmd)
					end
				end
			end
		end
	end

	-- 记录最近一次的事件
	local last = self.last_events[event] or {}
	last.x, last.y, last.time, last.zone_handled = self.x, self.y, mp.get_time(), zone_handled
	self.last_events[event] = last

	-- 刷新光标自动隐藏计时器。
	self:queue_autohide()
end

-- 根据已绑定的事件监听器，启用或禁用按键绑定组。
function cursor:decide_keybinds()
	local new_levels = {mbtn_left = 0, mbtn_right = 0, wheel = 0}
	self.is_dragging_prevented = false

	-- 检查全局事件。
	for name, handlers in ipairs(self.handlers) do
		local binding = self.event_binding_map[name]
		if binding then
			new_levels[binding] = math.max(new_levels[binding], #handlers > 0 and 1 or 0)
		end
	end

	-- 检查区域(zone)。
	for _, zone in ipairs(self.zones) do
		local binding = self.event_binding_map[zone.event]
		if binding and cursor:collides_with(zone.hitbox) then
			local new_level = (self.window_dragging_blockers[zone.event] and zone.hitbox.window_drag ~= true) and 2
				or math.max(new_levels[binding], zone.hitbox.window_drag == false and 2 or 1)

			-- 只有当光标位于可拖动元素之上时，才允许使用阻止拖动的级别，
			-- 否则会破坏窗口拖动。这意味着触摸设备需要先点一下可拖动元素
			-- 才能开始拖动它。目前想不到绕开这个限制的办法。
			if new_level > 1 and not cursor:collides_with(zone.hitbox) then
				new_level = 1
			end

			new_levels[binding] = math.max(new_levels[binding], new_level)
			if new_level > 1 then
				self.is_dragging_prevented = true
			end
		end
	end

	-- 只有位于元素之上时才会阻止窗口拖动，而这正是应当忽略双击的时机。
	new_levels.mbtn_left_dbl = new_levels.mbtn_left == 2 and 2 or 0

	for name, level in pairs(new_levels) do
		if level ~= self.binding_levels[name] then
			local flags = level == 1 and 'allow-vo-dragging+allow-hide-cursor' or ''
			mp[(level == 0 and 'disable' or 'enable') .. '_key_bindings'](name, flags)
			self.binding_levels[name] = level
			self:queue_autohide()
		end
	end
end

function cursor:_find_history_sample()
	local time = mp.get_time()
	for _, e in self.history:iter_rev() do
		if time - e.time > 0.1 then
			return e
		end
	end
	return self.history:tail()
end

-- 返回当前速度向量，单位为像素每秒。
---@return Point
function cursor:get_velocity()
	local snap = self:_find_history_sample()
	if snap then
		local x, y, time = self.x - snap.x, self.y - snap.y, mp.get_time()
		local time_diff = time - snap.time
		if time_diff > 0.001 then
			return {x = x / time_diff, y = y / time_diff}
		end
	end
	return {x = 0, y = 0}
end

---@param x integer
---@param y integer
function cursor:move(x, y)
	local old_x, old_y = self.x, self.y

	-- 在 Linux 上 mpv 上报的初始鼠标位置为 (0, 0)，这总会
	-- 显示顶栏，因此我们把光标位置硬编码为无穷大，直到
	-- 收到第一个坐标不为 0,0 的真实鼠标移动事件为止。
	if not self.first_real_mouse_move_received then
		if x > 0 and y > 0 and x < 99999999 and y < 99999999 then
			self.first_real_mouse_move_received = true
		else
			x, y = math.huge, math.huge
		end
	end

	-- 加 0.5 使其位于像素中心
	self.x, self.y = x + 0.5, y + 0.5

	if old_x ~= self.x or old_y ~= self.y then
		if self.x == math.huge or self.y == math.huge then
			self.hidden = true
			self.history:clear()

			-- 缓慢淡出当前可见的元素
			for _, id in ipairs(config.cursor_leave_fadeout_elements) do
				local element = Elements[id]
				if element then
					local visibility = element:get_visibility()
					if visibility > 0 then
						element:tween_property('forced_visibility', visibility, 0, function()
							element.forced_visibility = nil
						end)
					end
				end
			end

			Elements:update_proximities()
			Elements:trigger('global_mouse_leave')
		else
			if self.hidden then
				-- 取消可能正在进行的淡出
				for _, id in ipairs(config.cursor_leave_fadeout_elements) do
					if Elements[id] then Elements[id]:tween_stop() end
				end

				self.hidden = false
				Elements:trigger('global_mouse_enter')
			end

			-- 更新当前移动的累计位移距离
			-- 加入 `mp.get_time() - last.time < 0.5` 检查是为了忽略长时间静止后的第一个事件，
			-- 从而过滤掉因窗口被重新定位/缩放(例如打开另一个文件)导致的大幅跳变。
			local last = self.last_events.move
			if last and last.x < math.huge and last.y < math.huge and mp.get_time() - last.time < 0.5 then
				self.distance = self.distance + get_point_to_point_proximity(cursor, last)
				cursor.distance_reset_timer:kill()
				cursor.distance_reset_timer:resume()
			end

			Elements:update_proximities()
			-- 更新历史记录
			self.history:insert({x = self.x, y = self.y, time = mp.get_time()})
		end

		Elements:proximity_trigger('mouse_move')
		self:queue_autohide()
	end

	self:trigger('move')

	request_render()
end

function cursor:leave() self:move(math.huge, math.huge) end

function cursor:is_autohide_allowed()
	return options.autohide and (not self.autohide_fs_only or state.fullscreen)
		and not self.is_dragging_prevented
		and not Menu:is_open()
end
mp.observe_property('cursor-autohide-fs-only', 'bool', function(_, val) cursor.autohide_fs_only = val end)

-- 一段时间无操作后自动隐藏光标。
function cursor:autohide()
	if self:is_autohide_allowed() then
		self:leave()
		self.autohide_timer:kill()
	end
end

function cursor:queue_autohide()
	if self:is_autohide_allowed() then
		self.autohide_timer:kill()
		self.autohide_timer:resume()
	end
end

-- 计算若光标沿当前路径继续移动，到达矩形所需的距离。
-- 如果光标并未朝矩形方向移动，则返回 `nil`。
---@param rect Rect
function cursor:direction_to_rectangle_distance(rect)
	local prev = self:_find_history_sample()
	if not prev then return false end
	local end_x, end_y = self.x + (self.x - prev.x) * 1e10, self.y + (self.y - prev.y) * 1e10
	return get_ray_to_rectangle_distance(self.x, self.y, end_x, end_y, rect)
end

---@param event string
---@param shortcut Shortcut
---@param cb? fun(shortcut: Shortcut)
function cursor:create_handler(event, shortcut, cb)
	return function()
		if cb then cb(shortcut) end
		self:trigger(event, shortcut)
	end
end

-- 移动
local function handle_mouse_pos(_, mouse)
	if not mouse then return end
	if cursor.last_hover and not mouse.hover then
		cursor:leave()
	elseif not (cursor.last_hover == false and mouse.hover == false) then -- 过滤掉重复的鼠标移出(mouse out)事件
		cursor:move(mouse.x, mouse.y)
	end
	cursor.last_hover = mouse.hover
end

local function handle_touch_pos(_, touches)
	if not touches then return end
	local touch = touches[1]
	if touch then
		cursor:move(touch.x, touch.y)
	end
end

mp.observe_property('mouse-pos', 'native', handle_mouse_pos)
mp.observe_property('touch-pos', 'native', handle_touch_pos)

-- 按键绑定组
local modifiers = {nil, 'alt', 'alt+ctrl', 'alt+shift', 'alt+ctrl+shift', 'ctrl', 'ctrl+shift', 'shift'}
local primary_bindings = {}
for i = 1, #modifiers do
	local mods = modifiers[i]
	local mp_name = (mods and mods .. '+' or '') .. 'mbtn_left'
	primary_bindings[#primary_bindings + 1] = {
		mp_name,
		cursor:create_handler('primary_up', create_shortcut('primary_up', mods)),
		cursor:create_handler('primary_down', create_shortcut('primary_down', mods), function(...)
			handle_mouse_pos(nil, mp.get_property_native('mouse-pos'))
		end),
	}
end
mp.set_key_bindings(primary_bindings, 'mbtn_left', 'force')
mp.set_key_bindings({
	{'mbtn_left_dbl', 'ignore'},
}, 'mbtn_left_dbl', 'force')
mp.set_key_bindings({
	{
		'mbtn_right',
		cursor:create_handler('secondary_up', create_shortcut('secondary_up')),
		cursor:create_handler('secondary_down', create_shortcut('secondary_down')),
	},
}, 'mbtn_right', 'force')
mp.set_key_bindings({
	{'wheel_up', cursor:create_handler('wheel_up', create_shortcut('wheel_up'))},
	{'wheel_down', cursor:create_handler('wheel_down', create_shortcut('wheel_down'))},
}, 'wheel', 'force')

return cursor
