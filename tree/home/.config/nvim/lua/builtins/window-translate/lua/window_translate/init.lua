--- 按源窗口管理翻译。只把完整请求写入当前 buffer 版本的缓存；
--- 流式结果属于当前请求，关闭显示、换 buffer 或编辑源文时失效。
local M = {}
local api = vim.api
local client = require("window_translate.client")
local view = require("window_translate.view")
local blocks = require("window_translate.blocks")

---@class WindowTranslate.Config: WindowTranslate.ClientConfig
---@field mode WindowTranslate.Mode 新窗口的初始显示方式。
---@field debounce_ms integer 新请求的合并等待时间；不延迟缓存显示或流式渲染。
---@field max_concurrent integer 每个源窗口同时进行的请求上限。
---@field chunks_per_request integer 单次请求最多翻译的块数，尾批按实际数量发送。
---@field prefetch WindowTranslate.Prefetch
---@field style WindowTranslate.Style 合并默认值后的完整样式。

--- setup 接收的覆盖项；省略的字段使用默认值，样式按字段合并，highlight 按组替换。
---@class WindowTranslate.Options
---@field mode? WindowTranslate.Mode 默认 inline。
---@field api? WindowTranslate.Api 默认 responses。
---@field endpoint? string 完整请求地址；切换 API 时同时修改地址、模型和密钥变量名。
---@field model? string 默认 gpt-6-luna。
---@field api_key_env? string 默认 OPENAI_API_KEY；Neovim 必须已继承该环境变量。
---@field fast? boolean 默认 true，仅控制 Responses 服务档位，不开启思考。
---@field target_language? string 默认简体中文。
---@field debounce_ms? integer 默认 50 毫秒；设为 0 立即请求。手动开启不等待。
---@field max_concurrent? integer 默认 4，当前可见块优先使用空闲额度。
---@field chunks_per_request? integer 默认 3；并发上限仍按请求数计。
---@field prefetch? WindowTranslate.PrefetchOptions
---@field timeout_seconds? integer 默认 60 秒。
---@field style? WindowTranslate.StyleOptions

--- 一屏按源窗口高度折算源行数；前后方向随实际滚动方向交换。
---@class WindowTranslate.Prefetch
---@field ahead integer 滚动方向前方预取屏数，默认 2。
---@field behind integer 反方向预取屏数，默认 1。

---@class WindowTranslate.PrefetchOptions
---@field ahead? integer 设为 0 关闭该方向预取。
---@field behind? integer 设为 0 关闭该方向预取。

---@type WindowTranslate.Config
local config = {
  mode = "inline",
  api = "responses",
  endpoint = "https://api.openai.com/v1/responses",
  model = "gpt-6-luna",
  api_key_env = "OPENAI_API_KEY",
  fast = true,
  target_language = "简体中文",
  debounce_ms = 50,
  max_concurrent = 4,
  chunks_per_request = 3,
  prefetch = { ahead = 2, behind = 1 },
  timeout_seconds = 60,
  style = {
    inline = {
      prefix = "  │ ",
      highlight = "WindowTranslate",
      padding_above = 0,
      padding_below = 0,
      reserve_lines = 1,
      wrap = true,
    },
    window = function()
      local width = math.floor(vim.o.columns * 0.45)
      return {
        relative = "editor",
        row = 2,
        col = vim.o.columns - width - 2,
        width = width,
        height = math.floor(vim.o.lines * 0.6),
        style = "minimal",
        border = "rounded",
        title = " 翻译 ",
        title_pos = "center",
      }
    end,
    split = function(source_window)
      return {
        split = "right",
        win = source_window,
        width = math.floor(api.nvim_win_get_width(source_window) / 2),
      }
    end,
    window_options = {
      wrap = true,
      linebreak = true,
      number = false,
      relativenumber = false,
      signcolumn = "no",
      foldcolumn = "0",
      winblend = 0,
      winhighlight = "Normal:WindowTranslateNormal,NormalFloat:WindowTranslateNormal,FloatBorder:WindowTranslateBorder",
    },
    progress = { text = "翻译中…", highlight = "WindowTranslateProgress" },
    highlights = {
      WindowTranslate = { link = "DiagnosticInfo" },
      WindowTranslateNormal = { link = "NormalFloat" },
      WindowTranslateBorder = { link = "FloatBorder" },
      WindowTranslateProgress = { link = "Comment" },
    },
  },
}

--- 一个请求持有一批块；本批数组位置对应模型的译文数组，对象身份用于拒绝过期回调。
---@class WindowTranslate.Request
---@field blocks WindowTranslate.Block[] 按源文顺序排列，发送后不再改变成员或顺序。
---@field process? vim.SystemObj 启动后赋值，异步回调前已就绪。

--- toggle 创建；换 buffer 或关闭源窗口时移除。关闭显示保留完整缓存。
---@class WindowTranslate.Window
---@field window integer 源 window ID，也是 windows 表的键。
---@field buffer integer 创建时的源 buffer ID；此窗口换 buffer 时整个状态失效。
---@field namespace integer 只在源 window 可见的 extmark namespace。
---@field mode WindowTranslate.Mode
---@field active? boolean nil/false 表示隐藏，不再接收新请求。
---@field changedtick? integer 当前缓存对应的源 buffer 版本；编辑后整体清空。
---@field blocks WindowTranslate.Block[] 当前 buffer 版本的块，完整缓存和流式文本均挂在对应块上。
---@field by_line table<integer, WindowTranslate.Block[]> 一个源行可包含多个块。
---@field requests table<integer, WindowTranslate.Request> 仅包含已发出的批次，以本批首块 ID 索引。
---@field scheduled? table 尚未发出的请求回调身份；新滚动事件替换，关闭时清空。
---@field viewport? WindowTranslate.Viewport
---@field direction integer 1 表示向下，-1 表示向上；初始向下。
---@field inline_marks table<integer, WindowTranslate.VirtualLine[]> view 缓存的虚拟行，以 1-based 源行号索引。
---@field layouts table<integer, WindowTranslate.Layout> 按块 ID 索引的排版缓存。
---@field progress_line? integer 进度提示当前所在的源行。
---@field output_window? integer view 创建的译文窗口；其 scratch buffer 随窗口关闭销毁。
---@field output_lines string[] 上次写入译文 buffer 的行，供局部更新比较。
---@field output_topline? integer 上次展示的源首行；只有源视口移动才将译文光标移回顶部。

---@type table<integer, WindowTranslate.Window>
local windows = {}
---@type fun(state: WindowTranslate.Window, immediate?: boolean)
local refresh
---@type fun(state: WindowTranslate.Window)
local fill_slots

--- 先解除请求归属，再终止 curl；退出回调仍会到达，但不再拥有这个窗口。
---@param state WindowTranslate.Window
local function cancel(state)
  local requests = state.requests
  state.requests = {}
  state.scheduled = nil
  for _, request in pairs(requests) do
    request.process:kill(15)
    for _, block in ipairs(request.blocks) do
      block.text = ""
    end
  end
end

--- 隐藏所有显示资源并丢弃未完成请求，保留同一 buffer 版本的完整缓存。
---@param state WindowTranslate.Window
local function disable(state)
  state.active = false
  state.scheduled = nil
  cancel(state)
  view.close(state)
end

---@class WindowTranslate.SourceLine
---@field line integer 原 buffer 的 1-based 行号。
---@field text string

--- 只用未被折叠隐藏的源行选择块；被选中的块始终使用完整文本。
---@param state WindowTranslate.Window
---@param first integer
---@param last integer
---@return WindowTranslate.SourceLine[]
local function source_lines(state, first, last)
  return api.nvim_win_call(state.window, function()
    local lines = {}
    local line = first
    while line <= last do
      local fold_start = vim.fn.foldclosed(line)
      if fold_start == -1 or fold_start == line then
        lines[#lines + 1] = { line = line, text = api.nvim_buf_get_lines(state.buffer, line - 1, line, false)[1] }
      end
      line = fold_start == -1 and line + 1 or vim.fn.foldclosedend(line) + 1
    end
    return lines
  end)
end

---@class WindowTranslate.Viewport
---@field first integer 屏幕首个源行。
---@field last integer 屏幕末个源行。
---@field height integer 源窗口高度，用于确定预取批次大小。
---@field lines WindowTranslate.SourceLine[] 范围不变时复用，不随每个流式分片重新读 buffer。
---@field blocks WindowTranslate.Block[] 与可见源行相交的完整块。
---@field block_ids table<integer, boolean> 可见块集合，供流式回调判断是否需要更新视图。

---@param state WindowTranslate.Window
---@param lines WindowTranslate.SourceLine[]
---@return WindowTranslate.Block[]
---@return table<integer, boolean>
local function selected_blocks(state, lines)
  local selected, seen = {}, {}
  for _, source in ipairs(lines) do
    for _, block in ipairs(state.by_line[source.line] or {}) do
      if not seen[block.id] then
        selected[#selected + 1] = block
        seen[block.id] = true
      end
    end
  end
  return selected, seen
end

---@param state WindowTranslate.Window
---@return WindowTranslate.Viewport
local function viewport(state)
  local first, last = unpack(api.nvim_win_call(state.window, function()
    return { vim.fn.line("w0"), vim.fn.line("w$") }
  end))
  local previous = state.viewport
  if previous and previous.first ~= first then
    state.direction = first > previous.first and 1 or -1
  end
  if not previous or previous.first ~= first or previous.last ~= last then
    local lines = source_lines(state, first, last)
    local selected, ids = selected_blocks(state, lines)
    state.viewport = {
      first = first,
      last = last,
      height = api.nvim_win_get_height(state.window),
      lines = lines,
      blocks = selected,
      block_ids = ids,
    }
  else
    previous.height = api.nvim_win_get_height(state.window)
  end
  return state.viewport
end

--- 每次补充并发额度时按最新视口重建候选顺序，不保留已经滚出范围的等待任务。
---@param state WindowTranslate.Window
---@param frame WindowTranslate.Viewport
---@return WindowTranslate.Block[]
local function candidates(state, frame)
  local result = vim.list_extend({}, frame.blocks)
  local total = api.nvim_buf_line_count(state.buffer)
  for _, side in ipairs({
    { direction = state.direction, screens = config.prefetch.ahead },
    { direction = -state.direction, screens = config.prefetch.behind },
  }) do
    local first = side.direction == 1 and frame.last + 1 or math.max(1, frame.first - side.screens * frame.height)
    local last = side.direction == 1 and math.min(total, frame.last + side.screens * frame.height) or frame.first - 1
    if side.screens > 0 and first <= last then
      local nearby = selected_blocks(state, source_lines(state, first, last))
      if side.direction == -1 then
        for index = #nearby, 1, -1 do
          result[#result + 1] = nearby[index]
        end
      else
        vim.list_extend(result, nearby)
      end
    end
  end
  return result
end

--- 已发请求不因滚动或切换 tab 而取消；只有当前仍可见的块才更新布局。
---@param state WindowTranslate.Window
---@param changed WindowTranslate.Block[]
local function render_blocks(state, changed)
  if api.nvim_win_get_tabpage(state.window) == api.nvim_get_current_tabpage() then
    local frame = viewport(state)
    for _, block in ipairs(changed) do
      if frame.block_ids[block.id] then
        view.render(state, frame, config.style)
        break
      end
    end
  end
end

---@param state WindowTranslate.Window
---@param request WindowTranslate.Request
---@return boolean
local function owns_request(state, request)
  if state.requests[request.blocks[1].id] ~= request then
    return false
  end
  if api.nvim_buf_get_changedtick(state.buffer) ~= state.changedtick then
    refresh(state)
    return false
  end
  return true
end

---@param state WindowTranslate.Window
---@param batch WindowTranslate.Block[]
local function start_request(state, batch)
  table.sort(batch, function(left, right)
    return left.id < right.id
  end)
  ---@type WindowTranslate.Request
  local request = { blocks = batch }
  state.requests[batch[1].id] = request
  ---@type string[]
  local chunks = {}
  for _, block in ipairs(batch) do
    chunks[#chunks + 1] = block.source
  end
  local path = api.nvim_buf_get_name(state.buffer)
  request.process = client.translate(config, {
    file = {
      name = vim.fn.fnamemodify(path, ":t"),
      path = path,
      filetype = vim.bo[state.buffer].filetype,
    },
    chunks = chunks,
  }, {
    update = function(changes)
      if owns_request(state, request) then
        local changed = {}
        for _, update in ipairs(changes) do
          local block = batch[update.index]
          block.text = update.text
          changed[#changed + 1] = block
        end
        render_blocks(state, changed)
      end
    end,
    done = function(translations, err)
      if not owns_request(state, request) then
        return
      end
      state.requests[batch[1].id] = nil
      for index, block in ipairs(batch) do
        if err then
          block.text, block.error = "", err
        else
          local text = translations[index]
          block.text = text == block.source and "" or text
          block.complete = true
        end
      end
      if err then
        vim.notify("窗口翻译：" .. err, vim.log.levels.ERROR)
      end
      render_blocks(state, batch)
      fill_slots(state)
    end,
  })
end

--- 可见块优先组批；已发批次与本轮已选块都不重复入队。并发额度按请求计，不抢占在途批次。
fill_slots = function(state)
  if
    not state.active
    or state.scheduled
    or api.nvim_win_get_tabpage(state.window) ~= api.nvim_get_current_tabpage()
  then
    return
  end
  if vim.tbl_count(state.requests) >= config.max_concurrent then
    return
  end
  local pending = {}
  for _, request in pairs(state.requests) do
    for _, block in ipairs(request.blocks) do
      pending[block.id] = true
    end
  end
  local batch = {}
  for _, block in ipairs(candidates(state, viewport(state))) do
    if not block.complete and not block.error and not pending[block.id] then
      pending[block.id] = true
      batch[#batch + 1] = block
      if #batch == config.chunks_per_request then
        start_request(state, batch)
        batch = {}
        if vim.tbl_count(state.requests) >= config.max_concurrent then
          return
        end
      end
    end
  end
  if #batch > 0 then
    start_request(state, batch)
  end
end

--- 显示立即刷新；50ms 合并只影响尚未发出的请求。编辑源文使整组块和旧请求失效。
---@param state WindowTranslate.Window
---@param immediate? boolean 绕过新请求的合并等待，用于手动开启和定时回调。
refresh = function(state, immediate)
  if not state.active then
    return
  end
  local tick = api.nvim_buf_get_changedtick(state.buffer)
  if tick ~= state.changedtick then
    cancel(state)
    state.layouts, state.viewport = {}, nil
    view.clear_inline(state)
    local ok, parsed, index = pcall(blocks.read, state.buffer)
    if not ok then
      disable(state)
      vim.notify("窗口翻译：" .. tostring(parsed), vim.log.levels.ERROR)
      return
    end
    state.blocks, state.by_line, state.changedtick = parsed, index, tick
  end
  if api.nvim_win_get_tabpage(state.window) ~= api.nvim_get_current_tabpage() then
    return
  end
  view.open(state, config.style)
  local frame = viewport(state)
  view.render(state, frame, config.style)
  if not immediate and config.debounce_ms > 0 then
    local pending = {}
    state.scheduled = pending
    vim.defer_fn(function()
      if state.active and state.scheduled == pending then
        state.scheduled = nil
        refresh(state, true)
      end
    end, config.debounce_ms)
    return
  end
  state.scheduled = nil
  fill_slots(state)
end

--- 在译文窗口调用命令时，返回它对应的源窗口状态。
---@return WindowTranslate.Window?
local function current_state()
  local current = api.nvim_get_current_win()
  for _, state in pairs(windows) do
    if state.output_window == current then
      return state
    end
  end
  return windows[current]
end

--- 开关当前源窗口的译文；也可从对应译文窗口调用。
--- 开启时立即分配显示区域。密钥缺失时提示错误，不创建翻译状态。
function M.toggle()
  local state = current_state()
  if state and state.active then
    disable(state)
    return
  end
  if not vim.env[config.api_key_env] or vim.env[config.api_key_env] == "" then
    vim.notify("窗口翻译：请设置 " .. config.api_key_env, vim.log.levels.ERROR)
    return
  end
  if not state then
    local window = api.nvim_get_current_win()
    state = {
      window = window,
      buffer = api.nvim_get_current_buf(),
      namespace = api.nvim_create_namespace("window_translate." .. window),
      mode = config.mode,
      blocks = {},
      by_line = {},
      requests = {},
      direction = 1,
      inline_marks = {},
      layouts = {},
      output_lines = {},
    }
    -- 同一个 buffer 在另一个窗口打开时，不显示这里的虚拟行。
    api.nvim__ns_set(state.namespace, { wins = { window } })
    windows[window] = state
  end
  for _, block in ipairs(state.blocks) do
    block.error = nil
  end
  state.active = true
  refresh(state, true)
end

--- 重建当前译文视图，复用缓存和进行中的请求；同时设置新窗口的默认模式。
---@param mode WindowTranslate.Mode
function M.set_mode(mode)
  assert(vim.tbl_contains({ "inline", "window", "split" }, mode), "模式应为 inline、window 或 split")
  config.mode = mode
  local state = current_state()
  if state then
    local active = state.active
    state.active = false
    view.close(state)
    state.mode = mode
    state.active = active
    if state.active then
      refresh(state)
    end
  end
end

--- 合并配置并注册命令、窗口生命周期和配色回调，由 lazy.nvim 在首次加载时调用。
--- 解析失败关闭显示；请求失败保留其他块，不自动重试或切换服务。
---@param options? WindowTranslate.Options
function M.setup(options)
  options = options or {}
  config = vim.tbl_deep_extend("force", config, options)
  assert(vim.tbl_contains({ "responses", "deepseek" }, config.api), "api 应为 responses 或 deepseek")
  assert(config.max_concurrent >= 1 and config.max_concurrent % 1 == 0, "max_concurrent 应为正整数")
  assert(config.chunks_per_request >= 1 and config.chunks_per_request % 1 == 0, "chunks_per_request 应为正整数")
  -- highlight 定义整项替换，避免旧 link 覆盖新设置的 fg/bg。
  for name, attributes in pairs(vim.tbl_get(options, "style", "highlights") or {}) do
    config.style.highlights[name] = attributes
  end
  local function highlights()
    for name, attributes in pairs(config.style.highlights) do
      api.nvim_set_hl(0, name, attributes)
    end
  end
  highlights()
  api.nvim_create_user_command("TranslateToggle", M.toggle, {})
  api.nvim_create_user_command("TranslateMode", function(args)
    M.set_mode(args.args)
  end, {
    nargs = 1,
    complete = function()
      return { "inline", "window", "split" }
    end,
  })
  local group = api.nvim_create_augroup("WindowTranslate", { clear = true })
  api.nvim_create_autocmd("ColorScheme", { group = group, callback = highlights })
  api.nvim_create_autocmd("VimResized", {
    group = group,
    callback = function()
      for _, state in pairs(windows) do
        if state.active and api.nvim_win_get_tabpage(state.window) == api.nvim_get_current_tabpage() then
          view.resize(state, config.style)
          refresh(state)
        end
      end
    end,
  })
  api.nvim_create_autocmd("WinScrolled", {
    group = group,
    callback = function()
      -- v:event 只列出本次真正滚动/改变尺寸的窗口；译文窗口滚动不触发源窗口刷新。
      for window in pairs(vim.v.event) do
        local state = windows[tonumber(window)]
        if state then
          refresh(state)
        end
      end
    end,
  })
  api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    group = group,
    callback = function(args)
      for _, state in pairs(windows) do
        if state.buffer == args.buf then
          refresh(state)
        end
      end
    end,
  })
  api.nvim_create_autocmd("BufWinEnter", {
    group = group,
    callback = function(args)
      for window, state in pairs(windows) do
        if api.nvim_win_is_valid(window) and api.nvim_win_get_buf(window) == state.buffer then
          if args.buf == state.buffer then
            state.viewport = nil
            refresh(state)
          end
        else
          disable(state)
          windows[window] = nil
        end
      end
    end,
  })
  api.nvim_create_autocmd("WinEnter", {
    group = group,
    callback = function()
      local state = windows[api.nvim_get_current_win()]
      if state then
        refresh(state)
      end
    end,
  })
  api.nvim_create_autocmd({ "TabEnter", "TabLeave" }, {
    group = group,
    callback = function(args)
      for _, state in pairs(windows) do
        if api.nvim_win_get_tabpage(state.window) == api.nvim_get_current_tabpage() then
          if args.event == "TabLeave" then
            state.scheduled = nil
          else
            refresh(state)
          end
        end
      end
    end,
  })
  api.nvim_create_autocmd("WinClosed", {
    group = group,
    callback = function(args)
      local closed = tonumber(args.match)
      for window, state in pairs(windows) do
        if window == closed then
          disable(state)
          windows[window] = nil
        elseif state.output_window == closed then
          state.output_window = nil
          disable(state)
        end
      end
    end,
  })
  api.nvim_create_autocmd("BufWipeout", {
    group = group,
    callback = function(args)
      for window, state in pairs(windows) do
        if state.buffer == args.buf then
          disable(state)
          windows[window] = nil
        end
      end
    end,
  })
  api.nvim_create_autocmd("VimLeavePre", {
    group = group,
    callback = function()
      for _, state in pairs(windows) do
        cancel(state)
      end
    end,
  })
end

return M
