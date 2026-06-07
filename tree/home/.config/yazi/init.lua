require("full-border"):setup({
	type = ui.Border.ROUNDED,
})

-- git 状态标记（fetcher 注册在 yazi.toml [[plugin.prepend_fetchers]]）
require("git"):setup({
	order = 1500, -- 状态符在 linemode 区的排序权重（越大越靠右）
})

-- 自定义 linemode：大小 + 修改时间 同显（yazi.toml 里 linemode = "size_and_mtime"）
function Linemode:size_and_mtime()
	local time = math.floor(self._file.cha.mtime or 0)
	local size = self._file:size()
	return string.format(
		"%s %s",
		size and ya.readable_size(size) or "-",
		time ~= 0 and os.date("%y-%m-%d %H:%M", time) or "-"
	)
end
