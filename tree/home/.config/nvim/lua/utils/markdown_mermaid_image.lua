local M = {}
local api = vim.api

local function show_text(buf, text)
  local options = vim.bo[buf]
  options.modifiable = true
  api.nvim_buf_set_lines(buf, 0, -1, false, vim.split(text, "\n", { plain = true }))
  options.modifiable = false
  options.modified = false
end

local function save_png(bytes)
  -- 保留文件到 Neovim 退出，Snacks 和终端可能在分屏关闭后才读取它。
  local path = vim.fn.tempname() .. ".png"
  local file, open_error = io.open(path, "wb")
  if not file then
    return nil, open_error
  end
  local wrote, write_error = file:write(bytes)
  local closed, close_error = file:close()
  if not wrote or not closed then
    vim.uv.fs_unlink(path)
    return nil, write_error or close_error
  end
  return path
end

function M.open(source, color)
  if vim.fn.executable("mermaid-text") ~= 1 then
    vim.notify("找不到 mermaid-text", vim.log.levels.WARN)
    return
  end
  local image = Snacks and Snacks.image
  if not image or image.config.enabled == false then
    vim.notify("请启用 Snacks.image", vim.log.levels.WARN)
    return
  end
  if not image.supports_terminal() then
    vim.notify("当前终端不支持 Kitty 图形协议", vim.log.levels.WARN)
    return
  end

  local buf = api.nvim_create_buf(false, true)
  vim.bo[buf].bufhidden = "wipe"
  show_text(buf, "正在渲染 Mermaid…")
  local opened, open_error = pcall(api.nvim_open_win, buf, true, { split = "below" })
  if not opened then
    api.nvim_buf_delete(buf, { force = true })
    vim.notify(open_error, vim.log.levels.ERROR)
    return
  end

  local function on_exit(output)
    if not api.nvim_buf_is_loaded(buf) then
      return
    end
    if output.code ~= 0 or not output.stdout or output.stdout == "" then
      local stderr = vim.trim(output.stderr or "")
      local reason = output.code == 0 and "mermaid-text 未输出 PNG" or "mermaid-text 退出码 " .. output.code
      show_text(buf, stderr ~= "" and stderr or reason)
      return
    end
    local path, err = save_png(output.stdout)
    if not path then
      show_text(buf, "无法保存 Mermaid PNG: " .. err)
      return
    end
    show_text(buf, "")
    image.buf.attach(buf, { src = path })
  end

  local cmd = { "mermaid-text", "--png" }
  if color == false then
    cmd[#cmd + 1] = "--no-color"
  end
  local ok, job = pcall(vim.system, cmd, { stdin = source }, vim.schedule_wrap(on_exit))
  if not ok then
    show_text(buf, tostring(job))
    return
  end
  api.nvim_create_autocmd("BufUnload", {
    buffer = buf,
    once = true,
    callback = function()
      if not job:is_closing() then
        job:kill(15)
      end
    end,
  })
end

return M
