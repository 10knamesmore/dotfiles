--[[ The MIT License (MIT)

Copyright (c) 2020 Seth Warn

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE. ]]

-- fzy 字符串匹配算法的 lua 实现

local SCORE_GAP_LEADING = -0.005
local SCORE_GAP_TRAILING = -0.005
local SCORE_GAP_INNER = -0.01
local SCORE_MATCH_CONSECUTIVE = 1.0
local SCORE_MATCH_SLASH = 0.9
local SCORE_MATCH_WORD = 0.8
local SCORE_MATCH_CAPITAL = 0.7
local SCORE_MATCH_DOT = 0.6
local SCORE_MAX = math.huge
local SCORE_MIN = -math.huge
local MATCH_MAX_LENGTH = 1024

local fzy = {}

-- 检查 `needle` 是否是 `haystack` 的子序列。
--
-- 通常在 `score` 或 `positions` 之前调用。
--
-- 参数：
--   needle (string)
--   haystack (string)
--   case_sensitive (bool, 可选): 默认为 false
--
-- 返回：
--   bool
function fzy.has_match(needle, haystack, case_sensitive)
	if not case_sensitive then
		needle = string.lower(needle)
		haystack = string.lower(haystack)
	end

	local j = 1
	for i = 1, string.len(needle) do
		j = string.find(haystack, needle:sub(i, i), j, true)
		if not j then
			return false
		else
			j = j + 1
		end
	end

	return true
end

local function is_lower(c)
	return c:match("%l")
end

local function is_upper(c)
	return c:match("%u")
end

local function precompute_bonus(haystack)
	local match_bonus = {}

	local last_char = "/"
	for i = 1, string.len(haystack) do
		local this_char = haystack:sub(i, i)
		if last_char == "/" or last_char == "\\" then
			match_bonus[i] = SCORE_MATCH_SLASH
		elseif last_char == "-" or last_char == "_" or last_char == " " then
			match_bonus[i] = SCORE_MATCH_WORD
		elseif last_char == "." then
			match_bonus[i] = SCORE_MATCH_DOT
		elseif is_lower(last_char) and is_upper(this_char) then
			match_bonus[i] = SCORE_MATCH_CAPITAL
		else
			match_bonus[i] = 0
		end

		last_char = this_char
	end

	return match_bonus
end

local function compute(needle, haystack, D, M, case_sensitive)
	-- 注意：匹配加成必须在参数转为小写之前计算，
	-- 因为 camelCase 形式存在加成。
	local match_bonus = precompute_bonus(haystack)
	local n = string.len(needle)
	local m = string.len(haystack)

	if not case_sensitive then
		needle = string.lower(needle)
		haystack = string.lower(haystack)
	end

	-- 因为 lua 只能通过提取子串来访问字符，
	-- 所以现在一次性取出 haystack 的所有字符，供下面复用。
	local haystack_chars = {}
	for i = 1, m do
		haystack_chars[i] = haystack:sub(i, i)
	end

	for i = 1, n do
		D[i] = {}
		M[i] = {}

		local prev_score = SCORE_MIN
		local gap_score = i == n and SCORE_GAP_TRAILING or SCORE_GAP_INNER
		local needle_char = needle:sub(i, i)

		for j = 1, m do
			if needle_char == haystack_chars[j] then
				local score = SCORE_MIN
				if i == 1 then
					score = ((j - 1) * SCORE_GAP_LEADING) + match_bonus[j]
				elseif j > 1 then
					local a = M[i - 1][j - 1] + match_bonus[j]
					local b = D[i - 1][j - 1] + SCORE_MATCH_CONSECUTIVE
					score = math.max(a, b)
				end
				D[i][j] = score
				prev_score = math.max(score, prev_score + gap_score)
				M[i][j] = prev_score
			else
				D[i][j] = SCORE_MIN
				prev_score = prev_score + gap_score
				M[i][j] = prev_score
			end
		end
	end
end

-- 计算匹配得分。
--
-- 参数：
--   needle (string): 必须是 `haystack` 的子序列，否则结果未定义。
--   haystack (string)
--   case_sensitive (bool, 可选): 默认为 false
--
-- 返回：
--   number: 得分越高表示匹配越好。另见 `get_score_min`
--     与 `get_score_max`。
function fzy.score(needle, haystack, case_sensitive)
	local n = string.len(needle)
	local m = string.len(haystack)

	if n == 0 or m == 0 or m > MATCH_MAX_LENGTH or n > m then
		return SCORE_MIN
	elseif n == m then
		return SCORE_MAX
	else
		local D = {}
		local M = {}
		compute(needle, haystack, D, M, case_sensitive)
		return M[n][m]
	end
end

-- 计算 fzy 匹配字符串的位置。
--
-- 在最优匹配中，确定 `needle` 的每个字符在 `haystack` 中匹配到的位置。
--
-- 参数：
--   needle (string): 必须是 `haystack` 的子序列，否则结果未定义。
--   haystack (string)
--   case_sensitive (bool, 可选): 默认为 false
--
-- 返回：
--   {int,...}: 下标数组，其中 `indices[n]` 是 `needle` 的第 `n` 个
--     字符在 `haystack` 中的位置。
--   number: 与 `score` 返回的相同匹配得分
function fzy.positions(needle, haystack, case_sensitive)
	local n = string.len(needle)
	local m = string.len(haystack)

	if n == 0 or m == 0 or m > MATCH_MAX_LENGTH or n > m then
		return {}, SCORE_MIN
	elseif n == m then
		local consecutive = {}
		for i = 1, n do
			consecutive[i] = i
		end
		return consecutive, SCORE_MAX
	end

	local D = {}
	local M = {}
	compute(needle, haystack, D, M, case_sensitive)

	local positions = {}
	local match_required = false
	local j = m
	for i = n, 1, -1 do
		while j >= 1 do
			if D[i][j] ~= SCORE_MIN and (match_required or D[i][j] == M[i][j]) then
				match_required = (i ~= 1) and (j ~= 1) and (
				M[i][j] == D[i - 1][j - 1] + SCORE_MATCH_CONSECUTIVE)
				positions[i] = j
				j = j - 1
				break
			else
				j = j - 1
			end
		end
	end

	return positions, M[n][m]
end

-- 对一个 haystacks 数组应用 `has_match` 和 `positions`。
--
-- 参数：
--   needle (string)
--   haystack ({string, ...})
--   case_sensitive (bool, 可选): 默认为 false
--
-- 返回：
--   {{idx, positions, score}, ...}: 一个数组，`haystacks` 中每个匹配行对应一项，
--     每项给出该行在 `haystacks` 中的下标，
--     以及该行对应 `positions` 返回值的等价结果。
function fzy.filter(needle, haystacks, case_sensitive)
	local result = {}

	for i, line in ipairs(haystacks) do
		if fzy.has_match(needle, line, case_sensitive) then
			local p, s = fzy.positions(needle, line, case_sensitive)
			table.insert(result, {i, p, s})
		end
	end

	return result
end

-- `score` 返回的最小值。
--
-- 在两种特殊情况下：
--  - `needle` 为空，或
--  - `needle` 或 `haystack` 大于 `get_max_length`，
-- `score` 函数会返回这个确切的值，可用作哨兵值。这是可能的最低得分。
function fzy.get_score_min()
	return SCORE_MIN
end

-- 完全匹配时返回的得分。这是可能的最高得分。
function fzy.get_score_max()
	return SCORE_MAX
end

-- `fzy` 会计算得分的最大长度。
function fzy.get_max_length()
	return MATCH_MAX_LENGTH
end

-- 普通匹配返回的最小得分。
--
-- 对于不返回 `get_score_min` 的匹配，其得分会大于此值。
function fzy.get_score_floor()
	return MATCH_MAX_LENGTH * SCORE_GAP_INNER
end

-- 非完全匹配的最大得分。
--
-- 对于不返回 `get_score_max` 的匹配，其得分会小于此值。
function fzy.get_score_ceiling()
	return MATCH_MAX_LENGTH * SCORE_MATCH_CONSECUTIVE
end

-- 当前运行实现的名称，"lua" 或 "native"。
function fzy.get_implementation_name()
	return "lua"
end

return fzy
