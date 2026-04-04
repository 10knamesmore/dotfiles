local M = {}

--- 获取当前操作系统的简化标识。
---@return "Linux"|"Macos"|"Win"
function M.get_os()
    local sysname = vim.uv.os_uname().sysname

    if sysname == "Linux" then
        return "Linux"
    elseif sysname == "Darwin" then
        return "Macos"
    elseif sysname == "Windows_NT" then
        return "Win"
    end

    error("Unsupported OS: " .. sysname)
end

return M
