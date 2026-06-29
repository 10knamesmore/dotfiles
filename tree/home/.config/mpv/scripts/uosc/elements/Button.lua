local Element = require('elements/Element')

---@alias ButtonProps {icon: string; on_click?: function; is_clickable?: boolean; anchor_id?: string; active?: boolean; badge?: string|number; foreground?: string; background?: string; tooltip?: string}

---@class Button : Element
local Button = class(Element)

---@param id string
---@param props ButtonProps
function Button:new(id, props) return Class.new(self, id, props) --[[@as Button]] end
---@param id string
---@param props ButtonProps
function Button:init(id, props)
	self.icon = props.icon
	self.active = props.active
	self.tooltip = props.tooltip
	self.badge = props.badge
	self.foreground = props.foreground or fg
	self.background = props.background or bg
	self.is_clickable = true
	---@type fun()|nil
	self.on_click = props.on_click
	Element.init(self, id, props)
end

function Button:on_coordinates() self.font_size = round((self.by - self.ay) * 0.7) end
function Button:handle_cursor_click()
	if not self.on_click or not self.is_clickable then return end
	-- 把回调延迟到下一个 tick，否则在事件分发过程中
	-- 会有竞态风险。
	-- 例如：处理器可能往元素栈末尾添加一个菜单，而该菜单又会接住
	-- 我们当前正处理的这次点击事件，于是瞬间把自己关掉。
	mp.add_timeout(0.01, self.on_click)
end

function Button:render()
	local visibility = self:get_visibility()
	if visibility <= 0 then return end
	cursor:zone('primary_down', self, function() self:handle_cursor_click() end)

	local ass = assdraw.ass_new()
	local is_clickable = self.is_clickable and self.on_click ~= nil
	local is_hover = self.proximity_raw <= 0
	local foreground = self.active and self.background or self.foreground
	local background = self.active and self.foreground or self.background
	local background_opacity = self.active and 1 or config.opacity.controls

	if is_hover and is_clickable and background_opacity < 0.3 then background_opacity = 0.3 end

	-- 背景
	if background_opacity > 0 then
		ass:rect(self.ax, self.ay, self.bx, self.by, {
			color = (self.active or not is_hover) and background or foreground,
			radius = state.radius,
			opacity = visibility * background_opacity,
		})
	end

	-- hover 时显示 tooltip
	if is_hover and self.tooltip then ass:tooltip(self, self.tooltip) end

	-- 角标
	local icon_clip
	if self.badge then
		local badge_font_size = self.font_size * 0.6
		local badge_opts = {size = badge_font_size, color = background, opacity = visibility}
		local badge_width = text_width(self.badge, badge_opts)
		local width, height = math.ceil(badge_width + (badge_font_size / 7) * 2), math.ceil(badge_font_size * 0.93)
		local bx, by = self.bx - 1, self.by - 1
		ass:rect(bx - width, by - height, bx, by, {
			color = foreground,
			radius = state.radius,
			opacity = visibility,
			border = self.active and 0 or 1,
			border_color = background,
		})
		ass:txt(bx - width / 2, by - height / 2, 5, self.badge, badge_opts)

		local clip_border = math.max(self.font_size / 20, 1)
		local clip_path = assdraw.ass_new()
		clip_path:round_rect_cw(
			math.floor((bx - width) - clip_border), math.floor((by - height) - clip_border), bx, by, 3
		)
		icon_clip = '\\iclip(' .. clip_path.scale .. ', ' .. clip_path.text .. ')'
	end

	-- 图标
	local x, y = round(self.ax + (self.bx - self.ax) / 2), round(self.ay + (self.by - self.ay) / 2)
	ass:icon(x, y, self.font_size, self.icon, {
		color = foreground,
		border = self.active and 0 or options.text_border * state.scale,
		border_color = background,
		opacity = visibility,
		clip = icon_clip,
	})

	return ass
end

return Button
