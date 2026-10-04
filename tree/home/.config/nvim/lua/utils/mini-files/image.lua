--- 在 mini.files 的文件预览窗口中显示图片；解码、缩放和终端协议交给 Snacks。
local M = {}
local mini_files = require("mini.files")
local image = require("snacks").image

--- 每个预览窗口只保留当前图片，避免缓存的文件缓冲区继续占用图片资源。
---@type table<integer, { placement: snacks.image.Placement, winblend: integer }?>
local previews = {}

---@param win_id integer
local function close_preview(win_id)
  local preview = previews[win_id]
  if not preview then
    return
  end
  preview.placement:close()
  if vim.api.nvim_win_is_valid(win_id) then
    vim.wo[win_id].winblend = preview.winblend
  end
  previews[win_id] = nil
end

---@param buf_id integer
---@param win_id integer
local function update_preview(buf_id, win_id)
  local preview = previews[win_id]
  if preview and preview.placement.buf ~= buf_id then
    close_preview(win_id)
    preview = nil
  end

  if not preview then
    local state = assert(mini_files.get_explorer_state(), "MiniFilesWindowUpdate requires an active explorer")
    local path
    for _, window in ipairs(state.windows) do
      if window.win_id == win_id then
        path = window.path
        break
      end
    end
    -- 空目录或尚未同步的条目会产生以 NUL 结尾的虚拟路径，不能传给 Vimscript 路径函数。
    if not path or path:sub(-1) == "\0" or not image.supports_file(path) then
      return
    end
    local stat = vim.uv.fs_stat(path)
    if not stat or stat.type ~= "file" or not image.supports_terminal() then
      return
    end

    -- 保留 minifiles filetype 和可写状态，供 mini.files 跟踪焦点及刷新正文。
    vim.api.nvim_buf_set_lines(buf_id, 0, -1, false, { "" })
    preview = {
      -- 使用内联渲染，避免独立 viewer 的加载动画在已隐藏的缓存缓冲区继续运行。
      placement = image.placement.new(buf_id, path, { inline = true, conceal = true, auto_resize = true }),
      winblend = vim.wo[win_id].winblend,
    }
    previews[win_id] = preview
  end

  -- mini.files 按正文行数计算高度；图片的虚拟行不计入，需在每次刷新后展开窗口。
  local config = vim.api.nvim_win_get_config(win_id)
  local statusline_height = vim.o.laststatus > 0 and 1 or 0
  local height = vim.o.lines - config.row - vim.o.cmdheight - statusline_height - 2
  vim.api.nvim_win_set_height(win_id, math.max(1, height))
  vim.wo[win_id].winblend = 0
  preview.placement:update()
end

--- 在 mini.files.setup 后调用；切换文件、刷新正文或关闭窗口时释放旧图片。
function M.setup()
  local group = vim.api.nvim_create_augroup("MiniFilesImagePreview", { clear = true })
  vim.api.nvim_create_autocmd("User", {
    group = group,
    pattern = "MiniFilesWindowUpdate",
    callback = function(args)
      update_preview(args.data.buf_id, args.data.win_id)
    end,
  })
  vim.api.nvim_create_autocmd("User", {
    group = group,
    pattern = "MiniFilesBufferUpdate",
    callback = function(args)
      for win_id, preview in pairs(previews) do
        if preview.placement.buf == args.data.buf_id then
          close_preview(win_id)
        end
      end
    end,
  })
  vim.api.nvim_create_autocmd("WinClosed", {
    group = group,
    callback = function(args)
      close_preview(assert(tonumber(args.match)))
    end,
  })
end

return M
