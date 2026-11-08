local M = {}

--- 用临时 help buffer 让原生 :help 在当前窗口完成标签跳转。
--- 原 buffer 保留；找不到主题时恢复原 buffer 和阅读位置。
---@param subject? string
function M.open(subject)
  subject = subject and subject ~= "" and subject or "help.txt"
  if vim.bo.buftype == "help" then
    vim.cmd.help(subject)
    return
  end
  local buffer, saved = vim.api.nvim_get_current_buf(), vim.fn.winsaveview()
  vim.cmd.enew()
  vim.bo.bufhidden = "wipe"
  vim.bo.buftype = "help"
  local ok, err = pcall(vim.cmd.help, subject)
  if not ok then
    vim.api.nvim_set_current_buf(buffer)
    vim.fn.winrestview(saved)
    error(err, 0)
  end
end

return M
