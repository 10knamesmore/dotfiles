local Element = require('elements/Element')

---@alias MenuAction {name: string; icon: string; label?: string; filter_hidden?: boolean;}

-- `Menu:open(menu)` 接受的菜单数据结构。
---@alias MenuData {id?: string; type?: string; title?: string; hint?: string; footnote: string; search_style?: 'on_demand' | 'palette' | 'disabled';  item_actions?: MenuAction[]; item_actions_place?: 'inside' | 'outside'; callback?: string[]; keep_open?: boolean; bold?: boolean; italic?: boolean; muted?: boolean; separator?: boolean; align?: 'left'|'center'|'right'; items?: MenuDataChild[]; selected_index?: integer; on_search?: string|string[]; on_paste?: string|string[]; on_move?: string|string[]; on_close?: string|string[]; search_debounce?: number|string; search_submenus?: boolean; search_suggestion?: string; search_submit?: boolean; bind_keys?: string[]}
---@alias MenuDataChild MenuDataItem|MenuData
---@alias MenuDataItem {title?: string; hint?: string; icon?: string; value: any; actions?: MenuAction[]; actions_place?: 'inside' | 'outside'; active?: boolean; keep_open?: boolean; selectable?: boolean; bold?: boolean; italic?: boolean; muted?: boolean; separator?: boolean; align?: 'left'|'center'|'right'}
---@alias MenuOptions {mouse_nav?: boolean;}

-- 由 `MenuData` 生成的内部数据结构。
---@alias MenuStack {id?: string; type?: string; title?: string; hint?: string; footnote: string; search_style?: 'on_demand' | 'palette' | 'disabled';  item_actions?: MenuAction[]; item_actions_place?: 'inside' | 'outside'; callback?: string[]; selected_index?: number; action_index?: number; keep_open?: boolean; bold?: boolean; italic?: boolean; muted?: boolean; separator?: boolean; align?: 'left'|'center'|'right'; items: MenuStackChild[]; on_search?: string|string[]; on_paste?: string|string[]; on_move?: string|string[]; on_close?: string|string[]; search_debounce?: number|string; search_submenus?: boolean; search_suggestion?: string; search_submit?: boolean; bind_keys?: string[]; parent_menu?: MenuStack; submenu_path: integer[]; active?: boolean; width: number; height: number; top: number; scroll_y: number; scroll_height: number; title_width: number; hint_width: number; max_width: number; is_root?: boolean; fling?: Fling, search?: Search, ass_safe_title?: string}
---@alias MenuStackChild MenuStackItem|MenuStack
---@alias MenuStackItem {title?: string; hint?: string; icon?: string; value: any; actions?: MenuAction[]; actions_place?: 'inside' | 'outside'; active?: boolean; keep_open?: boolean; selectable?: boolean; bold?: boolean; italic?: boolean; muted?: boolean; separator?: boolean; align?: 'left'|'center'|'right'; title_width: number; hint_width: number; ass_safe_hint?: string}
---@alias Fling {y: number, distance: number, time: number, easing: fun(x: number), duration: number, update_cursor?: boolean}
---@alias Search {query: string; cursor: number; timeout: unknown; min_top: number; max_width: number; source: {width: number; top: number; scroll_y: number; selected_index?: integer; items?: MenuStackChild[]}}

---@alias MenuEventActivate {type: 'activate'; index: number; value: any; action?: string; modifiers?: string; alt: boolean; ctrl: boolean; shift: boolean; is_pointer: boolean; keep_open?: boolean; menu_id: string;}
---@alias MenuEventMove {type: 'move'; from_index: number; to_index: number; menu_id: string;}
---@alias MenuEventSearch {type: 'search'; query: string; menu_id: string;}
---@alias MenuEventKey {type: 'key'; id: string; key: string; modifiers?: string; alt: boolean; ctrl: boolean; shift: boolean; menu_id: string; selected_item?: {index: number; value: any; action?: string;}}
---@alias MenuEventPaste {type: 'paste'; value: string; menu_id: string; selected_item?: {index: number; value: any; action?: string;}}
---@alias MenuEventBack {type: 'back';}
---@alias MenuEventClose {type: 'close';}
---@alias MenuEvent MenuEventActivate | MenuEventMove | MenuEventSearch | MenuEventKey | MenuEventPaste | MenuEventBack | MenuEventClose
---@alias MenuCallback fun(data: MenuEvent)

---@class Menu : Element
local Menu = class(Element)

---@param data MenuData
---@param callback MenuCallback
---@param opts? MenuOptions
function Menu:open(data, callback, opts)
	local open_menu = Menu:is_open()
	if open_menu then
		open_menu.is_being_replaced = true
		open_menu:close(true)
	end
	return Menu:new(data, callback, opts)
end

---@param menu_type? string
---@return Menu|nil
function Menu:is_open(menu_type)
	return Elements.menu and (not menu_type or Elements.menu.type == menu_type) and Elements.menu or nil
end

---@param immediate? boolean 立即关闭，不播放淡出动画。
---@param callback? fun() 在动画（如有）结束、元素被移除并销毁后调用。
---@overload fun(callback: fun())
function Menu:close(immediate, callback)
	if type(immediate) ~= 'boolean' then callback = immediate end

	local menu = self == Menu and Elements.menu or self

	if state.ime_active == false and mp.get_property_bool('input-ime') then
		mp.set_property_bool('input-ime', false)
	end

	if menu and not menu.destroyed then
		if menu.is_closing then
			menu:tween_stop()
			return
		end

		local function close()
			local on_close = menu.root.on_close -- 会在 menu:destroy() 中被移除
			Elements:remove('menu') -- 内部会调用 menu:destroy()
			Elements:update_proximities()
			cursor:queue_autohide()

			-- 调用 :close() 的回调
			if callback then callback() end

			-- 调用菜单配置上定义的回调/事件
			local close_event = {type = 'close'}
			if not on_close or menu:command_or_event(on_close, {}, close_event) ~= 'event' then
				menu.callback(close_event)
			end

			request_render()
		end

		menu.is_closing = true

		if immediate then
			close()
		else
			menu:fadeout(close)
		end
	end
end

---@param data MenuData
---@param callback MenuCallback
---@param opts? MenuOptions
---@return Menu
function Menu:new(data, callback, opts) return Class.new(self, data, callback, opts) --[[@as Menu]] end
---@param data MenuData
---@param callback MenuCallback
---@param opts? MenuOptions
function Menu:init(data, callback, opts)
	Element.init(self, 'menu', {render_order = 1001})

	-----@type fun()
	self.callback = callback
	self.opts = opts or {}
	self.offset_x = 0 -- 用于子菜单切换动画。
	self.mouse_nav = self.opts.mouse_nav -- 关闭项目的预选中
	self.item_height = nil
	self.min_width = nil
	self.item_spacing = 1
	self.item_padding = nil
	self.separator_size = nil
	self.padding = nil
	self.gap = nil
	self.font_size = nil
	self.font_size_hint = nil
	self.scroll_step = nil -- 项目高度 + 项目间距。
	self.scroll_height = nil -- 项目 + 间距 - 容器高度。
	self.opacity = 0 -- 用于淡入/淡出。
	self.type = data.type
	---@type MenuStack 根 MenuStack。
	self.root = nil
	---@type MenuStack 当前 MenuStack。
	self.current = nil
	---@type MenuStack[] 扁平数组形式的所有菜单。
	self.all = nil
	---@type table<string, MenuStack> 以 id 为键的子菜单映射，例如 `'Tools > Aspect ratio'`。
	self.by_id = {}
	self.type_to_search = options.menu_type_to_search
	self.is_being_replaced = false
	self.is_closing = false
	self.drag_last_y = nil
	self.is_dragging = false

	if utils.shared_script_property_set then
		utils.shared_script_property_set('uosc-menu-type', self.type or 'undefined')
	end
	mp.set_property_native('user-data/uosc/menu/type', self.type or 'undefined')
	self:update(data)

	for _, menu in ipairs(self.all) do self:scroll_to_index(menu.selected_index, menu.id) end
	if self.mouse_nav then self.current.selected_index = nil end

	self:tween_property('opacity', 0, 1)
	self:enable_key_bindings()
	Elements:maybe('curtain', 'register', self.id)

	if data.search_submit then
		-- 这里必须延迟执行，以免在我们正在构造的菜单实例返回之前就触发菜单回调，
		-- 因为这些回调可能会依赖该实例。
		mp.add_timeout(0.01, function()
			self:search_submit()
		end)
	end
end

function Menu:destroy()
	Element.destroy(self)
	self.is_closing = false
	if not self.is_being_replaced then Elements:maybe('curtain', 'unregister', self.id) end
	if utils.shared_script_property_set then
		utils.shared_script_property_set('uosc-menu-type', nil)
	end
	mp.set_property_native('user-data/uosc/menu/type', nil)
end

---@param data MenuData
function Menu:update(data)
	local new_root = {is_root = true, submenu_path = {}}
	local new_all = {}
	local new_menus = {} -- 本次 `update()` 之前不存在的菜单
	local new_by_id = {}
	local menus_to_serialize = {{new_root, data}}
	local old_current_id = self.current and self.current.id
	local menu_state_props = {'selected_index', 'action_index', 'scroll_y', 'fling', 'search'}
	local internal_props_set = create_set(itable_append({'is_root', 'submenu_path', 'id', 'items'}, menu_state_props))

	table_assign_exclude(new_root, data, internal_props_set)

	local i = 0
	while i < #menus_to_serialize do
		i = i + 1
		local menu, menu_data = menus_to_serialize[i][1], menus_to_serialize[i][2]
		local parent_id = menu.parent_menu and not menu.parent_menu.is_root and menu.parent_menu.id
		if menu_data.id then
			menu.id = menu_data.id
		elseif not menu.is_root then
			menu.id = (parent_id and parent_id .. ' > ' or '') .. (menu_data.title or i)
		else
			menu.id = '{root}'
		end
		menu.icon = 'chevron_right'

		-- 规范化 `search_debounce`
		if type(menu_data.search_debounce) == 'number' then
			menu.search_debounce = math.max(0, menu_data.search_debounce --[[@as number]])
		elseif menu_data.search_debounce == 'submit' then
			menu.search_debounce = 'submit'
		else
			menu.search_debounce = menu.on_search and 300 or 0
		end

		-- 更新项目
		local first_active_index = nil
		menu.items = {
			{title = t('Empty'), value = 'ignore', italic = 'true', muted = 'true', selectable = false, align = 'center'},
		}

		for i, item_data in ipairs(menu_data.items or {}) do
			if item_data.active and not first_active_index then first_active_index = i end

			local item = {}
			table_assign_exclude(item, item_data, internal_props_set)
			if item.keep_open == nil then item.keep_open = menu.keep_open end

			-- 子菜单
			if item_data.items then
				item.parent_menu = menu
				item.submenu_path = itable_join(menu.submenu_path, {i})
				menus_to_serialize[#menus_to_serialize + 1] = {item, item_data}
			end

			menu.items[i] = item
		end

		if menu.is_root then menu.selected_index = menu_data.selected_index or first_active_index end

		-- 保留旧状态
		local old_menu = self.by_id[menu.id]
		if old_menu then
			table_assign_props(menu, old_menu, menu_state_props)
		else
			new_menus[#new_menus + 1] = menu
		end

		new_all[#new_all + 1] = menu
		new_by_id[menu.id] = menu
	end

	self.root, self.all, self.by_id = new_root, new_all, new_by_id
	self.current = self.by_id[old_current_id] or self.root

	self:update_content_dimensions()
	self:reset_navigation()

	-- 确保 palette 菜单有处于激活状态的搜索，并清理那些丢失了 `palette` 标志的菜单上的空搜索
	local update_dimensions_again = false
	for _, menu in ipairs(self.all) do
		local is_palette = menu.search_style == 'palette'
		if not menu.search and (is_palette or (menu.search_suggestion and itable_index_of(new_menus, menu))) then
			update_dimensions_again = true
			self:search_init(menu.id)
		elseif not is_palette and menu.search and menu.search.query == '' then
			update_dimensions_again = true
			menu.search = nil
		end
	end
	-- 之所以前后都更新一次，是因为 search_init 需要未搜索时菜单的初始位置和滚动状态，
	-- 以便保存到 `search.source` 表中。
	if update_dimensions_again then
		self:update_content_dimensions()
		self:reset_navigation()
	end
	-- 应用搜索建议
	for _, menu in ipairs(new_menus) do
		if menu.search_suggestion then
			menu.search.query = menu.search_suggestion
			menu.search.cursor = #menu.search_suggestion
		end
	end
	for _, menu in ipairs(self.all) do
		if menu.search then
			-- 菜单项目是新对象，搜索需要包含这些新对象
			menu.search.source.items = not menu.on_search and menu.items or nil
			-- 只有内部搜索会立即提交
			if not menu.on_search then self:search_internal(menu.id, true) end
		end

		if menu.selected_index then self:select_by_offset(0, menu) end
	end

	self:search_ensure_key_bindings()
end

---@param items MenuDataChild[]
function Menu:update_items(items)
	local data = table_assign({}, self.root)
	data.items = items
	self:update(data)
end

function Menu:update_content_dimensions()
	self.item_height = round(options.menu_item_height * state.scale)
	self.min_width = round(options.menu_min_width * state.scale)
	self.separator_size = round(1 * state.scale)
	self.scrollbar_size = round(2 * state.scale)
	self.padding = round(options.menu_padding * state.scale)
	self.gap = round(2 * state.scale)
	self.font_size = round(self.item_height * 0.48 * options.font_scale)
	self.font_size_hint = self.font_size - 1
	self.item_padding = round((self.item_height - self.font_size) * 0.6)
	self.scroll_step = self.item_height + self.item_spacing

	local title_opts = {size = self.font_size, italic = false, bold = false}
	local hint_opts = {size = self.font_size_hint}

	for _, menu in ipairs(self.all) do
		title_opts.bold, title_opts.italic = true, false
		local max_width = text_width(menu.title, title_opts) + 2 * self.item_padding

		-- 估算最宽项目的宽度
		for _, item in ipairs(menu.items) do
			local icon_width = item.icon and self.font_size or 0
			item.title_width = text_width(item.title, title_opts)
			item.hint_width = text_width(item.hint, hint_opts)
			local spacings_in_item = 1 + (item.title_width > 0 and 1 or 0)
				+ (item.hint_width > 0 and 1 or 0) + (icon_width > 0 and 1 or 0)
			local estimated_width = item.title_width + item.hint_width + icon_width
				+ (self.item_padding * spacings_in_item)
			if estimated_width > max_width then max_width = estimated_width end
		end

		menu.max_width = max_width
	end

	self:update_dimensions()
end

function Menu:update_dimensions()
	-- 这里的坐标和尺寸指的是可滚动区域。标题渲染在它上方，
	-- 因此 max_height 和 ay 位置都要把标题考虑进去。
	-- 这是历史遗留：当年光标事件处理方式不同，标题也很简陋、没有搜索输入框。
	-- 这部分值得重构。
	local margin = round(self.item_height / 2)
	local external_buttons_reserve = display.width / self.item_height > 14 and self.scroll_step * 6 - margin * 2 or 0
	local width_available = display.width - margin * 2 - self.padding * 2 - external_buttons_reserve
	local height_available = display.height - margin * 2 - self.padding * 2
	local min_width = math.min(self.min_width, width_available)

	for _, menu in ipairs(self.all) do
		local width = math.max(menu.search and menu.search.max_width or 0, menu.max_width)
		menu.width = round(clamp(min_width, width, width_available))
		local title_height = (menu.is_root and menu.title or menu.search) and
			self.scroll_step + self.separator_size + 1 or 0
		local footnote_height = self.font_size * 1.5
		local max_height = height_available - title_height - footnote_height
		local content_height = self.scroll_step * #menu.items
		menu.height = math.min(content_height - self.item_spacing, max_height)
		menu.top = clamp(
			title_height + margin + self.padding,
			menu.search and math.min(menu.search.min_top, menu.search.source.top) or height_available,
			round((height_available - menu.height + title_height) / 2)
		)
		if menu.search then
			menu.search.min_top = math.min(menu.search.min_top, menu.top)
			menu.search.max_width = math.max(menu.search.max_width, menu.width)
		end
		menu.scroll_height = math.max(content_height - menu.height - self.item_spacing, 0)
		self:set_scroll_to(menu.scroll_y, menu.id) -- 把 scroll_y 钳制到滚动范围内
	end

	self:update_coordinates()
end

-- 更新元素坐标，使其匹配当前打开的（子）菜单的 padding box。
function Menu:update_coordinates()
	local ax = round((display.width - self.current.width) / 2 - self.padding) + self.offset_x
	self:set_coordinates(
		ax, self.current.top - self.padding,
		ax + self.current.width + self.padding * 2, self.current.top + self.current.height + self.padding
	)
end

function Menu:reset_navigation()
	local menu = self.current

	-- 重置索引和滚动
	self:set_scroll_to(menu.scroll_y) -- 把 scroll_y 钳制到滚动范围内
	if menu.items and #menu.items > 0 then
		-- 始终规范化已有的 selected_index，但只有在键盘导航时才强制设置它
		if not self.mouse_nav then
			self:select_by_offset(0)
		end
	else
		self:select_index(nil)
	end

	-- 沿父菜单链向上走，激活那些通往当前菜单的项目
	local parent = menu.parent_menu
	while parent do
		parent.selected_index = itable_index_of(parent.items, menu)
		menu, parent = parent, parent.parent_menu
	end

	request_render()
end

function Menu:set_offset_x(offset)
	local delta = offset - self.offset_x
	self.offset_x = offset
	self:set_coordinates(self.ax + delta, self.ay, self.bx + delta, self.by)
end

function Menu:fadeout(callback) self:tween_property('opacity', 1, 0, callback) end

-- 若提供了 `menu_id`，返回该 id 对应的菜单或 `nil`。若 `menu_id` 为 `nil`，返回当前菜单。
---@param menu_id? string
---@return MenuStack | nil
function Menu:get_menu(menu_id) return menu_id == nil and self.current or self.by_id[menu_id] end

function Menu:get_first_active_index(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	for index, item in ipairs(menu.items) do
		if item.active then return index end
	end
end

---@param pos? number
---@param menu_id? string
function Menu:set_scroll_to(pos, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	menu.scroll_y = clamp(0, pos or 0, menu.scroll_height)
	request_render()
end

---@param delta? number
---@param menu_id? string
function Menu:set_scroll_by(delta, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	self:set_scroll_to(menu.scroll_y + delta, menu_id)
end

---@param pos? number
---@param menu_id? string
---@param fling_options? table
function Menu:scroll_to(pos, menu_id, fling_options)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	menu.fling = {
		y = menu.scroll_y,
		distance = clamp(-menu.scroll_y, pos - menu.scroll_y, menu.scroll_height - menu.scroll_y),
		time = mp.get_time(),
		duration = 0.1,
		easing = ease_out_sext,
	}
	if fling_options then table_assign(menu.fling, fling_options) end
	request_render()
end

---@param delta? number
---@param menu_id? string
---@param fling_options? Fling
function Menu:scroll_by(delta, menu_id, fling_options)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	self:scroll_to((menu.fling and (menu.fling.y + menu.fling.distance) or menu.scroll_y) + delta, menu_id, fling_options)
end

---@param index? integer
---@param menu_id? string
---@param immediate? boolean
function Menu:scroll_to_index(index, menu_id, immediate)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	if (index and index >= 1 and index <= #menu.items) then
		local position = round((self.scroll_step * (index - 1)) - ((menu.height - self.scroll_step) / 2))
		if immediate then
			self:set_scroll_to(position, menu_id)
		else
			self:scroll_to(position, menu_id)
		end
	end
end

---@param index? integer
---@param menu_id? string
function Menu:select_index(index, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	menu.selected_index = (index and index >= 1 and index <= #menu.items) and index or nil
	self:select_action(menu.action_index, menu_id) -- 规范化选中的 action 索引
	request_render()
end

---@param index? integer
---@param menu_id? string
function Menu:select_action(index, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local actions = menu.items[menu.selected_index] and menu.items[menu.selected_index].actions or menu.item_actions
	if not index or not actions or type(actions) ~= 'table' or index < 1 or index > #actions then
		menu.action_index = nil
		return
	end
	menu.action_index = index
	request_render()
end

---@param delta? integer
---@param menu_id? string
function Menu:navigate_action(delta, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local actions = menu.items[menu.selected_index] and menu.items[menu.selected_index].actions or menu.item_actions
	if actions and delta ~= 0 then
		-- 循环导航，其中 0 会被转换为 nil
		local index = (menu.action_index or (delta > 0 and 0 or #actions + 1)) + delta
		self:select_action(index <= #actions and index > 0 and (index - 1) % #actions + 1 or nil, menu_id)
	else
		self:select_action(nil, menu_id)
	end
	request_render()
end

function Menu:next_action() self:navigate_action(1) end
function Menu:prev_action() self:navigate_action(-1) end

---@param value? any
---@param menu_id? string
function Menu:select_value(value, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local index = itable_find(menu.items, function(item) return item.value == value end)
	self:select_index(index)
end

---@param menu_id? string
function Menu:deactivate_items(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	for _, item in ipairs(menu.items) do item.active = false end
	request_render()
end

---@param index? integer
---@param menu_id? string
function Menu:activate_index(index, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	if index and index >= 1 and index <= #menu.items then menu.items[index].active = true end
	request_render()
end

---@param value? any
---@param menu_id? string
function Menu:activate_value(value, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local index = itable_find(menu.items, function(item) return item.value == value end)
	self:activate_index(index, menu_id)
end

---@param value? any
---@param menu_id? string
function Menu:activate_one_value(value, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local index = itable_find(menu.items, function(item) return item.value == value end)
	self:activate_index(index, menu_id)
end

---@param id string `self.all` 中的某个菜单。
function Menu:activate_menu(id)
	local menu = self:get_menu(id)
	if menu then
		self.current = menu
		self:update_coordinates()
		self:reset_navigation()
		self:search_ensure_key_bindings()
		local parent = menu.parent_menu
		while parent do
			parent.selected_index = itable_index_of(parent.items, menu)
			self:scroll_to_index(parent.selected_index, parent)
			menu, parent = parent, parent.parent_menu
		end
		request_render()
	end
end

---@param index? integer
---@param menu_id? string
function Menu:delete_index(index, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	if (index and index >= 1 and index <= #menu.items) then
		table.remove(menu.items, index)
		self:update_content_dimensions()
		self:scroll_to_index(menu.selected_index, menu_id)
	end
end

---@param value? any
---@param menu_id? string
function Menu:delete_value(value, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local index = itable_find(menu.items, function(item) return item.value == value end)
	self:delete_index(index)
end

---@param id string 菜单 id。
---@param x number 从该 `x` 坐标滑入。
function Menu:slide_in_menu(id, x)
	local menu = self:get_menu(id)
	if not menu then return end
	self:activate_menu(id)
	self:tween(-(display.width / 2 - menu.width / 2 - x), 0, function(offset) self:set_offset_x(offset) end)
	self.opacity = 1 -- 以防上面的 tween 取消了淡入动画
end

function Menu:back()
	if not self:is_alive() then return end

	local current = self.current
	local parent = current.parent_menu

	if parent then
		self:slide_in_menu(parent.id, display.width / 2 - current.width / 2 - parent.width / 2 + self.offset_x)
	else
		self.callback({type = 'back'})
	end
end

---@param shortcut? Shortcut
---@param is_pointer? boolean 是否由指针触发。
function Menu:activate_selected_item(shortcut, is_pointer)
	local menu = self.current
	local item = menu.items[menu.selected_index]
	if item then
		-- 是子菜单
		if item.items then
			if not self.mouse_nav then
				self:select_index(1, item.id)
			end
			self:activate_menu(item.id)
			self:tween(self.offset_x + menu.width / 2, 0, function(offset) self:set_offset_x(offset) end)
			self.opacity = 1 -- 以防上面的 tween 取消了淡入动画
		else
			local actions = item.actions or menu.item_actions
			local action = actions and actions[menu.action_index]
			self.callback({
				type = 'activate',
				index = menu.selected_index,
				value = item.value,
				is_pointer = is_pointer == true,
				action = action and action.name,
				keep_open = item.keep_open or menu.keep_open,
				modifiers = shortcut and shortcut.modifiers or nil,
				alt = shortcut and shortcut.alt or false,
				ctrl = shortcut and shortcut.ctrl or false,
				shift = shortcut and shortcut.shift or false,
				menu_id = menu.id,
			})
		end
	end
end

---@param index integer
function Menu:move_selected_item_to(index)
	if self.current.search then return end -- 移动被过滤后的项目属于未定义行为
	local callback = self.current.on_move
	local from, items_count = self.current.selected_index, self.current.items and #self.current.items or 0
	if callback and from and from ~= index and index >= 1 and index <= items_count then
		local event = {type = 'move', from_index = from, to_index = index, menu_id = self.current.id}
		self:command_or_event(callback, {from, index, self.current.id}, event)
		self:select_index(index, self.current.id)
		self:scroll_to_index(index, self.current.id, true)
	end
end

---@param delta number
function Menu:move_selected_item_by(delta)
	local current_index, items_count = self.current.selected_index, self.current.items and #self.current.items or 0
	if current_index and items_count > 1 then
		local new_index = clamp(1, current_index + delta, items_count)
		if current_index ~= new_index then
			self:move_selected_item_to(new_index)
		end
	end
end

function Menu:on_display() self:update_dimensions() end
function Menu:on_prop_fullormaxed() self:update_content_dimensions() end
function Menu:on_options() self:update_content_dimensions() end

function Menu:handle_cursor_down()
	if self.proximity_raw <= 0 then
		self.drag_last_y = cursor.y
		self.current.fling = nil
	else
		self:close()
	end
end

---@param shortcut? Shortcut
function Menu:handle_cursor_up(shortcut)
	if self.proximity_raw <= -self.padding and self.drag_last_y and not self.is_dragging then
		self:activate_selected_item(shortcut, true)
	end
	if self.is_dragging then
		local distance = cursor:get_velocity().y / -3
		if math.abs(distance) > 50 then
			self.current.fling = {
				y = self.current.scroll_y,
				distance = distance,
				time = cursor.history:head().time,
				easing = ease_out_quart,
				duration = 0.5,
				update_cursor = true,
			}
			request_render()
		end
	end
	self.is_dragging = false
	self.drag_last_y = nil
end

function Menu:on_global_mouse_move()
	self.mouse_nav = true
	if self.drag_last_y then
		self.is_dragging = self.is_dragging or math.abs(cursor.y - self.drag_last_y) >= 10
		if self.is_dragging then
			local distance = self.drag_last_y - cursor.y
			if distance ~= 0 then self:set_scroll_by(distance) end
			self.drag_last_y = cursor.y
		end
	end
	request_render()
end

function Menu:handle_wheel_up() self:scroll_by(self.scroll_step * -3, nil, {update_cursor = true}) end
function Menu:handle_wheel_down() self:scroll_by(self.scroll_step * 3, nil, {update_cursor = true}) end

---@param offset integer
---@param menu? MenuStack
function Menu:select_by_offset(offset, menu)
	menu = menu or self.current

	-- 当导航越界且可提交搜索处于激活状态时，让 selected_index 失焦。
	-- 失焦的 selected_index 隐含表示输入框获得焦点，因此回车可以提交它。
	if menu.search and menu.search_debounce == 'submit' and (
			(menu.selected_index == 1 and offset < 0) or (menu.selected_index == #menu.items and offset > 0)
		) then
		self:select_index(nil, menu.id)
	else
		local index = clamp(1, (menu.selected_index or offset >= 0 and 0 or #menu.items + 1) + offset, #menu.items)
		local prev_index = itable_find(menu.items, function(item) return item.selectable ~= false end, index, 1)
		local next_index = itable_find(menu.items, function(item) return item.selectable ~= false end, index)
		if prev_index and next_index then
			if offset == 0 then
				self:select_index(index - prev_index <= next_index - index and prev_index or next_index, menu.id)
			elseif offset > 0 then
				self:select_index(next_index, menu.id)
			else
				self:select_index(prev_index, menu.id)
			end
		else
			self:select_index(prev_index or next_index or nil, menu.id)
		end
	end

	request_render()
end

---@param offset integer
---@param immediate? boolean
function Menu:navigate_by_items(offset, immediate)
	self:select_by_offset(offset)
	if self.current.selected_index then
		self:scroll_to_index(self.current.selected_index, self.current.id, immediate)
	end
end

---@param offset integer
---@param immediate? boolean
function Menu:navigate_by_page(offset, immediate)
	local items_per_page = round((self.current.height / self.scroll_step) * 0.4)
	self:navigate_by_items(items_per_page * offset, immediate)
end

function Menu:paste()
	local menu = self.current
	local payload = get_clipboard()
	if not payload then return end
	if menu.search then
		self:search_query_insert(payload)
	elseif menu.on_paste then
		local selected_item = menu.items and menu.selected_index and menu.items[menu.selected_index]
		local actions = selected_item and selected_item.actions or menu.item_actions
		local selected_action = actions and menu.action_index and actions[menu.action_index]
		self:command_or_event(menu.on_paste, {payload, menu.id}, {
			type = 'paste',
			value = payload,
			menu_id = menu.id,
			selected_item = selected_item and {
				index = menu.selected_index, value = selected_item.value, action = selected_action,
			},
		})
	elseif menu.search_style ~= 'disabled' then
		self:search_start(menu.id)
		self:search_query_replace(payload, menu.id)
	end
end

---@param menu_id string
---@param no_select_first? boolean
function Menu:search_internal(menu_id, no_select_first)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	local query = menu.search.query:lower()
	if query == '' then
		-- 把菜单状态重置为搜索前的样子
		for key, value in pairs(menu.search.source) do menu[key] = value end
	else
		-- 从父菜单继承 `search_submenus`
		local search_submenus, parent_menu = menu.search_submenus, menu.parent_menu
		while not search_submenus and parent_menu do
			search_submenus, parent_menu = parent_menu.search_submenus, parent_menu.parent_menu
		end
		menu.items = search_items(menu.search.source.items, query, search_submenus)
		-- 选中搜索结果中的第 1 个项目
		if not no_select_first then
			menu.scroll_y = 0
			self:select_index(1, menu_id)
		end
	end
	self:update_content_dimensions()
end

---@param items MenuStackChild[]
---@param query string
---@param recursive? boolean
---@param prefix? string
---@return MenuStackChild[]
function search_items(items, query, recursive, prefix)
	local result = {}
	local haystacks = {}
	local flat_items = {}
	local concat = table.concat
	local romanization = need_romanization()

	for _, item in ipairs(items) do
		if item.selectable ~= false then
			local prefixed_title = prefix and prefix .. ' / ' .. (item.title or '') or item.title
			haystacks[#haystacks + 1] = item.title
			flat_items[#flat_items + 1] = item

			if item.items and recursive then
				itable_append(result, search_items(item.items, query, recursive, prefixed_title))
			end
		end
	end

	local seen = {}

	local fuzzy = fzy.filter(query, haystacks, false)
	for _, match in ipairs(fuzzy) do
		local idx, positions, score = match[1], match[2], match[3]
		local matched_title = haystacks[idx]
		local item = flat_items[idx]
		local prefixed_title = prefix and prefix .. ' / ' .. (item.title or '') or item.title

		if item.selectable ~= false and not (item.items and recursive) and not seen[item] then
			local bold = item.bold or options.font_bold
			local font_color = item.active and fgt or bgt
			local ass_safe_title = highlight_match(matched_title, positions, font_color, bold) or nil
			local new_item = table_assign({}, item)
			new_item.title = prefixed_title
			new_item.ass_safe_title = prefix and prefix .. ' / ' .. (ass_safe_title or '') or ass_safe_title
			new_item.score = score
			result[#result + 1] = new_item
			seen[item] = true
		end
	end

	for _, item in ipairs(items) do
		local title = item.title and item.title:lower()
		local hint = item.hint and item.hint:lower()
		local bold = item.bold or options.font_bold
		local font_color = item.active and fgt or bgt
		local ass_safe_title = nil
		local prefixed_title = prefix and prefix .. ' / ' .. (item.title or '') or item.title
		if item.selectable ~= false and not (item.items and recursive) and not seen[item] then
			local score = 0
			local match = false

			if title and romanization then
				local ligature_conv_title, ligature_roman = char_conv(title, true)
				local initials_arr_conv, initials_roman = char_conv(title, false)
				local initials_conv_title = concat(initials(initials_arr_conv))
				if ligature_conv_title:find(query, 1, true) then
					match = true
					score = 1000
					local pos = get_roman_match_positions(title, query, 'ligature', ligature_roman)
					if pos then
						ass_safe_title = highlight_match(item.title, pos, font_color, bold)
					end
				elseif initials_conv_title:find(query, 1, true) then
					match = true
					score = 900
					local pos = get_roman_match_positions(title, query, 'initial', initials_roman)
					if pos then
						ass_safe_title = highlight_match(item.title, pos, font_color, bold)
					end
				end
			end

			if hint and not match then
				if hint:find(query, 1, true) then
					match = true
					score = 100
				elseif concat(initials(hint)):find(query, 1, true) then
					match = true
					score = 90
				end
			end

			if match then
				local new_item = table_assign({}, item)
				new_item.title = prefixed_title
				new_item.ass_safe_title = prefix and prefix .. ' / ' .. (ass_safe_title or '') or ass_safe_title
				new_item.score = score
				result[#result + 1] = new_item
			end
		end
	end

	table.sort(result, function(a, b) return a.score > b.score end)

	return result
end

---@param menu_id? string
function Menu:search_submit(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu or not menu.search then return end
	local callback, query = menu.on_search, menu.search.query
	if callback then
		self:command_or_event(callback, {query, menu.id}, {type = 'search', query = query, menu_id = menu.id})
	else
		self:search_internal(menu.id)
	end
end

-- 将搜索查询光标移动指定的量。
---@param amount number `<0` 向左，`>0` 向右。
---@param word_mode? boolean 按词/段移动。会覆盖 amount，但保留其方向。
function Menu:search_cursor_move(amount, word_mode)
	local menu = self:get_menu()
	if not menu or not menu.search then return end
	local query, cursor = menu.search.query, menu.search.cursor
	if word_mode then
		menu.search.cursor = find_string_segment_bound(query, cursor, amount) + (amount < 0 and -1 or 0)
	else
		local move = amount > 0 and utf8_next or utf8_prev
		local step_count = 0
		local limit = math.abs(amount)

		while step_count < limit do
			local next_cursor = move(query, cursor)
			if next_cursor == cursor then break end
			cursor = next_cursor
			step_count = step_count + 1
		end

		menu.search.cursor = clamp(0, cursor, #query)
	end
	request_render()
end

---@param query string
---@param menu_id? string
---@param immediate? boolean
function Menu:search_query_replace(query, menu_id, immediate)
	local menu = self:get_menu(menu_id)
	if not menu or not menu.search then return end
	menu.search.query = query
	menu.search.cursor = #query
	self:search_trigger(menu_id, immediate)
end

-- 在光标处向搜索查询插入字符串。
---@param str string
---@param menu_id? string
function Menu:search_query_insert(str, menu_id)
	local menu = self:get_menu(menu_id)
	if not menu or not menu.search then return end
	local query, cursor = menu.search.query, menu.search.cursor
	local head, tail = string.sub(query, 1, cursor), string.sub(query, cursor + 1)
	menu.search.query = head .. str .. tail
	menu.search.cursor = cursor + #str
	self:search_trigger(menu_id)
end

-- 触发菜单搜索回调，应在查询发生任何变化后调用。
function Menu:search_trigger(menu_id, immediate)
	local menu = self:get_menu(menu_id)
	if not menu or not menu.search then return end
	if menu.search_debounce ~= 'submit' then
		if menu.search.timeout then menu.search.timeout:kill() end
		if menu.search.timeout and not immediate then
			menu.search.timeout:resume()
		else
			self:search_submit(menu_id)
		end
	else
		-- `search_debounce='submit'` 行为：查询变化时让选中项失焦，
		-- 这样 [enter] 键就用于提交搜索而不是激活项目。
		self:select_index(nil, menu.id)
	end
	request_render()
end

---@param event? string
---@param word_mode? boolean 按词删除。
function Menu:search_query_backspace(event, word_mode)
	local search = self.current.search
	if not search then return end

	local cursor, old_query = search.cursor, search.query
	local head, tail = string.sub(old_query, 1, cursor), string.sub(old_query, cursor + 1)

	if word_mode then
		cursor = find_string_segment_bound(head, cursor, -1) - 1
	elseif cursor > 0 then
		-- while 循环用于跳过 utf8 续字节
		while cursor > 1 and old_query:byte(cursor) >= 0x80 and old_query:byte(cursor) <= 0xbf do
			cursor = cursor - 1
		end
		cursor = cursor - 1
	end

	local new_query = head:sub(1, cursor) .. tail
	if new_query ~= old_query then
		search.query = new_query
		search.cursor = math.max(0, cursor)
		self:search_trigger()
	end

	if #new_query == 0 then
		local is_palette = self.current.search_style == 'palette'
		if not is_palette and self.type_to_search then
			self:search_cancel()
		elseif is_palette and event ~= 'repeat' then
			self:back()
		end
	end
end

---@param event? string
---@param word_mode? boolean 按词删除。
function Menu:search_query_delete(event, word_mode)
	local search = self.current.search
	if not search then return end

	local cursor, old_query = search.cursor, search.query
	local head, tail = string.sub(old_query, 1, cursor), string.sub(old_query, cursor + 1)
	local tail_cursor = 1

	if word_mode then
		tail_cursor = find_string_segment_bound(tail, 0, 1) + 1
	else
		-- while 循环用于跳过 utf8 续字节
		while tail_cursor < #tail and tail:byte(tail_cursor) >= 0x80 and tail:byte(tail_cursor) <= 0xbf do
			tail_cursor = tail_cursor + 1
		end
		tail_cursor = tail_cursor + 1
	end

	local new_query = head .. tail:sub(tail_cursor)
	if new_query ~= old_query then
		search.query = new_query
		search.cursor = #head
		self:search_trigger()
	end

	if #new_query == 0 then
		local is_palette = self.current.search_style == 'palette'
		if not is_palette and self.type_to_search then
			self:search_cancel()
		elseif is_palette and event ~= 'repeat' then
			self:back()
		end
	end
end

function Menu:search_text_input(info)
	local menu = self.current
	if not menu.search and menu.search_style == 'disabled' then return end
	if info.event ~= 'up' then
		local key_text = info.key_text
		if not key_text then
			-- 可能是 KP0 到 KP9 或 KP_DEC
			key_text = info.key_name:match('KP_?(.+)')
			if not key_text then return end
			if key_text == 'DEC' then key_text = '.' end
		end
		if not menu.search then self:search_start() end
		self:search_query_insert(key_text)
	end
end

---@param menu_id? string
function Menu:search_cancel(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu or not menu.search or menu.search_style == 'palette' then
		self:search_query_replace('', menu_id)
		return
	end
	if state.ime_active == false then
		mp.set_property_bool('input-ime', false)
	end
	self:search_query_replace('', menu_id, true)
	menu.search = nil
	self:search_ensure_key_bindings()
	self:update_dimensions()
	self:reset_navigation()
end

---@param menu_id? string
function Menu:search_init(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	if menu.search then return end
	if state.ime_active == false then
		mp.set_property_bool('input-ime', true)
	end
	local timeout
	if menu.search_debounce ~= 'submit' and menu.search_debounce > 0 then
		timeout = mp.add_timeout(menu.search_debounce / 1000, self:create_action(function()
			self:search_submit(menu.id)
		end))
		timeout:kill()
	end
	menu.search = {
		query = '',
		cursor = 0,
		timeout = timeout,
		min_top = menu.top,
		max_width = menu.width,
		source = {
			width = menu.width,
			top = menu.top,
			scroll_y = menu.scroll_y,
			selected_index = menu.selected_index,
			items = not menu.on_search and menu.items or nil,
		},
	}
end

---@param menu_id? string
function Menu:search_start(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu or menu.search_style == 'disabled' then return end
	self:search_init(menu_id)
	self:search_ensure_key_bindings()
	self:update_dimensions()
end

---@param menu_id? string
function Menu:search_clear_query(menu_id)
	local menu = self:get_menu(menu_id)
	if not menu then return end
	if not self.current.search_style == 'palette' and self.type_to_search then
		self:search_cancel(menu_id)
	else
		self:search_query_replace('', menu_id)
	end
end

function Menu:search_enable_key_bindings()
	if self:has_keybindings('search') then return end
	local flags = {repeatable = true, complex = true}
	self:add_key_binding('any_unicode', {self:create_key_handler('search_text_input'), flags}, 'search')
	-- KP0 到 KP9 以及 KP_DEC 不包含在 any_unicode 中，
	-- 尽管它们通常会产生字符，但它们没有 info.key_text
	self:add_key_binding('kp_dec', {self:create_key_handler('search_text_input'), flags}, 'search')
	for i = 0, 9 do
		self:add_key_binding('kp' .. i, {self:create_key_handler('search_text_input'), flags}, 'search')
	end
end

function Menu:search_ensure_key_bindings()
	if self.current.search or (self.type_to_search and self.current.search_style ~= 'disabled') then
		self:search_enable_key_bindings()
	else
		self:remove_key_bindings('search')
	end
end

function Menu:enable_key_bindings()
	local standalone_keys = {'/', 'kp_divide', 'mbtn_back', 'ctrl+f', 'ctrl+v', 'ctrl+c'}
	if type(self.root.bind_keys) == 'table' then itable_append(standalone_keys, self.root.bind_keys) end
	-- 末尾的 `+` 表示启用 `repeatable` 标志
	local modifiable_keys = {'up+', 'down+', 'left', 'right', 'enter', 'kp_enter', 'bs', 'tab', 'esc', 'pgup+',
		'pgdwn+', 'home', 'end', 'del'}
	local modifiers = {nil, 'alt', 'alt+ctrl', 'alt+shift', 'alt+ctrl+shift', 'ctrl', 'ctrl+shift', 'shift'}

	---@param shortcut Shortcut
	---@param flags table<string, boolean>
	local function bind(shortcut, flags)
		local handler = self:create_action(function(info) self:handle_shortcut(shortcut, info) end)
		self:add_key_binding(shortcut.id, {handler, flags})
	end

	for i, key in ipairs(standalone_keys) do
		bind(create_shortcut(key), {repeatable = false, complex = true})
	end

	for i, key in ipairs(modifiable_keys) do
		local flags = {repeatable = false, complex = true}

		if key:sub(-1) == '+' then
			key = key:sub(1, -2)
			flags.repeatable = true
		end

		for j = 1, #modifiers do
			bind(create_shortcut(key, modifiers[j]), flags)
		end
	end

	self:search_ensure_key_bindings()
end

-- 处理所有键盘和鼠标按键快捷方式，unicode 输入除外。
---@param shortcut Shortcut
---@param info ComplexBindingInfo
function Menu:handle_shortcut(shortcut, info)
	if not self:is_alive() then return end

	self.mouse_nav = info.is_mouse
	local menu, id, key, modifiers = self.current, shortcut.id, shortcut.key, shortcut.modifiers
	local selected_index = menu.selected_index
	local selected_item = menu and selected_index and menu.items[selected_index]
	local is_submenu = selected_item and selected_item.items ~= nil
	local actions = selected_item and selected_item.actions or menu.item_actions
	local selected_action = actions and menu.action_index and actions[menu.action_index]

	if info.event == 'up' then return end

	function trigger_shortcut(shortcut)
		self.callback(table_assign({}, shortcut, {
			type = 'key',
			menu_id = menu.id,
			selected_item = selected_item and {
				index = selected_index, value = selected_item.value, action = selected_action,
			},
		}))
	end

	if (key == 'enter' and selected_item) or (id == 'right' and is_submenu and not menu.search) then
		self:activate_selected_item(shortcut)
	elseif id == 'enter' and menu.search and menu.search_debounce == 'submit' then
		self:search_submit()
	elseif id == 'up' or id == 'down' then
		self:navigate_by_items(id == 'up' and -1 or 1, true)
	elseif id == 'pgup' or id == 'pgdwn' then
		self:navigate_by_page(id == 'pgup' and -1 or 1)
	elseif menu.search and (id == 'left' or id == 'ctrl+left') then
		self:search_cursor_move(-1, modifiers == 'ctrl')
	elseif menu.search and (id == 'right' or id == 'ctrl+right') then
		self:search_cursor_move(1, modifiers == 'ctrl')
	elseif menu.search and id == 'home' then
		self:search_cursor_move(-math.huge)
	elseif menu.search and id == 'end' then
		self:search_cursor_move(math.huge)
	elseif id == 'home' or id == 'end' then
		self:navigate_by_items(id == 'home' and -math.huge or math.huge)
	elseif id == 'shift+tab' then
		self:prev_action()
	elseif id == 'tab' then
		self:next_action()
	elseif id == 'ctrl+up' then
		self:move_selected_item_by(-1)
	elseif id == 'ctrl+down' then
		self:move_selected_item_by(1)
	elseif id == 'ctrl+pgup' then
		self:move_selected_item_by(-round((menu.height / self.scroll_step) * 0.4))
	elseif id == 'ctrl+pgdwn' then
		self:move_selected_item_by(round((menu.height / self.scroll_step) * 0.4))
	elseif id == 'ctrl+home' then
		self:move_selected_item_by(-math.huge)
	elseif id == 'ctrl+end' then
		self:move_selected_item_by(math.huge)
	elseif id == '/' or id == 'kp_divide' or id == 'ctrl+f' then
		self:search_start()
	elseif key == 'esc' then
		if menu.search and menu.search_style ~= 'palette' then
			self:search_cancel()
		else
			self:close()
		end
	elseif id == 'left' and menu.parent_menu then
		self:back()
	elseif key == 'bs' then
		if menu.search then
			if modifiers == 'shift' then
				self:search_clear_query()
			elseif not modifiers or modifiers == 'ctrl' then
				self:search_query_backspace(info.event, modifiers == 'ctrl')
			end
		elseif not modifiers and info.event ~= 'repeat' then
			self:back()
		end
	elseif menu.search and (id == 'del' or id == 'ctrl+del' or id == 'shift+del') then
		if id == 'shift+del' then
			-- 搜索期间 `del` 会编辑字符串。我们把 `shift+del` 转换为
			-- `del`，以便有办法触发绑定到 `del` 的菜单回调。
			trigger_shortcut(create_shortcut('del'))
		else
			self:search_query_delete(info.event, modifiers == 'ctrl')
		end
	elseif key == 'mbtn_back' then
		self:back()
	elseif id == 'ctrl+v' then
		self:paste()
	else
		trigger_shortcut(shortcut)
	end
end

-- 检查菜单是否既未关闭也不在关闭中。
function Menu:is_alive() return not self.is_closing and not self.destroyed end

---@param name string
function Menu:create_key_handler(name)
	return self:create_action(function(...)
		self.mouse_nav = false
		self:maybe(name, ...)
	end)
end

-- 携带参数发送命令；若 `command == 'callback'`，则触发一个回调事件。
-- 用于处理 `on_{event}: 'callback' | string | string[]` 这类事件。
-- 返回实际发生的操作类型。
---@param command string|number|string[]|number[]
---@param params string[]|number[]
---@param event MenuEvent
---@return 'event' | 'command' | nil
function Menu:command_or_event(command, params, event)
	if command == 'callback' then
		self.callback(event)
		return 'event'
	elseif type(command) == 'table' then
		---@diagnostic disable-next-line: deprecated
		mp.command_native(itable_join(command, params))
		return 'command'
	elseif type(command) == 'string' then
		mp.command(command .. ' ' .. table.concat(params, ' '))
		return 'command'
	end
	return nil
end

function Menu:render()
	for _, menu in ipairs(self.all) do
		if menu.fling then
			local time_delta = state.render_last_time - menu.fling.time
			local progress = menu.fling.easing(math.min(time_delta / menu.fling.duration, 1))
			self:set_scroll_to(round(menu.fling.y + menu.fling.distance * progress), menu.id)
			if progress < 1 then request_render() else menu.fling = nil end
		end
	end

	cursor:zone('primary_down', display, self:create_action(function() self:handle_cursor_down() end))
	cursor:zone('primary_up', display, self:create_action(function(shortcut) self:handle_cursor_up(shortcut) end))
	cursor:zone('wheel_down', self, function() self:handle_wheel_down() end)
	cursor:zone('wheel_up', self, function() self:handle_wheel_up() end)

	local ass = assdraw.ass_new()
	local icon_size = self.font_size

	---@param menu MenuStack
	---@param x number
	---@param pos number 水平位置索引。0 = 当前菜单，<0 父菜单，>1 子菜单。
	local function draw_menu(menu, x, pos)
		local is_current, is_parent, is_submenu = pos == 0, pos < 0, pos > 0
		local menu_opacity = (pos == 0 and 1 or config.opacity.submenu ^ math.abs(pos)) * self.opacity
		-- 可滚动内容区域的坐标
		local content_rect = {
			ax = x + self.padding,
			ay = menu.top,
			bx = x + self.padding + menu.width,
			by = menu.top + menu.height,
		}
		-- local ax, ay, bx, by = x + self.padding, menu.top, x + menu.width + self.padding, menu.top + menu.height
		local draw_title = menu.is_root and menu.title or menu.search
		local scroll_clip = '\\clip(0,' .. content_rect.ay .. ',' .. display.width .. ',' .. content_rect.by .. ')'
		local start_index = math.floor(menu.scroll_y / self.scroll_step) + 1
		local end_index = math.ceil((menu.scroll_y + menu.height) / self.scroll_step)
		local bg_rect = {
			ax = x,
			ay = content_rect.ay - (draw_title and self.scroll_step or 0) - self.padding,
			bx = content_rect.bx + self.padding,
			by = content_rect.by + self.padding,
		}
		local blur_action_index = self.mouse_nav and menu.action_index ~= nil

		-- 背景
		ass:rect(bg_rect.ax, bg_rect.ay, bg_rect.bx, bg_rect.by, {
			color = bg,
			opacity = menu_opacity * config.opacity.menu,
			radius = state.radius > 0 and math.min(state.radius + self.padding, state.radius * 3) or 0,
		})

		if is_parent then
			cursor:zone('primary_down', bg_rect, self:create_action(function() self:slide_in_menu(menu.id, x) end))
		end

		-- 滚动条
		if menu.scroll_height > 0 then
			local groove_height = menu.height - 2
			local thumb_height = math.max((menu.height / (menu.scroll_height + menu.height)) * groove_height, 40)
			local thumb_y = content_rect.ay + 1 + ((menu.scroll_y / menu.scroll_height) * (groove_height - thumb_height))
			local sax = content_rect.bx - round(self.scrollbar_size / 2)
			local sbx = sax + self.scrollbar_size
			ass:rect(sax, thumb_y, sbx, thumb_y + thumb_height, {color = fg, opacity = menu_opacity * 0.8})
		end

		-- 若已选中则绘制子菜单
		local submenu_rect, current_item = nil, is_current and menu.selected_index and menu.items[menu.selected_index]
		local submenu_is_hovered = false
		if current_item and current_item.items then
			submenu_rect = draw_menu(current_item --[[@as MenuStack]], bg_rect.bx + self.gap, 1)
			cursor:zone('primary_down', submenu_rect, self:create_action(function(shortcut)
				self:activate_selected_item(shortcut, true)
			end))
		end

		---@type MenuAction|nil
		local selected_action
		for index = start_index, end_index, 1 do
			local item = menu.items[index]

			if not item then break end

			local item_ay = content_rect.ay - menu.scroll_y + self.scroll_step * (index - 1)
			local item_by = item_ay + self.item_height
			local item_center_y = item_ay + (self.item_height / 2)
			local item_clip = (item_ay < content_rect.ay or item_by > content_rect.by) and scroll_clip or nil
			local content_ax, content_bx = content_rect.ax + self.item_padding,
				content_rect.bx - self.item_padding
			local is_selected = menu.selected_index == index
			local item_rect_hitbox = {
				ax = content_rect.ax,
				ay = math.max(item_ay, bg_rect.ay),
				bx = bg_rect.bx + (item.items and self.gap or -self.padding), -- 用于桥接光标与子菜单之间的间隙
				by = math.min(item_ay + self.scroll_step, bg_rect.by),
			}

			-- 选中悬停的项目
			if is_current and self.mouse_nav and item.selectable ~= false
				-- 如果光标正朝着子菜单移动，则不选中项目
				and (not submenu_rect or not cursor:direction_to_rectangle_distance(submenu_rect))
				and (submenu_is_hovered or get_point_to_rectangle_proximity(cursor, item_rect_hitbox) <= 0) then
				menu.selected_index = index
				if not is_selected then
					is_selected = true
					request_render()
				end
			end

			local has_background = is_selected or item.active
			local next_item = menu.items[index + 1]
			local next_is_active = next_item and next_item.active
			local next_has_background = menu.selected_index == index + 1 or next_is_active
			local font_color = item.active and fgt or bgt
			local actions = is_selected and (item.actions or menu.item_actions) -- 非 nil = actions 可见
			local action = actions and actions[menu.action_index] -- 非 nil = 已选中某个 action

			if action then selected_action = action end

			-- 分隔线
			if item_by < content_rect.by and ((not has_background and not next_has_background) or item.separator) then
				local ay, by = item_by, item_by + self.separator_size
				if has_background then
					ay, by = ay + self.separator_size, by + self.separator_size
				elseif next_has_background then
					ay, by = ay - self.separator_size, by - self.separator_size
				end
				ass:rect(
					content_rect.ax + self.item_padding, ay, content_rect.bx - self.item_padding, by,
					{color = fg, opacity = menu_opacity * (item.separator and 0.13 or 0.04)}
				)
			end

			-- 背景
			local highlight_opacity = 0 + (item.active and 0.8 or 0) + (is_selected and 0.15 or 0)
			if highlight_opacity > 0 then
				ass:rect(content_rect.ax, item_ay, content_rect.bx, item_by, {
					radius = state.radius,
					color = fg,
					opacity = highlight_opacity * menu_opacity,
					clip = item_clip,
				})
			end

			local title_clip_bx = content_bx

			-- Actions（动作按钮）
			local actions_rect
			if is_selected and actions and #actions > 0 and not item.items then
				local place = item.actions_place or menu.item_actions_place
				local margin = self.gap * 2
				local size = item_by - item_ay - margin * 2
				local rect_width = size * #actions + margin * (#actions - 1)

				-- 当请求且有足够空间时，把 actions 放到菜单外侧
				actions_rect = {
					ay = item_ay + margin,
					by = item_by - margin,
					is_outside = place == 'outside' and display.width - bg_rect.bx + margin * 2 > rect_width,
				}
				actions_rect.bx = actions_rect.is_outside and bg_rect.bx + margin + rect_width or
					content_rect.bx - margin
				actions_rect.ax = actions_rect.bx

				for i = 1, #actions, 1 do
					local action_index = #actions - (i - 1)
					local action = actions[action_index]

					-- 当项目是搜索/过滤的结果且该 action 不应显示时，将其隐藏
					if not (action.filter_hidden and menu.search) then
						local is_active = action_index == menu.action_index
						local bx = actions_rect.ax - (i == 1 and 0 or margin)
						local rect = {
							ay = actions_rect.ay,
							by = actions_rect.by,
							ax = bx - size,
							bx = bx,
						}
						actions_rect.ax = rect.ax

						ass:rect(rect.ax, rect.ay, rect.bx, rect.by, {
							radius = state.radius > 2 and state.radius - 1 or state.radius,
							color = is_active and fg or bg,
							border = is_active and self.gap or nil,
							border_color = bg,
							opacity = menu_opacity,
							clip = item_clip,
						})
						ass:icon(rect.ax + size / 2, rect.ay + size / 2, size * 0.66, action.icon, {
							color = is_active and bg or fg, opacity = menu_opacity, clip = item_clip,
						})

						-- 复用 rect 作为 hitbox：放大它以桥接间隙，防止闪烁
						rect.ay, rect.by, rect.bx = item_ay, item_ay + self.scroll_step, rect.bx + margin

						-- 光标悬停时选中该 action
						if self.mouse_nav and get_point_to_rectangle_proximity(cursor, rect) <= 0 then
							cursor:zone('primary_down', rect, self:create_action(function(shortcut)
								self:activate_selected_item(shortcut, true)
							end))
							blur_action_index = false
							if not is_active then
								menu.action_index = action_index
								selected_action = actions[action_index]
								request_render()
							end
						end
					end
				end

				title_clip_bx = actions_rect.ax - self.gap * 2
			end

			-- 选中项目的指示线
			if is_selected and not selected_action then
				local size = round(2 * state.scale)
				local v_padding = math.min(state.radius, math.ceil(self.item_height / 3))
				ass:rect(
					content_rect.ax - size - 1, item_ay + v_padding,
					content_rect.ax - 1, item_by - v_padding,
					{radius = 1 * state.scale, color = fg, opacity = menu_opacity, clip = item_clip}
				)
			end

			-- 图标
			if item.icon then
				if not actions_rect or actions_rect.is_outside then
					local x = (not item.title and not item.hint and item.align == 'center')
						and bg_rect.ax + (bg_rect.bx - bg_rect.ax) / 2
						or content_bx - (icon_size / 2)
					if item.icon == 'spinner' then
						ass:spinner(x, item_center_y, icon_size * 1.5, {color = font_color, opacity = menu_opacity * 0.8})
					else
						ass:icon(x, item_center_y, icon_size * 1.5, item.icon, {
							color = font_color, opacity = menu_opacity, clip = item_clip,
						})
					end
				end
				content_bx = content_bx - icon_size - self.item_padding
				title_clip_bx = math.min(content_bx, title_clip_bx)
			end

			local hint_clip_bx = title_clip_bx
			if item.hint_width > 0 then
				-- 按 title 与 hint 的宽度比例控制二者的裁剪
				-- title 和 hint 各自至少分得 50% 的宽度，除非它们本身就比这更窄
				local width = content_bx - content_ax - self.item_padding
				local title_min = math.min(item.title_width, width * 0.5)
				local hint_min = math.min(item.hint_width, width * 0.5)
				local title_ratio = item.title_width / (item.title_width + item.hint_width)
				title_clip_bx = math.min(
					title_clip_bx,
					round(content_ax + clamp(title_min, width * title_ratio, width - hint_min))
				)
			end

			-- Hint（提示）
			if item.hint then
				item.ass_safe_hint = item.ass_safe_hint or ass_escape(item.hint)
				local clip = '\\clip(' .. title_clip_bx + self.item_padding .. ','
					.. math.max(item_ay, content_rect.ay) .. ',' .. hint_clip_bx .. ','
					.. math.min(item_by, content_rect.by) .. ')'
				ass:txt(content_bx, item_center_y, 6, item.ass_safe_hint, {
					size = self.font_size_hint,
					color = font_color,
					wrap = 2,
					opacity = 0.5 * menu_opacity,
					clip = clip,
				})
			end

			-- 标题
			if item.title then
				item.ass_safe_title = item.ass_safe_title or ass_escape(item.title)
				local clip = '\\clip(' .. content_rect.ax .. ',' .. math.max(item_ay, content_rect.ay) .. ','
					.. title_clip_bx .. ',' .. math.min(item_by, content_rect.by) .. ')'
				local title_x, align = content_ax, 4
				if item.align == 'right' then
					title_x, align = title_clip_bx, 6
				elseif item.align == 'center' then
					title_x, align = content_ax + (title_clip_bx - content_ax) / 2, 5
				end
				ass:txt(title_x, item_center_y, align, item.ass_safe_title, {
					size = self.font_size,
					color = font_color,
					italic = item.italic,
					bold = item.bold,
					wrap = 2,
					opacity = menu_opacity * (item.muted and 0.5 or 1),
					clip = clip,
				})
			end
		end

		-- Footnote / 选中 action 的标签
		if is_current and (menu.footnote or selected_action) then
			local height_half = self.font_size
			local icon_x, icon_y = content_rect.ax + self.font_size / 2, bg_rect.by + height_half
			local is_icon_hovered = false
			local icon_hitbox = {
				ax = icon_x - height_half,
				ay = icon_y - height_half,
				bx = icon_x + height_half,
				by = icon_y + height_half,
			}
			is_icon_hovered = get_point_to_rectangle_proximity(cursor, icon_hitbox) <= 0
			local text = selected_action and selected_action.label or is_icon_hovered and menu.footnote
			local opacity = (is_icon_hovered and 1 or 0.5) * menu_opacity
			ass:icon(icon_x, icon_y, self.font_size, is_icon_hovered and 'help' or 'help_outline', {
				color = fg, border = state.scale, border_color = bg, opacity = opacity,
			})
			if text then
				ass:txt(icon_x + self.font_size * 0.75, icon_y, 4, text, {
					size = self.font_size,
					color = fg,
					border = state.scale,
					border_color = bg,
					opacity = menu_opacity,
					italic = true,
				})
			end
		end

		-- 菜单标题
		if draw_title then
			local requires_submit = menu.search_debounce == 'submit'
			local rect = {
				ax = content_rect.ax,
				ay = content_rect.ay - self.scroll_step - self.separator_size - 1,
				bx = content_rect.bx,
				by = content_rect.ay - self.separator_size - 1,
			}
			-- 中心点
			rect.cx, rect.cy = round(rect.ax + (rect.bx - rect.ax) / 2), round(rect.ay + (rect.by - rect.ay) / 2)

			if menu.title and not menu.ass_safe_title then
				menu.ass_safe_title = ass_escape(menu.title)
			end

			-- 分隔线
			ass:rect(
				rect.ax, rect.by, rect.bx, rect.by + self.separator_size, {color = fg, opacity = menu_opacity * 0.2}
			)

			-- 用户点击标题时让选中失焦（同时也会激活搜索输入框）
			if is_current then
				cursor:zone('primary_down', rect, function()
					self:select_index(nil)
				end)
			end

			-- 标题
			if menu.search then
				-- 图标
				local icon_size, icon_opacity = self.font_size * 1.3, menu_opacity * (requires_submit and 0.5 or 1)
				local icon_rect = {
					ax = rect.ax,
					ay = rect.ay,
					bx = content_rect.ax + icon_size + self.item_padding * 1.5,
					by = rect.by,
				}

				if is_current and requires_submit then
					cursor:zone('primary_down', icon_rect, function() self:search_submit() end)
					if get_point_to_rectangle_proximity(cursor, icon_rect) <= 0 then
						icon_opacity = menu_opacity
					end
				end

				ass:icon(rect.ax + icon_size / 2, rect.cy, icon_size, 'search', {
					color = fg,
					opacity = icon_opacity,
					clip = '\\clip(' ..
						icon_rect.ax .. ',' .. icon_rect.ay .. ',' .. icon_rect.bx .. ',' .. icon_rect.by .. ')',
				})

				-- 查询内容/占位符
				local cursor_height_half, cursor_thickness = round(self.font_size * 0.6), round(self.font_size / 12)
				local cursor_ax = rect.bx + 1
				if menu.search.query ~= '' then
					local opts = {
						size = self.font_size,
						color = bgt,
						wrap = 2,
						opacity = menu_opacity,
						clip = '\\clip(' .. icon_rect.bx .. ',' .. rect.ay .. ',' .. rect.bx .. ',' .. rect.by .. ')',
					}
					local query, cursor = menu.search.query, menu.search.cursor
					-- 追加一个 ZWNBSP 后缀，防止 libass 裁掉末尾空格
					local head = ass_escape(string.sub(query, 1, cursor)) .. '\239\187\191'
					local tail_no_escape = string.sub(query, cursor + 1)
					local tail = ass_escape(tail_no_escape) .. '\239\187\191'
					cursor_ax = math.max(round(cursor_ax - text_width(tail_no_escape, opts)), rect.cx)
					ass:txt(cursor_ax, rect.cy, 6, head, opts)
					ass:txt(cursor_ax, rect.cy, 4, tail, opts)
				else
					local placeholder = (menu.search_style == 'palette' and menu.ass_safe_title)
						and menu.ass_safe_title
						or (requires_submit and t('type & ctrl+enter to search') or t('type to search'))
					ass:txt(rect.bx, rect.cy, 6, placeholder, {
						size = self.font_size,
						italic = true,
						color = bgt,
						wrap = 2,
						opacity = menu_opacity * 0.4,
						clip = '\\clip(' .. rect.ax .. ',' .. rect.ay .. ',' .. rect.bx .. ',' .. rect.by .. ')',
					})
				end

				-- 可提交搜索的输入框选中指示。
				-- （当 `selected_index` 为 `nil` 时表示输入框被选中）
				if menu.search_debounce == 'submit' and not menu.selected_index then
					local size_half = round(1 * state.scale)
					ass:rect(
						content_rect.ax, rect.by - size_half, content_rect.bx, rect.by + size_half,
						{color = fg, opacity = menu_opacity}
					)
				end
				local input_is_blurred = menu.search_debounce == 'submit' and menu.selected_index

				-- 光标
				local cursor_bx = cursor_ax + cursor_thickness
				ass:rect(cursor_ax, rect.cy - cursor_height_half, cursor_bx, rect.cy + cursor_height_half, {
					color = fg,
					opacity = menu_opacity * (input_is_blurred and 0.5 or 1),
					clip = '\\clip(' .. cursor_ax .. ',' .. rect.ay .. ',' .. cursor_bx .. ',' .. rect.by .. ')',
				})
			else
				ass:txt(rect.cx, rect.cy, 5, menu.ass_safe_title, {
					size = self.font_size,
					bold = true,
					color = bgt,
					wrap = 2,
					opacity = menu_opacity,
					clip = '\\clip(' .. rect.ax .. ',' .. rect.ay .. ',' .. rect.bx .. ',' .. rect.by .. ')',
				})
			end
		end

		if blur_action_index then
			menu.action_index = nil
			request_render()
		end

		return bg_rect
	end

	-- 当前激活的菜单
	draw_menu(self.current, self.ax, 0)

	-- 父菜单
	local parent_menu = self.current.parent_menu
	local parent_offset_x, parent_horizontal_index = self.ax, -1

	while parent_menu do
		parent_offset_x = parent_offset_x - parent_menu.width - self.padding * 2 - self.gap
		draw_menu(parent_menu, parent_offset_x, parent_horizontal_index)
		parent_horizontal_index = parent_horizontal_index - 1
		parent_menu = parent_menu.parent_menu
	end

	return ass
end

return Menu
