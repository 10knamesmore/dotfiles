--[[ UI 相关的工具函数，可能依赖也可能不依赖其 state 或 options ]]

---@alias Point {x: number; y: number}
---@alias Rect {ax: number, ay: number, bx: number, by: number, window_drag?: boolean}
---@alias Circle {point: Point, r: number, window_drag?: boolean}
---@alias Hitbox Rect|Circle
---@alias ComplexBindingInfo {event: 'down' | 'repeat' | 'up' | 'press'; is_mouse: boolean; canceled: boolean; key_name?: string; key_text?: string;}

-- 字符串排序
do
	----- winapi start -----
	-- 在 windows 系统上，可以使用 win32 API 提供的排序函数
	-- 参见 https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-strcmplogicalw
	-- 此函数取自 https://github.com/mpvnet-player/mpv.net/issues/575#issuecomment-1817413401
	local winapi = nil

	if state.platform == 'windows' and config.refine.sorting then
		-- is_ffi_loaded 为 false 通常意味着 mpv 编译时未带 luajit
		local is_ffi_loaded, ffi = pcall(require, 'ffi')

		if is_ffi_loaded then
			winapi = {
				ffi = ffi,
				C = ffi.C,
				CP_UTF8 = 65001,
				shlwapi = ffi.load('shlwapi'),
			}

			-- ffi 代码来自 https://github.com/po5/thumbfast，Mozilla Public License Version 2.0
			ffi.cdef [[
				int __stdcall MultiByteToWideChar(unsigned int CodePage, unsigned long dwFlags, const char *lpMultiByteStr,
				int cbMultiByte, wchar_t *lpWideCharStr, int cchWideChar);
				int __stdcall StrCmpLogicalW(wchar_t *psz1, wchar_t *psz2);
			]]

			winapi.utf8_to_wide = function(utf8_str)
				if utf8_str then
					local utf16_len = winapi.C.MultiByteToWideChar(winapi.CP_UTF8, 0, utf8_str, -1, nil, 0)

					if utf16_len > 0 then
						local utf16_str = winapi.ffi.new('wchar_t[?]', utf16_len)

						if winapi.C.MultiByteToWideChar(winapi.CP_UTF8, 0, utf8_str, -1, utf16_str, utf16_len) > 0 then
							return utf16_str
						end
					end
				end

				return ''
			end
		end
	end
	----- winapi end -----

	-- Lua 中符合人类直觉的字母数字混合排序
	-- http://notebook.kulchenko.com/algorithms/alphanumeric-natural-sorting-for-humans-in-lua
	local function padnum(n, d)
		return #d > 0 and ('%03d%s%.12f'):format(#n, n, tonumber(d) / (10 ^ #d))
			or ('%03d%s'):format(#n, n)
	end

	local function sort_lua(strings)
		local tuples = {}
		for i, f in ipairs(strings) do
			tuples[i] = {f:lower():gsub('0*(%d+)%.?(%d*)', padnum), f}
		end
		table.sort(tuples, function(a, b)
			return a[1] == b[1] and #b[2] < #a[2] or a[1] < b[1]
		end)
		for i, tuple in ipairs(tuples) do strings[i] = tuple[2] end
		return strings
	end

	---@param strings string[]
	function sort_strings(strings)
		if winapi then
			table.sort(strings, function(a, b)
				return winapi.shlwapi.StrCmpLogicalW(winapi.utf8_to_wide(a), winapi.utf8_to_wide(b)) == -1
			end)
		else
			sort_lua(strings)
		end
	end
end

-- 生成中间帧，将数值从 `from` 平滑动画过渡到 `to`。
---@param from number
---@param to number|fun():number
---@param setter fun(value: number)
---@param duration_or_callback? number|fun() 以毫秒为单位的时长，或一个回调函数。
---@param callback? fun() 在动画结束或动画被终止时调用。
function tween(from, to, setter, duration_or_callback, callback)
	local duration = duration_or_callback
	if type(duration_or_callback) == 'function' then callback = duration_or_callback end
	if type(duration) ~= 'number' then duration = options.animation_duration end

	local current, done, timeout = from, false, nil
	local get_to = type(to) == 'function' and to or function() return to --[[@as number]] end
	local distance = math.abs(get_to() - current)
	local cutoff = distance * 0.01
	local target_ticks = (math.max(duration, 1) / (state.render_delay * 1000))
	local decay = 1 - ((cutoff / distance) ^ (1 / target_ticks))

	local function finish()
		if not done then
			setter(get_to())
			done = true
			timeout:kill()
			if callback then callback() end
			request_render()
		end
	end

	local function tick()
		local to = get_to()
		current = current + ((to - current) * decay)
		local is_end = math.abs(to - current) <= cutoff
		if is_end then
			finish()
		else
			setter(current)
			timeout:resume()
			request_render()
		end
	end

	timeout = mp.add_timeout(state.render_delay, tick)
	if cutoff > 0 then tick() else finish() end

	return finish
end

-- 返回带符号的距离（负值表示该点在矩形内部的深度）。
---@param point Point
---@param rect Rect
function get_point_to_rectangle_proximity(point, rect)
	local dx = math.max(rect.ax - point.x, point.x - rect.bx)
	local dy = math.max(rect.ay - point.y, point.y - rect.by)
	local distance = math.sqrt(math.max(0, dx)^2 + math.max(0, dy)^2)
	return distance + math.min(0, math.max(dx, dy))
end

---@param point_a Point
---@param point_b Point
function get_point_to_point_proximity(point_a, point_b)
	local dx, dy = point_a.x - point_b.x, point_a.y - point_b.y
	return math.sqrt(dx * dx + dy * dy)
end

---@param point Point
---@param hitbox Hitbox
function point_collides_with(point, hitbox)
	return (hitbox.r and get_point_to_point_proximity(point, hitbox.point) <= hitbox.r) or
		(not hitbox.r and get_point_to_rectangle_proximity(point, hitbox --[[@as Rect]]) <= 0)
end

---@param lax number
---@param lay number
---@param lbx number
---@param lby number
---@param max number
---@param may number
---@param mbx number
---@param mby number
function get_line_to_line_intersection(lax, lay, lbx, lby, max, may, mbx, mby)
	-- 计算两条线的方向
	local uA = ((mbx - max) * (lay - may) - (mby - may) * (lax - max)) /
		((mby - may) * (lbx - lax) - (mbx - max) * (lby - lay))
	local uB = ((lbx - lax) * (lay - may) - (lby - lay) * (lax - max)) /
		((mby - may) * (lbx - lax) - (mbx - max) * (lby - lay))

	-- 若 uA 与 uB 都在 0-1 之间，则两条线相交
	if uA >= 0 and uA <= 1 and uB >= 0 and uB <= 1 then
		return lax + (uA * (lbx - lax)), lay + (uA * (lby - lay))
	end

	return nil, nil
end

-- 返回从一条有限射线起点（假定位于 (rax, ray) 坐标）
-- 到某条线的距离。
---@param rax number
---@param ray number
---@param rbx number
---@param rby number
---@param lax number
---@param lay number
---@param lbx number
---@param lby number
function get_ray_to_line_distance(rax, ray, rbx, rby, lax, lay, lbx, lby)
	local x, y = get_line_to_line_intersection(rax, ray, rbx, rby, lax, lay, lbx, lby)
	if x then
		return math.sqrt((rax - x) ^ 2 + (ray - y) ^ 2)
	end
	return nil
end

-- 返回从一条有限射线起点（假定位于 (ax, ay) 坐标）
-- 到某个矩形的距离。若射线起点位于矩形内部则返回 `0`。
---@param  ax number
---@param  ay number
---@param  bx number
---@param  by number
---@param  rect Rect
---@return number|nil
function get_ray_to_rectangle_distance(ax, ay, bx, by, rect)
	-- 在内部
	if ax >= rect.ax and ax <= rect.bx and ay >= rect.ay and ay <= rect.by then
		return 0
	end

	local closest = nil

	local function updateDistance(distance)
		if distance and (not closest or distance < closest) then closest = distance end
	end

	updateDistance(get_ray_to_line_distance(ax, ay, bx, by, rect.ax, rect.ay, rect.bx, rect.ay))
	updateDistance(get_ray_to_line_distance(ax, ay, bx, by, rect.bx, rect.ay, rect.bx, rect.by))
	updateDistance(get_ray_to_line_distance(ax, ay, bx, by, rect.ax, rect.by, rect.bx, rect.by))
	updateDistance(get_ray_to_line_distance(ax, ay, bx, by, rect.ax, rect.ay, rect.ax, rect.by))

	return closest
end

-- 通过 Catmull-Rom 到 Bezier 的转换，将扁平的点集表转换为平滑曲线。
---@param points number[] 扁平表：x1, y1, x2, y2, ...
---@return number[] 扁平表：起点后跟各段数据 cp1x, cp1y, cp2x, cp2y, px, py, ...
function points_to_bezier(points)
	if not points or #points < 4 then return {} end
	local function catmullrom_to_bezier(p0x, p0y, p1x, p1y, p2x, p2y, p3x, p3y)
		local cp1x = p1x + (p2x - p0x) / 6
		local cp1y = p1y + (p2y - p0y) / 6
		local cp2x = p2x - (p3x - p1x) / 6
		local cp2y = p2y - (p3y - p1y) / 6
		return cp1x, cp1y, cp2x, cp2y
	end
	-- 从扁平表中取出 x, y 的辅助函数
	local function get_xy(i)
		return points[i * 2 - 1], points[i * 2]
	end
	local curve = {points[1], points[2]}
	local xy_pairs = #points / 2
	for i = 1, xy_pairs - 1 do
		local p0x, p0y = get_xy(math.max(i - 1, 1))
		local p1x, p1y = get_xy(i)
		local p2x, p2y = get_xy(i+1)
		local p3x, p3y = get_xy(math.min(i + 2, xy_pairs))
		local cp1x, cp1y, cp2x, cp2y = catmullrom_to_bezier(p0x, p0y, p1x, p1y, p2x, p2y, p3x, p3y)
		local n = #curve
		curve[n+1], curve[n+2], curve[n+3], curve[n+4], curve[n+5], curve[n+6] =
			cp1x, cp1y, cp2x, cp2y, p2x, p2y
	end
	return curve
end

-- 提取该字符串在 property 展开时所用到的 property。
---@param str string
---@param res { [string] : boolean } | nil
---@return { [string] : boolean }
function get_expansion_props(str, res)
	res = res or {}
	for str in str:gmatch('%$(%b{})') do
		local name, str = str:match('^{[?!]?=?([^:]+):?(.*)}$')
		if name then
			local s = name:find('==') or nil
			if s then name = name:sub(0, s - 1) end
			res[name] = true
			if str and str ~= '' then get_expansion_props(str, res) end
		end
	end
	return res
end

-- 转义字符串，使其能在 OSD 上原样显示。
---@param str string
function ass_escape(str)
	-- ASS 中没有针对 '\' 的转义（大概吧？），但 '\' 若后面没有跟可识别的
	-- 字符，就会被原样使用，所以在其后加一个零宽
	-- 非断行空格（ZWNBSP）
	str = str:gsub('\\', '\\\239\187\191')
	str = str:gsub('{', '\\{')
	str = str:gsub('}', '\\}')
	-- 在换行符前加一个 ZWNBSP，以防止 ASS 把
	-- 连续换行符诡异地合并
	str = str:gsub('\n', '\239\187\191\\N')
	-- 把行首空格转为硬空格，防止 ASS 将其剥除
	str = str:gsub('\\N ', '\\N\\h')
	str = str:gsub('^ ', '\\h')
	return str
end

---@param seconds number
---@param max_seconds number|nil 若时间预计达不到该量级，则裁掉多余的 `00:`。
---@return string
function format_time(seconds, max_seconds)
	local human = mp.format_time(seconds)
	if options.time_precision > 0 then
		local formatted = string.format('%.' .. options.time_precision .. 'f', math.abs(seconds) % 1)
		human = human .. '.' .. string.sub(formatted, 3)
	end
	if max_seconds then
		local trim_length = (max_seconds < 60 and 7 or (max_seconds < 3600 and 4 or 0))
		if trim_length > 0 then
			local has_minus = seconds < 0
			human = string.sub(human, trim_length + (has_minus and 1 or 0))
			if has_minus then human = '-' .. human end
		end
	end
	return human
end

---@param opacity number 0-1
function opacity_to_alpha(opacity)
	return 255 - math.ceil(255 * opacity)
end

path_separator = (function()
	local os_separator = state.platform == 'windows' and '\\' or '/'

	-- 为给定路径获取合适的路径分隔符。
	---@param path string
	---@return string
	return function(path)
		return path:sub(1, 2) == '\\\\' and '\\' or os_separator
	end
end)()

-- 用与操作系统相符的路径分隔符或 UNC 分隔符拼接路径。
---@param p1 string
---@param p2 string
---@return string
function join_path(p1, p2)
	local p1, separator = trim_trailing_separator(p1)
	-- 避免在拼接盘符时多加一个冗余分隔符（`C:\\foo`），
	-- 因为 `trim_trailing_separator()` 不会裁掉盘符上的分隔符。
	return p1:sub(#p1) == separator and p1 .. p2 or p1 .. separator .. p2
end

-- 检查路径是否为绝对路径。
---@param path string
---@return boolean
function is_absolute(path)
	if path:sub(1, 2) == '\\\\' then
		return true
	elseif state.platform == 'windows' then
		return path:find('^%a+:') ~= nil
	else
		return path:sub(1, 1) == '/'
	end
end

-- 确保路径为绝对路径。
---@param path string
---@return string
function ensure_absolute(path)
	if is_absolute(path) then return path end
	return join_path(state.cwd, path)
end

-- 移除末尾的斜杠/反斜杠。
---@param path string
---@return string path, string trimmed_separator_type
function trim_trailing_separator(path)
	local separator = path_separator(path)
	path = trim_end(path, separator)
	if state.platform == 'windows' then
		-- windows 上的盘符需要末尾的反斜杠
		if path:sub(#path) == ':' then path = path .. '\\' end
	else
		if path == '' then path = '/' end
	end
	return path, separator
end

-- 确保路径为绝对路径，并移除末尾的斜杠/反斜杠。
-- normalize_path 的轻量版本，用于性能敏感的部分。
---@param path string
---@return string
function normalize_path_lite(path)
	if not path or is_protocol(path) then return path end
	path = trim_trailing_separator(ensure_absolute(path))
	return path
end

-- 确保路径为绝对路径，移除末尾的斜杠/反斜杠，并对路径分隔符做归一化与去重。
---@param path string
---@return string
function normalize_path(path)
	if not path or is_protocol(path) then return path end

	path = ensure_absolute(path)
	local is_unc = path:sub(1, 2) == '\\\\'
	if state.platform == 'windows' or is_unc then path = path:gsub('/', '\\') end
	path = trim_trailing_separator(path)

	-- 对路径分隔符去重
	if is_unc then
		path = path:gsub('(.\\)\\+', '%1')
	elseif state.platform == 'windows' then
		path = path:gsub('\\\\+', '\\')
	else
		path = path:gsub('//+', '/')
	end

	return path
end

-- 检查路径是否为某种协议，例如 `http://...`。
---@param path string
function is_protocol(path)
	return type(path) == 'string' and (path:find('^%a[%w.+-]-://') ~= nil or path:find('^%a[%w.+-]-:%?') ~= nil)
end

---@param path string
---@param extensions string[] 不带点的小写扩展名。
function has_any_extension(path, extensions)
	local path_last_dot_index = string_last_index_of(path, '.')
	if not path_last_dot_index then return false end
	local path_extension = path:sub(path_last_dot_index + 1):lower()
	for _, extension in ipairs(extensions) do
		if path_extension == extension then return true end
	end
	return false
end

-- 执行以字符串或 itable 形式定义的 mp 命令；若 command 为其他任意值则不做任何事。
-- 返回布尔值，指明命令是否被执行。
---@param command string | string[] | nil | any
---@return boolean executed 若命令被执行则为 `true`。
function execute_command(command)
	local command_type = type(command)
	if command_type == 'string' then
		mp.command(command)
		return true
	elseif command_type == 'table' and #command > 0 then
		mp.command_native(command)
		return true
	end
	return false
end

-- 将路径序列化为其语义组成部分。
---@param path string
---@return nil|{path: string; is_root: boolean; dirname?: string; basename: string; filename: string; extension?: string;}
function serialize_path(path)
	if not path or is_protocol(path) then return end

	local normal_path = normalize_path_lite(path)
	local dirname, basename = utils.split_path(normal_path)
	if basename == '' then basename, dirname = dirname:sub(1, #dirname - 1), nil end
	local dot_i = string_last_index_of(basename, '.')

	return {
		path = normal_path,
		is_root = dirname == nil,
		dirname = dirname,
		basename = basename,
		filename = dot_i and basename:sub(1, dot_i - 1) or basename,
		extension = dot_i and basename:sub(dot_i + 1) or nil,
	}
end

local system_files = create_set({
	'$RECYCLE.BIN', '$Recycle.Bin', '$SysReset', '$WinREAgent', '.sys', 'pagefile.sys', 'hiberfil.sys', 'config.sys',
	'swapfile.sys', 'Thumbs.db', 'desktop.ini',
})

-- 读取目录中的条目，并将其拆分为目录表和文件表。
---@param path string
---@param opts? {types?: string[], hidden?: boolean}
---@return string[] files
---@return string[] directories
---@return string|nil error
function read_directory(path, opts)
	opts = opts or {}
	local items, error = utils.readdir(path, 'all')
	local files, directories = {}, {}

	if not items then
		return files, directories, 'Reading directory "' .. path .. '" failed. Error: ' .. utils.to_string(error)
	end

	for _, item in ipairs(items) do
		if item ~= '.' and item ~= '..' and not system_files[item] and (opts.hidden or item:sub(1, 1) ~= '.') then
			local info = utils.file_info(join_path(path, item))
			if info then
				if info.is_file then
					if not opts.types or has_any_extension(item, opts.types) then
						files[#files + 1] = item
					end
				else
					directories[#directories + 1] = item
				end
			end
		end
	end

	return files, directories
end

-- 返回与 `file_path` 处于同一目录下所有文件的完整绝对路径，
-- 以及当前文件在该表中的索引。
-- 无论 `allowed_types` 如何，返回的表都会始终包含 `file_path`。
---@param file_path string
---@param opts? {types?: string[], hidden?: boolean}
function get_adjacent_files(file_path, opts)
	opts = opts or {}
	local current_meta = serialize_path(file_path)
	if not current_meta then return end
	local files, _dirs, error = read_directory(current_meta.dirname, {hidden = opts.hidden})
	if error then
		msg.error(error)
		return
	end
	sort_strings(files)
	local current_file_index
	local paths = {}
	for _, file in ipairs(files) do
		local is_current_file = current_meta.basename == file
		if is_current_file or not opts.types or has_any_extension(file, opts.types) then
			paths[#paths + 1] = join_path(current_meta.dirname, file)
			if is_current_file then current_file_index = #paths end
		end
	end
	if not current_file_index then return end
	return paths, current_file_index
end

-- 在列表中导航，使用 delta，或在启用 `state.shuffle` 时
-- 使用随机性来决定下一个条目。若启用了 `loop-playlist` 则循环回绕。
---@param paths table
---@param current_index number
---@param delta number 1 或 -1 分别表示前进或后退
function decide_navigation_in_list(paths, current_index, delta)
	if #paths < 2 then return end
	delta = delta < 0 and -1 or 1

	-- 随机播放会查看已播放文件历史（裁剪到 paths 长度的 80%），
	-- 并将其中所有路径从候选的随机池中移除。这保证了
	-- 在播放列表至少消耗 80% 之前不会出现路径重复。
	if state.shuffle then
		state.shuffle_history = state.shuffle_history or {
			pos = #state.history,
			paths = itable_slice(state.history),
		}
		state.shuffle_history.pos = state.shuffle_history.pos + delta
		local history_path = state.shuffle_history.paths[state.shuffle_history.pos]
		local next_index = history_path and itable_index_of(paths, history_path)
		if next_index then
			return next_index, history_path
		end
		if delta < 0 then
			state.shuffle_history.pos = state.shuffle_history.pos - delta
		else
			state.shuffle_history.pos = math.min(state.shuffle_history.pos, #state.shuffle_history.paths + 1)
		end

		local trimmed_history = itable_slice(state.history, -math.floor(#paths * 0.8))
		local shuffle_pool = {}

		for index, value in ipairs(paths) do
			if not itable_has(trimmed_history, value) then
				shuffle_pool[#shuffle_pool + 1] = index
			end
		end

		math.randomseed(os.time())
		local next_index = shuffle_pool[math.random(#shuffle_pool)]
		local next_path = paths[next_index]
		table.insert(state.shuffle_history.paths, state.shuffle_history.pos, next_path)
		return next_index, next_path
	end

	local new_index = current_index + delta
	if mp.get_property_native('loop-playlist') then
		if new_index > #paths then
			new_index = new_index % #paths
		elseif new_index < 1 then
			new_index = #paths - new_index
		end
	elseif new_index < 1 or new_index > #paths then
		return
	end

	return new_index, paths[new_index]
end

---@param delta number
function navigate_directory(delta)
	if not state.path or is_protocol(state.path) then return false end
	local paths, current_index = get_adjacent_files(state.path, {
		types = config.types.load,
		hidden = options.show_hidden_files,
	})
	if paths and current_index then
		local _, path = decide_navigation_in_list(paths, current_index, delta)
		if path then
			mp.commandv('loadfile', path)
			return true
		end
	end
	return false
end

---@param delta number
function navigate_playlist(delta)
	local playlist, pos = mp.get_property_native('playlist'), mp.get_property_native('playlist-pos-1')
	if playlist and #playlist > 1 and pos then
		local paths = itable_map(playlist, function(item) return normalize_path(item.filename) end)
		local index = decide_navigation_in_list(paths, pos, delta)
		if index then
			mp.commandv('playlist-play-index', index - 1)
			return true
		end
	end
	return false
end

---@param delta number
function navigate_item(delta)
	if state.has_playlist then return navigate_playlist(delta) else return navigate_directory(delta) end
end

-- 不能用 `os.remove()`，因为它在含 unicode 字符的路径上会失败。
-- 返回 `result, error`，result 是如下结构的表：
-- `status:number(<0=error), stdout, stderr, error_string, killed_by_us:boolean`
---@param path string
function delete_file(path)
	if state.platform == 'windows' then
		if options.use_trash then
			local ps_code = [[
				Add-Type -AssemblyName Microsoft.VisualBasic
				[Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('__path__', 'OnlyErrorDialogs', 'SendToRecycleBin')
			]]

			local escaped_path = string.gsub(path, "'", "''")
			escaped_path = string.gsub(escaped_path, '’', '’’')
			escaped_path = string.gsub(escaped_path, '%%', '%%%%')
			ps_code = string.gsub(ps_code, '__path__', escaped_path)
			args = {'powershell', '-NoProfile', '-Command', ps_code}
		else
			args = {'cmd', '/C', 'del', path}
		end
	else
		if options.use_trash then
			-- 在 Linux 和 macOS 上必须先安装 trash-cli/trash 程序。
			args = {'trash', path}
		else
			args = {'rm', path}
		end
	end
	return mp.command_native({
		name = 'subprocess',
		args = args,
		playback_only = false,
		capture_stdout = true,
		capture_stderr = true,
	})
end

function delete_file_navigate(delta)
	local path, playlist_pos = state.path, state.playlist_pos
	local is_local_file = path and not is_protocol(path)

	if navigate_item(delta) then
		if state.has_playlist then
			mp.commandv('playlist-remove', playlist_pos - 1)
		end
	else
		mp.command('stop')
	end

	if is_local_file then
		if Menu:is_open('open-file') then
			Elements:maybe('menu', 'delete_value', path)
		end
		if path then delete_file(path) end
	end
end

function serialize_chapter_ranges(normalized_chapters)
	local ranges = {}
	local simple_ranges = {
		{
			name = 'openings',
			patterns = {
				'^op ', '^op$', ' op$',
				'^opening$', ' opening$',
			},
			requires_next_chapter = true,
		},
		{
			name = 'intros',
			patterns = {
				'^intro$', ' intro$',
				'^avant$', '^prologue$',
			},
			requires_next_chapter = true,
		},
		{
			name = 'endings',
			patterns = {
				'^ed ', '^ed$', ' ed$',
				'^ending ', '^ending$', ' ending$',
			},
		},
		{
			name = 'outros',
			patterns = {
				'^outro$', ' outro$',
				'^closing$', '^closing ',
				'^preview$', '^pv$',
			},
		},
	}
	local sponsor_ranges = {}

	-- 用备用 pattern 扩展
	for _, meta in ipairs(simple_ranges) do
		local alt_patterns = config.chapter_ranges[meta.name] and config.chapter_ranges[meta.name].patterns
		if alt_patterns then meta.patterns = itable_join(meta.patterns, alt_patterns) end
	end

	-- 克隆章节
	local chapters = {}
	for i, normalized in ipairs(normalized_chapters) do chapters[i] = table_assign({}, normalized) end

	for i, chapter in ipairs(chapters) do
		-- 简单区间
		for _, meta in ipairs(simple_ranges) do
			if config.chapter_ranges[meta.name] then
				local match = itable_find(meta.patterns, function(p) return chapter.lowercase_title:find(p) end)
				if match then
					local next_chapter = chapters[i + 1]
					if next_chapter or not meta.requires_next_chapter then
						ranges[#ranges + 1] = table_assign({
							start = chapter.time,
							['end'] = next_chapter and next_chapter.time or math.huge,
						}, config.chapter_ranges[meta.name])
					end
				end
			end
		end

		-- 赞助商区块（sponsor block）
		if config.chapter_ranges.ads then
			local id = chapter.lowercase_title:match('segment start *%(([%w]%w-)%)')
			if id then -- 来自 sponsorblock 的广告区间
				for j = i + 1, #chapters, 1 do
					local end_chapter = chapters[j]
					local end_match = end_chapter.lowercase_title:match('segment end *%(' .. id .. '%)')
					if end_match then
						local range = table_assign({
							start_chapter = chapter,
							end_chapter = end_chapter,
							start = chapter.time,
							['end'] = end_chapter.time,
						}, config.chapter_ranges.ads)
						ranges[#ranges + 1], sponsor_ranges[#sponsor_ranges + 1] = range, range
						end_chapter.is_end_only = true
						break
					end
				end -- 广告对应单个章节
			elseif not chapter.is_end_only and
				(chapter.lowercase_title:find('%[sponsorblock%]:') or chapter.lowercase_title:find('^sponsors?')) then
				local next_chapter = chapters[i + 1]
				ranges[#ranges + 1] = table_assign({
					start = chapter.time,
					['end'] = next_chapter and next_chapter.time or math.huge,
				}, config.chapter_ranges.ads)
			end
		end
	end

	-- 修正相互重叠的赞助商区块片段
	for index, range in ipairs(sponsor_ranges) do
		local next_range = sponsor_ranges[index + 1]
		if next_range then
			local delta = next_range.start - range['end']
			if delta < 0 then
				local mid_point = range['end'] + delta / 2
				range['end'], range.end_chapter.time = mid_point - 0.01, mid_point - 0.01
				next_range.start, next_range.start_chapter.time = mid_point, mid_point
			end
		end
	end
	table.sort(chapters, function(a, b) return a.time < b.time end)

	return chapters, ranges
end

-- 确保章节按时间先后顺序排列
function normalize_chapters(chapters)
	if not chapters then return {} end
	-- 确保按时间先后排序
	table.sort(chapters, function(a, b) return a.time < b.time end)
	-- 确保有标题
	for index, chapter in ipairs(chapters) do
		local chapter_number = chapter.title and string.match(chapter.title, '^Chapter (%d+)$')
		if chapter_number then
			chapter.title = t('Chapter %s', tonumber(chapter_number))
		end
		chapter.title = chapter.title ~= '(unnamed)' and chapter.title ~= '' and chapter.title or t('Chapter %s', index)
		chapter.lowercase_title = chapter.title:lower()
	end
	return chapters
end

function serialize_chapters(chapters)
	chapters = normalize_chapters(chapters)
	if not chapters then return end
	--- 这里取不到 timeline 字号，所以先归一化到 size 1，在渲染时再缩放
	local opts = {size = 1, bold = true}
	for index, chapter in ipairs(chapters) do
		chapter.index = index
		chapter.title_wrapped, chapter.title_lines = wrap_text(chapter.title, opts, 25)
		chapter.title_wrapped_width = text_width(chapter.title_wrapped, opts)
		chapter.title_wrapped = ass_escape(chapter.title_wrapped)
	end
	return chapters
end

---查找所有生效的快捷键 binding，或某个 key 对应的生效 binding
---@param key string|nil
---@return {[string]: table}|table
function find_active_keybindings(key)
	local bindings = mp.get_property_native('input-bindings', {})
	local active_map = {} -- 映射：key-name -> bind-info
	local active_table = {}
	for _, bind in pairs(bindings) do
		if bind.owner ~= 'uosc' and bind.priority >= 0 and (not key or bind.key == key) and (
				not active_map[bind.key]
				or (active_map[bind.key].is_weak and not bind.is_weak)
				or (bind.is_weak == active_map[bind.key].is_weak and bind.priority > active_map[bind.key].priority)
			)
		then
			active_table[#active_table + 1] = bind
			active_map[bind.key] = bind
		end
	end
	return key and active_map[key] or active_table
end

do
	local key_subs = {{'^#$', ''}, {anycase('sharp'), '#'}}

	-- 替换诸如 `SHARP` -> `#`、`#` -> `` 之类的内容
	---@param keybind string
	function keybind_to_human(keybind)
		for _, sub in ipairs(key_subs) do
			keybind = string.gsub(keybind, sub[1], sub[2])
		end
		return keybind
	end
end

---@param type 'sub'|'audio'|'video'
---@param path string
function load_track(type, path)
	mp.commandv(type .. '-add', path, 'cached')
	-- 若加载的是字幕轨，则假定用户也想看到它
	if type == 'sub' then
		mp.commandv('set', 'sub-visibility', 'yes')
	end
end

---@param args (string|number)[]
---@return string|nil error
---@return table data
function call_ziggy(args)
	local result = mp.command_native({
		name = 'subprocess',
		capture_stderr = true,
		capture_stdout = true,
		playback_only = false,
		args = itable_join({config.ziggy_path}, args),
	})

	if result.status ~= 0 then
		return 'Calling ziggy failed. Exit code ' .. result.status .. ': ' .. result.stdout .. result.stderr, {}
	end

	local data = utils.parse_json(result.stdout)
	if not data then
		return 'Ziggy response error. Couldn\'t parse json: ' .. result.stdout, {}
	elseif data.error then
		return 'Ziggy error: ' .. data.message, {}
	else
		return nil, data
	end
end

---@param args (string|number)[]
---@param callback fun(error: string|nil, data: table)
---@return fun() abort 用于中止该请求的函数。
function call_ziggy_async(args, callback)
	local abort_signal = mp.command_native_async({
		name = 'subprocess',
		capture_stderr = true,
		capture_stdout = true,
		playback_only = false,
		args = itable_join({config.ziggy_path}, args),
	}, function(success, result, error)
		if not success or not result or result.status ~= 0 then
			local exit_code = (result and result.status or 'unknown')
			local message = error or (result and result.stdout .. result.stderr) or ''
			callback('Calling ziggy failed. Exit code: ' .. exit_code .. ' Error: ' .. message, {})
			return
		end

		local json = result and type(result.stdout) == 'string' and result.stdout or ''
		local data = utils.parse_json(json)
		if not data then
			callback('Ziggy response error. Couldn\'t parse json: ' .. json, {})
		elseif data.error then
			callback('Ziggy error: ' .. data.message, {})
		else
			return callback(nil, data)
		end
	end)

	return function()
		mp.abort_async_command(abort_signal)
	end
end

---@return string|nil
function get_clipboard()
	local data, err = mp.get_property('clipboard/text')
	if data then
		return data
	end
	if err and err ~= 'property not found' and err ~= 'property unavailable' then
		mp.commandv('show-text', 'Get clipboard error: ' .. err)
		return nil
	end

	local err, data = call_ziggy({'get-clipboard'})
	if err then
		mp.commandv('show-text', 'Get clipboard error. See console for details.')
		msg.error(err)
	end
	return data and data.payload
end

---@param payload any
---@return string|nil payload 被复制到剪贴板的字符串。
function set_clipboard(payload)
	payload = tostring(payload)

	local success, err = mp.set_property('clipboard/text', payload)
	if success then
		mp.commandv('show-text', t('Copied to clipboard') .. ': ' .. payload, 3000)
		return payload
	end
	if err and err ~= 'property not found' and err ~= 'property unavailable' then
		mp.commandv('show-text', 'Set clipboard error: ' .. err)
		return nil
	end

	local err, data = call_ziggy({'set-clipboard', payload})
	if err then
		mp.commandv('show-text', 'Set clipboard error. See console for details.')
		msg.error(err)
	else
		mp.commandv('show-text', t('Copied to clipboard') .. ': ' .. payload, 3000)
	end
	return data and data.payload
end

-- 返回 Youtube 热度图（heatmap）数据（若可用）。
---@return number[]|nil 归一化点的扁平表（0–1）
function load_youtube_heatmap()
	if not state.path or not is_protocol(state.path) then return end
	-- 匹配 mpv 的 ytdl 白名单
	if not (state.path:match('^https?://%w+%.youtube%.com/') or
			state.path:match('^https?://youtube%.com/') or
			state.path:match('^https?://youtu%.be/')) then return end

	local r = mp.get_property_native('user-data/mpv/ytdl/json-subprocess-result')
	local ytdl_result = r and utils.parse_json(r.stdout)
	if ytdl_result and ytdl_result.heatmap then
		local data = ytdl_result.heatmap
		local max_val = 0
		local vid_length = data[#data].end_time
		for _, seg in ipairs(data) do
			max_val = math.max(max_val, seg.value)
		end
		-- 归一化并钳制，以避免热度图出现空隙
		local is_above = options.timeline_heatmap == 'above'
		local min_height, graph_height = 4, is_above and 40 or options.timeline_size
		local max_norm_y = 1 - (min_height / graph_height)
		local norm = {0, 1}
		for _, seg in ipairs(data) do
			local center_time = (seg.start_time + seg.end_time) / 2
			local norm_x = center_time / vid_length
			local norm_y = math.min(max_norm_y, 1 - (seg.value / max_val))
			norm[#norm + 1], norm[#norm + 2] = norm_x, norm_y
		end
		-- 添加最后的锚点
		local last_y = math.min(max_norm_y, 1 - (data[#data].value / max_val))
		norm[#norm + 1], norm[#norm + 2] = 1, last_y
		norm[#norm + 1], norm[#norm + 2] = 1, 1
		return points_to_bezier(norm)
	end
end

--[[ 渲染 ]]

function render()
	if not display.initialized then return end
	state.render_last_time = mp.get_time()

	cursor:clear_zones()

	-- 实际渲染
	local ass = assdraw.ass_new()

	-- 空闲指示器
	if state.is_idle and not Manager.disabled.idle_indicator then
		local smaller_side = math.min(display.width, display.height)
		local center_x, center_y, icon_size = display.width / 2, display.height / 2, math.max(smaller_side / 4, 56)
		ass:icon(center_x, center_y - icon_size / 4, icon_size, 'not_started', {
			color = fg, opacity = config.opacity.idle_indicator,
		})
		ass:txt(center_x, center_y + icon_size / 2, 8, t('Drop files or URLs to play here'), {
			size = icon_size / 4, color = fg, opacity = config.opacity.idle_indicator,
		})
	end

	-- 音频指示器
	if state.is_audio and not state.has_image and not Manager.disabled.audio_indicator
		and not (state.pause and options.pause_indicator == 'static') then
		local smaller_side = math.min(display.width, display.height)
		ass:icon(display.width / 2, display.height / 2, smaller_side / 4, 'graphic_eq', {
			color = fg, opacity = config.opacity.audio_indicator,
		})
	end

	-- 各元素（Elements）
	for _, element in Elements:ipairs() do
		if element.enabled then
			local result = element:maybe('render')
			if result then
				ass:new_event()
				ass:merge(result)
			end
		end
	end

	cursor:decide_keybinds()

	-- 提交
	if osd.res_x == display.width and osd.res_y == display.height and osd.data == ass.text then
		return
	end

	osd.res_x = display.width
	osd.res_y = display.height
	osd.data = ass.text
	osd.z = 2000
	osd:update()

	update_margins()
end

-- 请求调用 render()。
-- 之后渲染要么立即执行，要么在不久前刚调用过时被限流（rate-limited）。
state.render_timer = mp.add_timeout(0, render)
state.render_timer:kill()
function request_render()
	if state.render_timer:is_enabled() then return end
	local timeout = math.max(0, state.render_delay - (mp.get_time() - state.render_last_time))
	state.render_timer.timeout = timeout
	state.render_timer:resume()
end
