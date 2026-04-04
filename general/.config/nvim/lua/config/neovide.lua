local math_utils = require("utils.math")
local os_utils = require("utils.os")

local M = {}

--- 将字体缩放值裁剪并保留两位小数。
---@param scale number
---@param min_value number
---@param max_value number
---@return number
local function normalize_font_scale(scale, min_value, max_value)
    return math_utils.clamp(math_utils.round(scale, 2), min_value, max_value)
end

--- 配置 Neovide 的字体、动画与缩放快捷键。
function M.setup()
    local os = os_utils.get_os()

    if os == "Linux" then
        vim.o.guifont = "VictorMono Nerd Font Mono:h14"
    elseif os == "Macos" then
        vim.o.guifont = "Victor Mono:h14"
    end

    local font_scale_step = 0.1
    local font_scale_min = 0.6
    local font_scale_max = 2.0
    local font_scale_default = 1.0

    --- 调整当前 Neovide 字体缩放比例。
    ---@param delta number
    local function adjust_font_scale(delta)
        local current = vim.g.neovide_scale_factor or font_scale_default
        local snapped = math_utils.round(current / font_scale_step) * font_scale_step
        local next_scale = normalize_font_scale(snapped + delta, font_scale_min, font_scale_max)
        vim.g.neovide_scale_factor = next_scale
    end

    vim.g.neovide_scale_factor = normalize_font_scale(
        vim.g.neovide_scale_factor or font_scale_default,
        font_scale_min,
        font_scale_max
    )

    vim.g.neovide_scroll_animation_length = 0.2
    vim.g.neovide_normal_opacity = 1
    vim.g.neovide_cursor_animation_length = 0.12
    vim.g.neovide_cursor_trail_size = 2
    vim.g.neovide_cursor_animate_in_insert_mode = true

    local inc_font_key, dec_font_key
    if os == "Linux" then
        inc_font_key = "<C-+>"
        dec_font_key = "<C-_>"
    elseif os == "Macos" then
        inc_font_key = "<C-=>"
        dec_font_key = "<C-->"
    end

    vim.keymap.set("n", inc_font_key, function()
        adjust_font_scale(font_scale_step)
    end, { desc = "Increase Font Size" })

    vim.keymap.set("n", dec_font_key, function()
        adjust_font_scale(-font_scale_step)
    end, { desc = "Decrease Font Size" })
end

return M
