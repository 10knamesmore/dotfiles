-- mini file 文件浏览/管理
-- <leader>e 打开于当前文件
--
local path_utils = require("utils.path")

--- 显示所有文件系统条目。
---@param _ { fs_type: string, name: string, path: string }?
---@return boolean
local filter_show = function(_)
  return true
end

--- 隐藏以 `.` 开头的文件。
---@param fs_entry { fs_type: string, name: string, path: string }
---@return boolean
local filter_hide = function(fs_entry)
  return not vim.startswith(fs_entry.name, ".")
end

local root_augroup = vim.api.nvim_create_augroup("MiniFilesTabRoot", { clear = false })
local transient_mini_files_tabs = {}

--- 在新标签页中打开 mini.files，并为后续进入的文件设置 tab 根目录。
--- @param tab_id integer
local set_tab_root_on_file_enter = function(tab_id)
  local autocmd_id

  --- 进入真实文件后更新标签页根目录，并取消临时 mini.files 状态。
  autocmd_id = vim.api.nvim_create_autocmd("BufEnter", {
    group = root_augroup,
    callback = function(args)
      if vim.api.nvim_get_current_tabpage() ~= tab_id then
        return
      end

      local buf_id = args.buf
      local filetype = vim.bo[buf_id].filetype
      if vim.bo[buf_id].buftype ~= "" or filetype == "minifiles" or filetype == "minifiles-help" then
        return
      end

      local filepath = path_utils.bufpath(buf_id)
      local stat = filepath and vim.uv.fs_stat(filepath) or nil
      if not stat or stat.type ~= "file" then
        return
      end

      local roots = path_utils.detect({ all = false, buf = buf_id })
      local root = roots[1] and roots[1].paths[1] or vim.fs.dirname(filepath) or vim.uv.cwd()
      if root and root ~= "" then
        vim.cmd.tcd(vim.fn.fnameescape(root))
      end

      transient_mini_files_tabs[tab_id] = nil
      pcall(vim.api.nvim_del_autocmd, autocmd_id)
    end,
  })
end

--- 关闭尚未进入实际文件的临时 mini.files 标签页。
---@param tab_id integer
local close_transient_mini_files_tab = function(tab_id)
  if not transient_mini_files_tabs[tab_id] or not vim.api.nvim_tabpage_is_valid(tab_id) then
    transient_mini_files_tabs[tab_id] = nil
    return
  end

  transient_mini_files_tabs[tab_id] = nil
  vim.schedule(function()
    if vim.api.nvim_tabpage_is_valid(tab_id) and #vim.api.nvim_list_tabpages() > 1 then
      vim.cmd(vim.api.nvim_tabpage_get_number(tab_id) .. "tabclose")
    end
  end)
end

--- 在新标签页中以当前路径为锚点打开 mini.files。
local open_mini_files_in_new_tab = function()
  local current_path = vim.api.nvim_buf_get_name(0)
  local anchor = current_path ~= "" and current_path or vim.uv.cwd()

  vim.cmd.tabnew()
  local tab_id = vim.api.nvim_get_current_tabpage()
  transient_mini_files_tabs[tab_id] = true
  set_tab_root_on_file_enter(tab_id)
  require("mini.files").open(anchor, true)
end

local symlink_target_ns = vim.api.nvim_create_namespace("mini-files-symlink-target")

--- 将主目录前缀折叠为 `~`，便于缩短路径显示。
---@param path string
---@return string
local shorten_home_path = function(path)
  local home = vim.uv.os_homedir()
  if not home or home == "" then
    return path
  end

  return (path:gsub("^" .. vim.pesc(home), "~/"))
end

--- 为符号链接返回专用图标，其余条目沿用默认前缀。
---@param fs_entry { path: string }
---@return string, string
local prefix_with_symlink_icon = function(fs_entry)
  local stat = vim.uv.fs_lstat(fs_entry.path)
  if stat and stat.type == "link" then
    return "󰌷 ", "MiniIconsGreen"
  end

  return MiniFiles.default_prefix(fs_entry)
end

--- 在 mini.files 行尾渲染符号链接的目标路径。
---@param buf_id integer
local show_symlink_targets = function(buf_id)
  vim.api.nvim_buf_clear_namespace(buf_id, symlink_target_ns, 0, -1)

  for line = 1, vim.api.nvim_buf_line_count(buf_id) do
    local fs_entry = MiniFiles.get_fs_entry(buf_id, line)
    if fs_entry then
      ---@cast fs_entry { path: string }
      local stat = vim.uv.fs_lstat(fs_entry.path)
      if stat and stat.type == "link" then
        local target = vim.uv.fs_readlink(fs_entry.path)
        if target and target ~= "" then
          vim.api.nvim_buf_set_extmark(buf_id, symlink_target_ns, line - 1, 0, {
            virt_text = { { "-> " .. shorten_home_path(target), "Comment" } },
            virt_text_pos = "eol",
          })
        end
      end
    end
  end
end

--- 为 mini.files 创建分屏打开映射，并更新目标窗口。
---@param buf_id integer
---@param lhs string
---@param direction string
local map_split = function(buf_id, lhs, direction)
  --- 在新分屏中打开当前条目。
  local rhs = function()
    -- Make new window and set it as target
    local cur_target = MiniFiles.get_explorer_state().target_window
    local new_target = vim.api.nvim_win_call(cur_target, function()
      vim.cmd(direction .. " split")
      return vim.api.nvim_get_current_win()
    end)

    MiniFiles.set_target_window(new_target)

    require("mini.files").go_in()
    -- This intentionally doesn't act on file under cursor in favor of
    -- explicit "go in" action (`l` / `L`). To immediately open file,
    -- add appropriate `MiniFiles.go_in()` call instead of this comment.
  end

  -- Adding `desc` will result into `show_help` entries
  local desc = "Split " .. direction
  vim.keymap.set("n", lhs, rhs, { buffer = buf_id, desc = desc })
end

return {
  "nvim-mini/mini.files",
  keys = {
    {
      "<leader>e",
      function()
        require("mini.files").open(vim.api.nvim_buf_get_name(0), true)
      end,
      desc = "Open mini.files (current file)",
    },
    {
      "<leader>tn",
      open_mini_files_in_new_tab,
      desc = "New tab with mini.files",
    },
  },
  opts = {
    windows = {
      preview = true,
      width_preview = 80,
    },
    content = {
      filter = filter_hide,
      prefix = prefix_with_symlink_icon,
    },
    mappings = {
      trim_left = "H",
      trim_right = "L",
      reset = "<Esc>",
    },
    options = {
      permanent_delete = false,
    },
  },
  config = function(_, opts)
    require("mini.files").setup(opts)

    -- 默认显示
    local show_dotfiles = false

    --- 切换 mini.files 中隐藏文件的显示状态。
    local toggle_dotfiles = function()
      show_dotfiles = not show_dotfiles
      local new_filter = show_dotfiles and filter_show or filter_hide
      require("mini.files").refresh({ content = { filter = new_filter } })
    end

    --- 写入 `+` 寄存器并回读校验。
    --- notify 带 `mini.files:` 前缀以区分 yank-more 的全局 gy（消息几乎同文案）；
    --- 回读不一致时报 ERROR——用于排查「gy 偶发无效果」到底丢在映射层还是剪贴板层。
    ---@param label string
    ---@param text string
    local yank_to_clipboard = function(label, text)
      vim.fn.setreg("+", text)
      local got = vim.fn.getreg("+")
      if got ~= text then
        vim.notify(("mini.files: `+` 寄存器写入失败 (want=%q got=%q)"):format(text, got), vim.log.levels.ERROR)
        return
      end
      vim.notify(("mini.files: yanked %s: %s"):format(label, text), vim.log.levels.INFO)
    end

    --- 复制光标所在条目的文件名到系统剪贴板。
    local yank_filename = function()
      local filepath = (MiniFiles.get_fs_entry() or {}).path
      if filepath == nil then
        vim.notify("Cursor is not on valid entry", vim.log.levels.WARN)
        return
      end

      yank_to_clipboard("filename", vim.fs.basename(filepath))
    end

    --- 复制光标所在条目的绝对路径到系统剪贴板。
    local yank_filepath = function()
      local filepath = (MiniFiles.get_fs_entry() or {}).path
      if filepath == nil then
        vim.notify("Cursor is not on valid entry", vim.log.levels.WARN)
        return
      end

      yank_to_clipboard("path", filepath)
    end

    --- 复制 visual 选区内所有有效条目到系统剪贴板，每行一项，经 `transform` 提取目标文本。
    --- 在 visual keymap 中 `'<`/`'>` 尚未更新，故用 `line("v")`/`line(".")` 实时取选区两端。
    ---@param label string
    ---@param transform fun(path: string): string
    local yank_selection = function(label, transform)
      local buf_id = vim.api.nvim_get_current_buf()
      local line_start, line_end = vim.fn.line("v"), vim.fn.line(".")
      if line_start > line_end then
        line_start, line_end = line_end, line_start
      end

      -- 退出 visual 模式，避免选区高亮残留（行号已在上面读取，feedkeys 异步退出不影响）。
      vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<Esc>", true, false, true), "n", false)

      local items = {}
      for line = line_start, line_end do
        local fs_entry = MiniFiles.get_fs_entry(buf_id, line)
        if fs_entry then
          table.insert(items, transform(fs_entry.path))
        end
      end

      if #items == 0 then
        vim.notify("No valid entries in selection", vim.log.levels.WARN)
        return
      end

      yank_to_clipboard(("%d %s"):format(#items, label), table.concat(items, "\n"))
    end

    --- 为 mini.files 缓冲区补齐专属按键。
    --- `MiniFilesBufferCreate` 已保证目标就是 mini.files buffer，不依赖 filetype 已就绪。
    ---@param buf_id integer
    local set_mini_files_keymaps = function(buf_id)
      if not vim.api.nvim_buf_is_valid(buf_id) then
        return
      end

      vim.keymap.set("n", ".", toggle_dotfiles, { buffer = buf_id, desc = "Toggle hidden files" })
      vim.keymap.set("n", "gy", yank_filename, { buffer = buf_id, desc = "Yank Filename" })
      vim.keymap.set("n", "gY", yank_filepath, { buffer = buf_id, desc = "Yank Absolute Path" })
      vim.keymap.set("x", "gy", function()
        yank_selection("filenames", vim.fs.basename)
      end, { buffer = buf_id, desc = "Yank Filenames (selection)" })
      vim.keymap.set("x", "gY", function()
        yank_selection("paths", function(path)
          return path
        end)
      end, { buffer = buf_id, desc = "Yank Absolute Paths (selection)" })
      map_split(buf_id, "<C-s>", "belowright horizontal")
      map_split(buf_id, "<C-v>", "belowright vertical")
    end

    --- 在目录缓冲区创建或刷新后注册专属按键并渲染符号链接。
    --- 同时监听 BufferCreate 和 BufferUpdate，确保导航到已缓存的目录时按键仍可用。
    vim.api.nvim_create_autocmd("User", {
      pattern = { "MiniFilesBufferCreate", "MiniFilesBufferUpdate" },
      callback = function(args)
        local buf_id = args.data.buf_id
        set_mini_files_keymaps(buf_id)
        show_symlink_targets(buf_id)
      end,
    })

    --- 在 mini.files 重命名后同步 snacks 的文件重命名处理。
    vim.api.nvim_create_autocmd("User", {
      pattern = "MiniFilesActionRename",
      callback = function(event)
        require("snacks").rename.on_rename_file(event.data.from, event.data.to)
      end,
    })

    --- 关闭未进入实际文件的临时 mini.files 标签页。
    vim.api.nvim_create_autocmd("User", {
      pattern = "MiniFilesExplorerClose",
      callback = function()
        close_transient_mini_files_tab(vim.api.nvim_get_current_tabpage())
      end,
    })

    --- 在 mini.files 窗口打开时应用浮窗样式。
    vim.api.nvim_create_autocmd("User", {
      pattern = "MiniFilesWindowOpen",
      callback = function(args)
        local win_id = args.data.win_id

        -- Customize window-local settings
        vim.wo[win_id].winblend = 10
        local config = vim.api.nvim_win_get_config(win_id)
        config.border, config.title_pos = "rounded", "right"
        vim.api.nvim_win_set_config(win_id, config)
      end,
    })

    --- 在窗口边框右下角常驻显示 dotfiles 过滤状态（`.` 切换）。
    --- 区分「目录里没有隐藏文件」和「隐藏文件被过滤了」两种情况。
    vim.api.nvim_create_autocmd("User", {
      pattern = "MiniFilesWindowUpdate",
      callback = function(args)
        local win_id = args.data.win_id
        local config = vim.api.nvim_win_get_config(win_id)
        config.footer = show_dotfiles and { { " 󰈈 dot", "MiniFilesTitleFocused" } }
          or { { " 󰈉 dot", "Comment" } }
        config.footer_pos = "right"
        vim.api.nvim_win_set_config(win_id, config)
      end,
    })
  end,
}
