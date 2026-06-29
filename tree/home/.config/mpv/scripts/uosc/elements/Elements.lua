local Elements = {_all = {}}

---@param element Element
function Elements:add(element)
	if not element.id then
		msg.error('attempt to add element without "id" property')
		return
	end

	if self:has(element.id) then Elements:remove(element.id) end

	self._all[#self._all + 1] = element
	self[element.id] = element

	-- 按渲染顺序排序
	table.sort(self._all, function(a, b) return a.render_order < b.render_order end)

	request_render()
end

function Elements:remove(idOrElement)
	if not idOrElement then return end
	local id = type(idOrElement) == 'table' and idOrElement.id or idOrElement
	local element = Elements[id]
	if element then
		if not element.destroyed then element:destroy() end
		element.enabled = false
		self._all = itable_delete_value(self._all, self[id])
		self[id] = nil
		request_render()
	end
end

function Elements:update_proximities()
	local curtain_render_order = Elements.curtain.opacity > 0 and Elements.curtain.render_order or 0
	local mouse_leave_elements = {}
	local mouse_enter_elements = {}

	-- 计算所有元素的接近度（proximity）
	for _, element in self:ipairs() do
		if element.enabled then
			local previous_proximity_raw = element.proximity_raw

			-- 若 curtain 已展开，则禁用所有渲染层级在它之下的元素
			if not element.ignores_curtain and element.render_order < curtain_render_order then
				element:reset_proximity()
			else
				element:update_proximity()
			end

			if element.proximity_raw <= 0 then
				-- 鼠标进入元素区域
				if previous_proximity_raw > 0 then
					mouse_enter_elements[#mouse_enter_elements + 1] = element
				end
			else
				-- 鼠标离开元素区域
				if previous_proximity_raw <= 0 then
					mouse_leave_elements[#mouse_leave_elements + 1] = element
				end
			end
		end
	end

	-- 触发 `mouse_leave` 与 `mouse_enter` 事件
	for _, element in ipairs(mouse_leave_elements) do element:trigger('mouse_leave') end
	for _, element in ipairs(mouse_enter_elements) do element:trigger('mouse_enter') end
end

-- 在 0 与 1 之间切换所传入元素的最小可见度（min visibility）。
---@param ids string[] 要窥视（peek）的元素 ID 列表。
function Elements:toggle(ids)
	local has_invisible = itable_find(ids, function(id)
		return Elements[id] and Elements[id].enabled and (Elements[id].min_visibility or 0) ~= 1
	end)

	self:set_min_visibility(has_invisible and 1 or 0, ids)

	-- 切换关闭时重置接近度。必须在 `set_min_visibility` 之后执行，
	-- 因为后者会把接近度作为补间（tween）的起点。
	if not has_invisible then
		for _, id in ipairs(ids) do
			if Elements[id] then Elements[id]:reset_proximity() end
		end
	end
end

-- 将元素的最小可见度设置（以动画过渡）为所传入的值。
---@param visibility number 0-1 之间的浮点数。
---@param ids string[] 要窥视（peek）的元素 ID 列表。
function Elements:set_min_visibility(visibility, ids)
	for _, id in ipairs(ids) do
		local element = Elements[id]
		if element then
			local from = math.max(0, element:get_visibility())
			element:tween_property('min_visibility', from, visibility)
		end
	end
end

-- 闪烁（flash）所传入的元素。
---@param ids string[] 要窥视（peek）的元素 ID 列表。
function Elements:flash(ids)
	local elements = itable_filter(self._all, function(element) return itable_has(ids, element.id) end)
	for _, element in ipairs(elements) do element:flash() end

	-- 'progress' 是特例：它是 timeline 的一种状态，而非独立元素
	if itable_has(ids, 'progress') and not itable_has(ids, 'timeline') then
		Elements:maybe('timeline', 'flash_progress')
	end
end

---@param name string 事件名称。
function Elements:trigger(name, ...)
	for _, element in self:ipairs() do element:trigger(name, ...) end
end

-- 根据元素与光标的接近度，触发两个事件：`name` 和 `global_name`。
-- 已禁用的元素不会收到这些事件。
---@param name string 事件名称。
function Elements:proximity_trigger(name, ...)
	for i = #self._all, 1, -1 do
		local element = self._all[i]
		if element.enabled then
			if element.proximity_raw <= 0 then
				if element:trigger(name, ...) == 'stop_propagation' then break end
			end
			if element:trigger('global_' .. name, ...) == 'stop_propagation' then break end
		end
	end
end

-- 若存在 ID 为 `id` 的元素，则返回它的某个 property，可选地带一个 fallback 默认值。
---@param id string
---@param prop string
---@param fallback any
function Elements:v(id, prop, fallback)
	if self[id] and self[id].enabled and self[id][prop] ~= nil then return self[id][prop] end
	return fallback
end

-- 若存在 ID 为 `id` 的元素，则在它上面调用某个方法。
---@param id string
---@param method string
function Elements:maybe(id, method, ...)
	if self[id] then return self[id]:maybe(method, ...) end
end

function Elements:has(id) return self[id] ~= nil end
function Elements:ipairs() return ipairs(self._all) end

return Elements
