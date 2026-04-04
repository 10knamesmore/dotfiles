---@class util.lualine
local M = {}

--- 根据状态函数返回值构造带高亮的图标组件。
---@param icon string
---@param status fun(): nil|"ok"|"error"|"pending"
function M.status(icon, status)
    local colors = {
        ok = "Special",
        error = "DiagnosticError",
        pending = "DiagnosticWarn",
    }
    return {
        function()
            return icon
        end,
        cond = function()
            return status() ~= nil
        end,
        color = function()
            return { fg = Snacks.util.color(colors[status()] or colors.ok) }
        end,
    }
end

--- 按指定高亮组包装 lualine 文本片段。
---@param component any
---@param text string
---@param hl_group? string
---@return string
function M.format(component, text, hl_group)
    text = text:gsub("%%", "%%%%")
    if not hl_group or hl_group == "" then
        return text
    end
    ---@type table<string, string>
    component.hl_cache = component.hl_cache or {}
    local lualine_hl_group = component.hl_cache[hl_group]
    if not lualine_hl_group then
        local utils = require("lualine.utils.utils")
        ---@type string[]
        local gui = vim.tbl_filter(function(x)
            return x
        end, {
            utils.extract_highlight_colors(hl_group, "bold") and "bold",
            utils.extract_highlight_colors(hl_group, "italic") and "italic",
        })

        lualine_hl_group = component:create_hl({
            fg = utils.extract_highlight_colors(hl_group, "fg"),
            gui = #gui > 0 and table.concat(gui, ",") or nil,
        }, "LV_" .. hl_group) --[[@as string]]
        component.hl_cache[hl_group] = lualine_hl_group
    end
    return component:format_hl(lualine_hl_group) .. text .. component:get_default_hl()
end

--- 规范化文件路径，展开 `~` 并统一路径分隔符。
---@param path string
---@return string
function M.norm(path)
    if path:sub(1, 1) == "~" then
        local home = vim.uv.os_homedir()
        if home and (home:sub(-1) == "\\" or home:sub(-1) == "/") then
            home = home:sub(1, -2)
        end
        path = home .. path:sub(2)
    end
    path = path:gsub("\\", "/"):gsub("/+", "/")
    return path:sub(-1) == "/" and path:sub(1, -2) or path
end

--- 生成用于 lualine 的紧凑路径显示函数。
---@param opts? {relative: "cwd"|"root", modified_hl: string?, directory_hl: string?, filename_hl: string?, modified_sign: string?, readonly_icon: string?, length: number?}
---@return fun(self:any): string
function M.pretty_path(opts)
    opts = vim.tbl_extend("force", {
        relative = "cwd",
        modified_hl = "MatchParen",
        directory_hl = "",
        filename_hl = "Bold",
        modified_sign = "",
        readonly_icon = " 󰌾 ",
        length = 3,
    }, opts or {})

    return function(self)
        local path = vim.fn.expand("%:p") --[[@as string]]

        if path == "" then
            return ""
        end

        path = M.norm(path)
        local root = utils.path.get_root({ normalize = true })
        local cwd = utils.path.cwd()

        -- original path is preserved to provide user with expected result of pretty_path, not a normalized one,
        -- which might be confusing
        local norm_path = path

        if opts.relative == "cwd" and norm_path:find(cwd, 1, true) == 1 then
            path = path:sub(#cwd + 2)
        elseif norm_path:find(root, 1, true) == 1 then
            path = path:sub(#root + 2)
        end

        local sep = package.config:sub(1, 1)
        local parts = vim.split(path, "[\\/]")

        if opts.length == 0 then
            parts = parts
        elseif #parts > opts.length then
            parts = { parts[1], "…", unpack(parts, #parts - opts.length + 2, #parts) }
        end

        if opts.modified_hl and vim.bo.modified then
            parts[#parts] = parts[#parts] .. opts.modified_sign
            parts[#parts] = M.format(self, parts[#parts], opts.modified_hl)
        else
            parts[#parts] = M.format(self, parts[#parts], opts.filename_hl)
        end

        local dir = ""
        if #parts > 1 then
            dir = table.concat({ unpack(parts, 1, #parts - 1) }, sep)
            dir = M.format(self, dir .. sep, opts.directory_hl)
        end

        local readonly = ""
        if vim.bo.readonly then
            readonly = M.format(self, opts.readonly_icon, opts.modified_hl)
        end
        return dir .. parts[#parts] .. readonly
    end
end

return M
