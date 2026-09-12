--- 为 mini.files 追加文件的 Git XY 状态和目录的递归汇总；查询结果按真实仓库路径共享。
local M = {}
local mini_files = require("mini.files")
local git_status = require("snacks.picker.source.git")
local namespace = vim.api.nvim_create_namespace("mini-files-git-status")

---@class MiniFilesGitDirectory
---@field symbols table<string, boolean> 自身及子孙条目的状态集合，用于目录标记。
---@field status string 按 Snacks explorer 规则合并的 XY 状态，保留暂存与冲突信息用于名称配色。

---@class MiniFilesGitRepository
---@field root string 真实工作树根目录，同时支持 .git 文件形式的 worktree。
---@field files table<string, string> 绝对路径到 porcelain v1 的两列状态。
---@field directories table<string, MiniFilesGitDirectory> 路径到自身及子孙条目的状态汇总。
---@field scheduled boolean 是否已有待执行的刷新。
---@field running boolean 是否正在查询 Git。
---@field dirty boolean 查询期间是否又发生了刷新请求。
---@field failed boolean 上次查询是否失败，用于合并连续失败通知。

---@type table<string, MiniFilesGitRepository>
local repositories = {}
---@type table<integer, false|{ directory: string, repository: MiniFilesGitRepository }>
local buffers = {}

local status_order = { "!", "M", "A", "D", "R", "C", "T", "?" }
local status_highlights = {
  ["!"] = "DiagnosticError",
  M = "DiagnosticWarn",
  A = "DiagnosticOk",
  D = "DiagnosticError",
  R = "DiagnosticInfo",
  C = "DiagnosticInfo",
  T = "DiagnosticWarn",
  ["?"] = "DiagnosticOk",
}

local function is_conflict(status)
  return status:find("U", 1, true) ~= nil or status == "AA" or status == "DD"
end

-- 与 Snacks explorer 的默认名称配色一致，色值由当前主题提供。
local filename_highlights = {
  added = "Added",
  modified = "DiagnosticWarn",
  deleted = "Removed",
  renamed = "Special",
  copied = "Special",
  untracked = "NonText",
  ignored = "NonText",
}

---@param xy string
---@return string
local function filename_highlight(xy)
  local status = git_status.git_status(xy)
  if status.unmerged then
    return "DiagnosticError"
  end
  if status.staged then
    return "DiagnosticHint"
  end
  return filename_highlights[status.status]
end

--- NUL 分隔保留空格、换行等合法文件名；重命名/复制记录的第二个路径是来源。
---@param root string
---@param output string
---@return table<string, string> files
---@return table<string, MiniFilesGitDirectory> directories
local function parse_status(root, output)
  local files, directories = {}, {}
  local records = vim.split(output, "\0", { plain = true, trimempty = true })
  local index = 1
  while index <= #records do
    local record = records[index]
    local status = record:sub(1, 2)
    local path = vim.fs.joinpath(root, (record:sub(4):gsub("/$", "")))
    files[path] = status

    local function summarize(ancestor, xy)
      local summary = directories[ancestor] or { symbols = {}, status = xy }
      summary.status = git_status.merge_status(summary.status, xy)
      local symbols = is_conflict(xy) and "!" or xy:gsub(" ", "")
      for symbol in symbols:gmatch(".") do
        summary.symbols[symbol] = true
      end
      directories[ancestor] = summary
    end

    -- 也保留条目自身的汇总，供子模块和未跟踪的整目录显示。
    summarize(path, status)
    for parent in vim.fs.parents(path) do
      summarize(parent, status)
      if parent == root then
        break
      end
    end

    if status:find("[RC]") then
      -- 跨目录重命名同时标记来源目录；复制不改变来源。
      if status:find("R", 1, true) then
        local source = vim.fs.joinpath(root, records[index + 1])
        for parent in vim.fs.parents(source) do
          summarize(parent, (status:gsub("[^R]", " ")))
          if parent == root then
            break
          end
        end
      end
      index = index + 1
    end
    index = index + 1
  end
  return files, directories
end

---@param buf_id integer
local function render(buf_id)
  local buffer = buffers[buf_id]
  if not buffer or not vim.api.nvim_buf_is_valid(buf_id) then
    return
  end
  vim.api.nvim_buf_clear_namespace(buf_id, namespace, 0, -1)

  for line, text in ipairs(vim.api.nvim_buf_get_lines(buf_id, 0, -1, false)) do
    local entry = mini_files.get_fs_entry(buf_id, line)
    if entry then
      -- 只解析父目录的真实路径：链接条目仍按链接本身查询 Git。
      local path = vim.fs.joinpath(buffer.directory, entry.name)
      local stat = vim.uv.fs_lstat(path)
      local chunks = {}
      local status = buffer.repository.files[path]
      if stat and stat.type == "directory" then
        local summary = buffer.repository.directories[path]
        status = summary and summary.status
        if summary then
          for _, symbol in ipairs(status_order) do
            if summary.symbols[symbol] then
              chunks[#chunks + 1] = { " " .. symbol, status_highlights[symbol] }
            end
          end
        end
      elseif status then
        chunks = { { " [", "Comment" } }
        for symbol in status:gmatch(".") do
          local highlight = is_conflict(status) and "DiagnosticError" or status_highlights[symbol] or "Comment"
          chunks[#chunks + 1] = { symbol, highlight }
        end
        chunks[#chunks + 1] = { "]", "Comment" }
      end
      -- mini.files 行格式为 /路径编号/图标/名称；只覆盖名称的默认高亮。
      local name_start = text:match("^/%d+/[^/]*/()")
      if status and name_start then
        vim.api.nvim_buf_set_extmark(buf_id, namespace, line - 1, name_start - 1, {
          end_col = #text,
          hl_group = filename_highlight(status),
          priority = 4097, -- 高于 mini.files 名称 extmark 的默认优先级 4096。
          right_gravity = false,
        })
      end
      if #chunks > 0 then
        vim.api.nvim_buf_set_extmark(buf_id, namespace, line - 1, #text, {
          virt_text = chunks,
          -- inline 放在文件名后、现有 eol 软链接目标前，不参与文件名编辑和同步。
          virt_text_pos = "inline",
        })
      end
    end
  end
end

--- 合并一次目录刷新中的多次事件；查询中发生变化时丢弃旧结果并重新读取。
---@param repository MiniFilesGitRepository
local function request_refresh(repository)
  repository.dirty = true
  if repository.scheduled or repository.running then
    return
  end
  repository.scheduled = true
  vim.defer_fn(function()
    repository.scheduled = false
    if repositories[repository.root] ~= repository then
      return
    end
    repository.running, repository.dirty = true, false
    local complete = vim.schedule_wrap(function(result)
      repository.running = false
      if repositories[repository.root] ~= repository then
        return
      end
      if repository.dirty then
        request_refresh(repository)
        return
      end
      if result.code == 0 then
        repository.files, repository.directories = parse_status(repository.root, result.stdout)
        repository.failed = false
      else
        repository.files, repository.directories = {}, {}
        if not repository.failed then
          vim.notify("mini.files: 无法读取 Git 状态：" .. repository.root, vim.log.levels.WARN)
        end
        repository.failed = true
      end
      for buf_id, buffer in pairs(buffers) do
        if buffer and buffer.repository == repository then
          render(buf_id)
        end
      end
    end)
    local ok = pcall(vim.system, {
      "git",
      "--no-optional-locks",
      "status",
      "--porcelain=v1",
      "-z",
      "--untracked-files=all",
      "--ignore-submodules=none",
    }, { cwd = repository.root }, complete)
    if not ok then
      complete({ code = -1 })
    end
  end, 50)
end

---@param buf_id integer
local function forget_buffer(buf_id)
  local buffer = buffers[buf_id]
  buffers[buf_id] = nil
  if not buffer then
    return
  end
  for _, other in pairs(buffers) do
    if other and other.repository == buffer.repository then
      return
    end
  end
  repositories[buffer.repository.root] = nil
end

local function refresh_all()
  for _, repository in pairs(repositories) do
    request_refresh(repository)
  end
end

--- 在 mini.files.setup 后调用；关闭最后一个目录缓冲区时释放对应仓库缓存。
function M.setup()
  local group = vim.api.nvim_create_augroup("MiniFilesGitStatus", { clear = true })
  vim.api.nvim_create_autocmd("User", {
    group = group,
    pattern = "MiniFilesWindowUpdate",
    callback = function(args)
      local buf_id = args.data.buf_id
      if buffers[buf_id] ~= nil then
        return
      end
      -- WindowUpdate 时公开的窗口路径已就绪；文件预览的正文不能用来识别目录。
      buffers[buf_id] = false
      local state = assert(mini_files.get_explorer_state(), "MiniFilesWindowUpdate requires an active explorer")
      local directory
      for _, window in ipairs(state.windows) do
        if window.win_id == args.data.win_id then
          local stat = vim.uv.fs_stat(window.path)
          if stat and stat.type == "directory" then
            directory = vim.uv.fs_realpath(window.path)
          end
          break
        end
      end
      local root = directory and vim.fs.root(directory, ".git")
      if not root then
        return
      end
      local repository = repositories[root]
      local needs_refresh = repository == nil
      if not repository then
        repository = {
          root = root,
          files = {},
          directories = {},
          scheduled = false,
          running = false,
          dirty = false,
          failed = false,
        }
        repositories[root] = repository
      end
      buffers[buf_id] = { directory = directory, repository = repository }
      render(buf_id)
      if needs_refresh then
        request_refresh(repository)
      end
    end,
  })
  vim.api.nvim_create_autocmd("User", {
    group = group,
    pattern = "MiniFilesBufferUpdate",
    callback = function(args)
      local buffer = buffers[args.data.buf_id]
      if buffer then
        render(args.data.buf_id)
        request_refresh(buffer.repository)
      else
        buffers[args.data.buf_id] = nil
      end
    end,
  })
  vim.api.nvim_create_autocmd("User", {
    group = group,
    pattern = { "MiniFilesExplorerOpen", "MiniFilesAction*", "GitSignsUpdate" },
    callback = refresh_all,
  })
  vim.api.nvim_create_autocmd({ "BufWritePost", "FocusGained", "TermLeave", "TermClose" }, {
    group = group,
    callback = refresh_all,
  })
  vim.api.nvim_create_autocmd("BufWipeout", {
    group = group,
    callback = function(args)
      forget_buffer(args.buf)
    end,
  })
end

return M
