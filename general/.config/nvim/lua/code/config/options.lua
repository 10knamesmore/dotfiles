-- ============================================================
-- VSCode Neovim Options Configuration
-- ============================================================
local opt = vim.opt

-- ========== 剪贴板 ==========
opt.clipboard = "unnamedplus"

-- ========== 搜索设置 (smartcase) ==========
-- Ignore case in search patterns
opt.ignorecase = true
-- Don't ignore case with capitals (智能大小写)
opt.smartcase = true
-- Show substitution preview in split window
opt.inccommand = "nosplit"

-- ========== 缩进和格式 ==========
opt.expandtab = true         -- Use spaces instead of tabs
opt.shiftwidth = 4           -- Size of an indent
opt.tabstop = 4              -- Number of spaces tabs count for
opt.softtabstop = 4          -- Insert indents
opt.shiftround = true        -- Round indent to multiple of shiftwidth
opt.smartindent = true       -- Insert indents automatically

-- ========== 视图和显示 ==========
opt.number = true            -- Print line number
opt.relativenumber = true    -- Relative line numbers
opt.cursorline = true        -- Highlight current line
opt.scrolloff = 4            -- Lines of context
opt.wrap = true              -- Enable line wrap
opt.linebreak = true         -- Wrap at word boundaries
opt.smoothscroll = true      -- Scrolling works with screen lines
opt.signcolumn = "yes"       -- Always show sign column

-- ========== 折叠设置 ==========
opt.foldlevel = 99           -- Don't fold by default
opt.foldmethod = "indent"    -- Fold based on indent

-- ========== 编辑行为 ==========
opt.mouse = "a"              -- Enable mouse in all modes
opt.virtualedit = "block"    -- Allow cursor beyond EOL in visual block mode
opt.confirm = true           -- Confirm to save changes before exiting
opt.undofile = true          -- Enable persistent undo
opt.undolevels = 10000       -- Maximum undo history

-- ========== 补全设置 ==========
opt.completeopt = "menu,menuone,noselect"
opt.pumheight = 10           -- Maximum number of popup menu items

-- ========== 窗口分割 ==========
opt.splitbelow = true        -- Put new windows below current
opt.splitright = true        -- Put new windows right of current
opt.splitkeep = "screen"     -- Keep screen content when splitting

-- ========== 时间设置 ==========
vim.o.updatetime = 300       -- Faster completion (VSCode default: 300)
opt.timeoutlen = 1000        -- Key sequence timeout (VSCode mode: longer for stability)

-- ========== 其他 ==========
opt.autowrite = true         -- Auto write when running commands
opt.jumpoptions = "view"     -- Restore view when jumping
opt.showmode = false         -- Don't show mode (VSCode shows it)

