-- lazygit 与 nvim 的接缝。
--
-- snacks 生成的 lazygit 配置（lua/plugins/ui/core/snacks.lua 的 lazygit.opts）把 os.edit
-- 指向这里的 hide()：lazygit 浮窗占着当前窗口，文件直接 --remote-silent 只会开进浮窗或新 tab。

local M = {}

--- 最近一次由 open() 打开的 lazygit 终端。
---
--- snacks 没有 "按条件找终端" 的查询接口，hide() 只能自己存着这个对象。
---@type snacks.terminal?
local lazygit

--- 打开 lazygit（<leader>gg 与 dashboard 的 g 都走这里）。
---@param opts? snacks.lazygit.Config
function M.open(opts)
  lazygit = require("snacks").lazygit(opts)
  return lazygit
end

--- 关掉 lazygit 浮窗，把窗口让回打开 lazygit 之前的位置，lazygit 进程留在后台。
---
--- 必须走 snacks 的 hide：直接 nvim_win_close 的话 snacks 不知道窗口已经没了，
--- 下次 <leader>gg 会另起一个 lazygit，而不是唤回这个会话。
---@return boolean 是否真的关掉了窗口
function M.hide()
  if not (lazygit and lazygit:valid()) then
    return false
  end
  lazygit:hide()
  return true
end

return M
