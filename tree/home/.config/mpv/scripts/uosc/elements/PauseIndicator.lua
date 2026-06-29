local Element = require('elements/Element')

---@class PauseIndicator : Element
local PauseIndicator = class(Element)

function PauseIndicator:new() return Class.new(self) --[[@as PauseIndicator]] end
function PauseIndicator:init()
	Element.init(self, 'pause_indicator', {render_order = 3})
	self.ignores_curtain = true
	self.paused = state.pause
	self.opacity = 0
	self.fadeout = false
	self:init_options()
end

function PauseIndicator:init_options()
	self.base_icon_opacity = options.pause_indicator == 'flash' and 1 or 0.8
	self.type = options.pause_indicator
	self:on_prop_pause()
end

function PauseIndicator:flash()
	-- 不能等 pause property 的事件监听器来设置它，因为当它被用在这样的 binding 里时：
	-- cycle pause; script-binding uosc/flash-pause-indicator
	-- pause 事件触发得不够快，导致 indicator 一开始用旧 icon 渲染。
	self.paused = mp.get_property_native('pause')
	self.fadeout, self.opacity = false, 1
	self:tween_property('opacity', 1, 0, 300)
end

-- 决定静态 indicator 是否应该显示。
function PauseIndicator:decide()
	self.paused = mp.get_property_native('pause') -- 这一行为何必要见 flash()
	self.fadeout, self.opacity = self.paused, self.paused and 1 or 0
	request_render()

	-- 绕过 windows 构建在 pause 时的一个 mpv 竞态条件 bug，该 bug 会导致 osd 更新被忽略。
	-- .03 仍会丢渲染，.04 没问题，但保险起见我多加了 10ms
	mp.add_timeout(.05, function() osd:update() end)
end

function PauseIndicator:on_prop_pause()
	if Elements:v('timeline', 'pressed') then return end
	if options.pause_indicator == 'flash' then
		if self.paused ~= state.pause then self:flash() end
	elseif options.pause_indicator == 'static' then
		self:decide()
	end
end

function PauseIndicator:on_options()
	self:init_options()
	if self.type == 'flash' then self.opacity = 0 end
end

function PauseIndicator:render()
	if self.opacity == 0 then return end

	local ass = assdraw.ass_new()

	-- 背景淡出
	if self.fadeout then
		ass:rect(0, 0, display.width, display.height, {color = bg, opacity = self.opacity * 0.3})
	end

	-- 图标
	local size = round(math.min(display.width, display.height) * (self.fadeout and 0.20 or 0.15))
	size = size + size * (1 - self.opacity)

	if self.paused then
		ass:icon(display.width / 2, display.height / 2, size, 'pause',
			{border = 1, opacity = self.base_icon_opacity * self.opacity}
		)
	else
		ass:icon(display.width / 2, display.height / 2, size * 1.2, 'play_arrow',
			{border = 1, opacity = self.base_icon_opacity * self.opacity}
		)
	end

	return ass
end

return PauseIndicator
