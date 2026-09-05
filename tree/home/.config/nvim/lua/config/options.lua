-- 手动格式化
vim.g.autoformat = false

local opt = vim.opt

-- 运行 next等 自动写入
opt.autowrite = true

-- 允许 .nvim.lua 提供本地配置。Neovim 从 cwd 向上搜索，
-- 不是只看项目根目录——这个仓库嵌套很深（~/dotfiles/tree/home/.config/nvim），
-- 上层任何一个 .nvim.lua 都会被拉进来。想截断就在外层写 vim.o.exrc = false。
-- 首次遇到未信任文件时先 (v)iew，再执行 :trust。
opt.exrc = true

-- 在SSH下用OSC 等插件处理剪切板.
opt.clipboard = vim.env.SSH_CONNECTION and "" or "unnamedplus" -- Sync with system clipboard

--- **menu**: 使用弹出菜单显示补全项
--- **menuone**: 即使只有一个匹配项也显示菜单
--- **noselect**: 不自动选择第一个补全项
--- **popup**: 显示 completionItem/resolve 的文档预览；
---            整体赋值时容易连它一起丢掉
opt.completeopt = "menu,menuone,noselect,popup"

-- 理论上应该被markdown插件接管, 不知道为什么要有
--- **0**: 正常显示所有文本
--- **1**: 用一个字符替代隐藏文本
--- **2**: 完全隐藏文本（可能显示替代字符）
--- **3**: 完全隐藏文本
opt.conceallevel = 2

--- **功能**: 退出未保存的缓冲区时显示确认对话框。
opt.confirm = true

--- **功能**: 高亮显示光标所在行。
opt.cursorline = true -- Enable highlighting of the current line

-- Use spaces instead of tabs
opt.expandtab = true

opt.fillchars = {
  foldopen = "",
  foldclose = "",
  fold = " ",
  foldsep = " ",
  diff = "╱",
  eob = " ",
}

opt.listchars = {
  tab = "<->",
  trail = "-", -- trailing space
  nbsp = "␣",
  -- 开了wrap, 下面应当不需要
  extends = "⟩",
  precedes = "⟨",
}

opt.foldcolumn = "0"

-- 小于这个level的被折叠
opt.foldlevel = 99

-- 默认 manual，让 LSP (vim.lsp.foldexpr) / treesitter / nvim-ufo 按需接管
opt.foldmethod = "manual"

opt.foldtext = ""
opt.formatexpr = "v:lua.utils.format.formatexpr()"
opt.formatoptions = "jcroqlnt" -- tcqj
-- :grep 包含隐藏文件并尊重 .gitignore，排除 Git 内部数据。
opt.grepprg = "rg --vimgrep --hidden --glob '!.git'"

-- Ignore case
opt.ignorecase = true

-- subsiture时 在同一个窗口里预览
opt.inccommand = "nosplit"

-- view 在跳转时恢复视图；clean 把已卸载 buffer 从 jumplist 移除。
-- H/L 绑定为 <C-o>/<C-i>，
-- 跳回一堆已关 buffer 的场景很常见。
opt.jumpoptions = "view,clean"

--每个窗口都有一个状态栏
opt.laststatus = 2

-- 尽量在单词边界换行
opt.linebreak = true -- Wrap lines at convenient points

-- 参考listchars
opt.list = true -- Show some invisible characters (tabs...,

-- 在所有模式下启用鼠标
opt.mouse = "a"

-- Print line number
opt.number = true

-- Popup blend
opt.pumblend = 50

-- Maximum number of entries in a popup
opt.pumheight = 10

-- Relative line numbers
opt.relativenumber = true

-- Disable the default ruler, lualine shows the cursor position
opt.ruler = false

-- 屏幕边缘至少多少行
opt.scrolloff = 4

-- 缩进舍入到 `shiftwidth` 的倍数。
opt.shiftround = true

-- Size of an indent
opt.shiftwidth = 4

--- **l**：用“999L, 888B”替代“999 lines, 888 bytes”。
--- **m**：用“[+]”替代“[Modified]”。
--- **r**：用“[RO]”替代“[readonly]”。
--- **w**：用“[w]”替代“written”，用“[a]”替代“appended”。
--- **a**：启用上述所有缩写（等价于 lmrw）。
--- **o**：写入文件的消息会被后续读入文件的消息覆盖（常用于 `:wn` 或 `autowrite`）。
--- **O**：读入文件的消息会覆盖之前的消息，也适用于 quickfix。
--- **s**：不显示“search hit BOTTOM, continuing at TOP”等搜索提示。
--- **t**：文件消息过长时，开头被截断，用“<”表示。
--- **T**：其他消息过长时，中间被“...”截断。
--- **W**：写入文件时不显示“written”或“[w]”。
--- **A**：不显示“ATTENTION”警告（如发现 swap 文件）。
--- **I**：启动时不显示欢迎信息。
--- **c**：不显示插入补全菜单相关提示。
--- **C**：扫描补全项时不显示消息。
--- **q**：录制宏时不显示“recording @a”。
--- **F**：编辑文件时不显示文件信息（如用 `:silent`）。
--- **S**：搜索时不显示搜索计数（如“[1/5]”）。
-- 追加而不是覆盖 shortmess，保留 t/T 长消息截断与 o/O 读写消息覆盖；否则容易触发
-- hit-enter 提示。noice 接管了大部分消息路径，但它覆盖不到的早期路径仍会漏。
opt.shortmess:append({ I = true, c = true, C = true })

-- Dont show mode since we have a statusline
opt.showmode = false

-- 左右两边至少留多少列
-- 因为开了wrap所以没必要
-- opt.sidescrolloff = 8
--
-- Always show the signcolumn
opt.signcolumn = "yes"

-- Don't ignore case with capitals
opt.smartcase = true

-- Insert indents automatically
opt.smartindent = true

-- Scrolling works with screen lines
opt.smoothscroll = true

-- 插入删除时
opt.softtabstop = 4

opt.spelllang = { "en" }

-- Put new windows below current
opt.splitbelow = true

-- split后 屏幕显示内容不变
opt.splitkeep = "screen"
-- Put new windows right of current
opt.splitright = true

-- 字符tab显示宽度
opt.tabstop = 4 -- Number of spaces tabs count for

-- True color support
opt.termguicolors = true

-- 500ms 允许 g→Shift+Y 等换挡序列完成；which-key 弹窗时机由 opts.delay 单独控制。
opt.timeoutlen = vim.g.vscode and 1000 or 500

-- undo
opt.undofile = true
opt.undolevels = 10000

-- Save swap file and trigger CursorHold
opt.updatetime = 350

-- 只在visual block模式下允许vituraledit
opt.virtualedit = "block"

-- Completion mode that is used for the character specified with
opt.wildmode = "longest:full,full"

-- Minimum window width
opt.winminwidth = 5

-- 为所有未显式指定 border 的浮窗提供统一圆角边框。
opt.winborder = "rounded"

-- 一行超出范围换行
opt.wrap = true

--
-- LazyVim root dir detection
-- Each entry can be:
-- * the name of a detector function like `lsp` or `cwd`
-- * a pattern or array of patterns like `.git` or `lua`.
-- * a function with signature `function(buf) -> string|string[]`
vim.g.root_spec = { "lsp", { ".git", "lua" }, "cwd" }

-- Set LSP servers to be ignored when used with `util.root.detectors.lsp`
-- for detecting the LSP root
vim.g.root_lsp_ignore = { "copilot" }
