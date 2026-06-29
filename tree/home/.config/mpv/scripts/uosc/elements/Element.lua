---@alias ElementProps {enabled?: boolean; render_order?: number; ax?: number; ay?: number; bx?: number; by?: number; ignores_curtain?: boolean; anchor_id?: string;}

-- 所有元素继承自的基类。
---@class Element : Class
local Element = class()

---@param id string
---@param props? ElementProps
function Element:init(id, props)
	self.id = id
	self.render_order = 1
	-- `false` 表示该元素不会被渲染，也不会接收事件
	self.enabled = true
	-- 元素坐标
	self.ax, self.ay, self.bx, self.by = 0, 0, 0, 0
	-- 相对接近度：`0` 表示鼠标在 `proximity_max` 范围之外，`1` 表示鼠标在 `proximity_min` 范围之内。
	self.proximity = 0
	-- 以像素为单位的原始接近度。
	self.proximity_raw = math.huge
	---@type number `0-1` 强制最小可见度的系数。用于切换元素的永久可见性。
	self.min_visibility = 0
	---@type number `0-1` 强制指定可见度值的系数。用于闪烁、淡出及其他动画
	self.forced_visibility = nil
	---@type boolean 即使 curtain 可见时也显示此元素。
	self.ignores_curtain = false
	---@type nil|string 此元素应继承其可见性的来源元素的 ID。
	self.anchor_id = nil
	---@type fun()[] 元素销毁时调用的析构函数。
	self._disposers = {}
	---@type table<string,table<string, boolean>> 带命名空间的活动按键绑定。默认命名空间是 `_`。
	self._key_bindings = {}

	if props then table_assign(self, props) end

	-- 闪烁计时器
	self._flash_out_timer = mp.add_timeout(options.flash_duration / 1000, function()
		local function getTo() return self.proximity end
		local function onTweenEnd() self.forced_visibility = nil end
		if self.enabled then
			self:tween_property('forced_visibility', self:get_visibility(), getTo, onTweenEnd)
		else
			onTweenEnd()
		end
	end)
	self._flash_out_timer:kill()

	Elements:add(self)
end

function Element:destroy()
	self:dispose()
	self.destroyed = true
	self:remove_key_bindings()
	Elements:remove(self)
end

-- 调用为此元素注册的所有析构函数（通常是 mpv 事件/property 观察器）。
function Element:dispose()
	for _, disposer in ipairs(self._disposers) do disposer() end
end

function Element:reset_proximity() self.proximity, self.proximity_raw = 0, math.huge end

---@param ax number
---@param ay number
---@param bx number
---@param by number
function Element:set_coordinates(ax, ay, bx, by)
	self.ax, self.ay, self.bx, self.by = ax, ay, bx, by
	Elements:update_proximities()
	self:maybe('on_coordinates')
end

function Element:update_proximity()
	if cursor.hidden then
		self:reset_proximity()
	else
		local range = options.proximity_out - options.proximity_in
		self.proximity_raw = get_point_to_rectangle_proximity(cursor, self)
		self.proximity = 1 - (clamp(0, self.proximity_raw - options.proximity_in, range) / range)
	end
end

function Element:is_persistent()
	local persist = config[self.id .. '_persistency']
	return persist and (
		(persist.audio and state.is_audio)
		or (
			persist.paused and state.pause
			and (not Elements.timeline or not Elements.timeline.pressed or Elements.timeline.pressed.pause)
		)
		or (persist.video and state.is_video)
		or (persist.image and state.is_image)
		or (persist.idle and state.is_idle)
		or (persist.windowed and not state.fullormaxed)
		or (persist.fullscreen and state.fullormaxed)
	)
end

-- 根据接近度及其他各种因素决定元素的可见性
function Element:get_visibility()
	-- 当 curtain 可见时隐藏，除非此元素忽略它
	local min_order = (Elements.curtain.opacity > 0 and not self.ignores_curtain) and Elements.curtain.render_order or 0
	if self.render_order < min_order then return 0 end

	-- 持久性
	if self:is_persistent() then return 1 end

	-- 强制可见性
	if self.forced_visibility then return math.max(self.forced_visibility, self.min_visibility) end

	-- 锚点继承
	-- 若锚点返回 -1，表示所有附着的元素都应强制隐藏。
	local anchor = self.anchor_id and Elements[self.anchor_id]
	local anchor_visibility = anchor and anchor:get_visibility() or 0

	return anchor_visibility == -1 and 0 or math.max(self.proximity, anchor_visibility, self.min_visibility)
end

-- 若方法存在则调用
function Element:maybe(name, ...)
	if self[name] then return self[name](self, ...) end
end

-- 为此元素附加一个补间（tween）动画
---@param from number
---@param to number|fun():number
---@param setter fun(value: number)
---@param duration_or_callback? number|fun() 以毫秒为单位的持续时长，或一个回调函数。
---@param callback? fun() 在动画结束或动画被终止时调用。
function Element:tween(from, to, setter, duration_or_callback, callback)
	self:tween_stop()
	self._kill_tween = self.enabled and tween(
		from, to, setter, duration_or_callback,
		function()
			self._kill_tween = nil
			if callback then callback() end
		end
	)
end

function Element:is_tweening() return self and self._kill_tween end
function Element:tween_stop() self:maybe('_kill_tween') end

-- 在两个值之间对元素的某个 property 做动画。
---@param prop string
---@param from number
---@param to number|fun():number
---@param duration_or_callback? number|fun() 以毫秒为单位的持续时长，或一个回调函数。
---@param callback? fun() 在动画结束或动画被终止时调用。
function Element:tween_property(prop, from, to, duration_or_callback, callback)
	self:tween(from, to, function(value) self[prop] = value end, duration_or_callback, callback)
end

---@param name string
function Element:trigger(name, ...)
	local result = self:maybe('on_' .. name, ...)
	request_render()
	return result
end

-- 让元素短暂闪烁 `options.flash_duration` 毫秒。
-- 适合在通过热键改动音量和 timeline 时将变化可视化。
function Element:flash()
	if self.enabled and options.flash_duration > 0 and (self.proximity < 1 or self._flash_out_timer:is_enabled()) then
		self:tween_stop()
		self.forced_visibility = 1
		request_render()
		self._flash_out_timer.timeout = options.flash_duration / 1000
		self._flash_out_timer:kill()
		self._flash_out_timer:resume()
	end
end

-- 注册一个在元素销毁时调用的析构函数。
---@param disposer fun()
function Element:register_disposer(disposer)
	if not itable_index_of(self._disposers, disposer) then
		self._disposers[#self._disposers + 1] = disposer
	end
end

-- 自动为传入的回调注册析构函数。
---@param event string
---@param callback fun()
function Element:register_mp_event(event, callback)
	mp.register_event(event, callback)
	self:register_disposer(function() mp.unregister_event(callback) end)
end

-- 自动为该观察器注册析构函数。
---@param name string
---@param type_or_callback string|fun(name: string, value: any)
---@param callback_maybe nil|fun(name: string, value: any)
function Element:observe_mp_property(name, type_or_callback, callback_maybe)
	local callback = type(type_or_callback) == 'function' and type_or_callback or callback_maybe
	local prop_type = type(type_or_callback) == 'string' and type_or_callback or 'native'
	mp.observe_property(name, prop_type, callback)
	self:register_disposer(function() mp.unobserve_property(callback) end)
end

-- 添加一个在元素生命周期内（或被手动移除前）有效的按键绑定。
---@param key string mpv 按键标识符。
---@param fnFlags fun()|string|table<fun()|string> 回调，或 `{callback, flags}` 元组。回调可以只是一个方法名，此时它会被包装进 `create_action(callback)`。
---@param namespace? string 按键绑定命名空间。默认为 `_`。
function Element:add_key_binding(key, fnFlags, namespace)
	local name = self.id .. '-' .. key
	local isTuple = type(fnFlags) == 'table'
	local fn = (isTuple and fnFlags[1] or fnFlags)
	local flags = isTuple and fnFlags[2] or nil
	namespace = namespace or '_'
	local names = self._key_bindings[namespace]
	if not names then
		names = {}
		self._key_bindings[namespace] = names
	end
	names[name] = true
	if type(fn) == 'string' then
		fn = self:create_action(fn)
	end
	mp.add_forced_key_binding(key, name, fn, flags)
end

-- 移除全部按键绑定，或仅移除属于某个特定命名空间的按键绑定。
---@param namespace? string 可选，要移除的按键绑定命名空间。
function Element:remove_key_bindings(namespace)
	local namespaces = namespace and {namespace} or table_keys(self._key_bindings)
	for _, namespace in ipairs(namespaces) do
		local names = self._key_bindings[namespace]
		if names then
			for name, _ in pairs(names) do
				mp.remove_key_binding(name)
			end
			self._key_bindings[namespace] = nil
		end
	end
end

-- 检查此元素是否有任何按键绑定（全部，或指定命名空间内的）。
---@param namespace? string 仅检查此命名空间。
function Element:has_keybindings(namespace)
	if namespace then
		return self._key_bindings[namespace] ~= nil
	else
		return #table_keys(self._key_bindings) > 0
	end
end

-- 检查元素是否未被销毁、也未以其他方式被禁用。
-- 设计为可被继承的元素重写以添加更多检查。
function Element:is_alive() return not self.destroyed end

-- 将函数包装成一个回调：当元素已销毁或以其他方式被禁用时不会运行。
---@param fn fun(...)|string 要调用的函数，或本类上某个方法的名称。
function Element:create_action(fn)
	if type(fn) == 'string' then
		local method = fn
		fn = function(...) self[method](self, ...) end
	end
	return function(...)
		if self:is_alive() then fn(...) end
	end
end

return Element
