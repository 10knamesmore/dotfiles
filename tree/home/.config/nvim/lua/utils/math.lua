local M = {}

--- 按指定精度对数字进行四舍五入。
---@param value number
---@param precision? integer
---@return number
function M.round(value, precision)
    local factor = 10 ^ (precision or 0)
    return math.floor(value * factor + 0.5) / factor
end

--- 将数值限制在给定区间内。
---@param value number
---@param min_value number
---@param max_value number
---@return number
function M.clamp(value, min_value, max_value)
    return math.max(min_value, math.min(max_value, value))
end

return M
