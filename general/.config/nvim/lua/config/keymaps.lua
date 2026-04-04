local map = vim.keymap.set
local function opts(desc)
    local opts = { noremap = true, silent = true }

    if desc then
        opts.desc = desc
    end
    return opts
end

map({ "i", "n", "s" }, "<esc>", function()
    vim.cmd("noh")
    vim.snippet.stop()
    return "<esc>"
end, { expr = true, desc = "Escape and Clear hlsearch" })

-- highlightgroup under cursor
map("n", "<leader>ui", vim.show_pos, { desc = "Inspect Pos" })

-- treesitter inspect tree
map("n", "<leader>uI", function()
    vim.treesitter.inspect_tree({
        command = (math.floor(vim.o.columns * 0.4)) .. "vnew",
    })
    vim.api.nvim_input("I")
end, { desc = "Inspect Tree" })

map("n", "<leader>uf", function()
    utils.format.toggle()
end, { desc = "Toggle Auto Format" })

map("n", "<leader>uh", function()
    local bufnr = vim.api.nvim_get_current_buf()
    local enabled = vim.lsp.inlay_hint.is_enabled({ bufnr = bufnr })
    local new_enabled = not enabled
    vim.lsp.inlay_hint.enable(new_enabled, { bufnr = bufnr })
    vim.api.nvim_echo({
        { "inlay_hint ", "Normal" },
        { new_enabled and "ENABLED" or "DISABLED", new_enabled and "DiagnosticOk" or "DiagnosticWarn" },
    }, false, {})
end, { desc = "Toggle Inlay Hints" })

-- better up/down
map({ "n", "x" }, "j", "v:count == 0 ? 'gj' : 'j'", { desc = "Down", expr = true, silent = true })
map({ "n", "x" }, "<Down>", "v:count == 0 ? 'gj' : 'j'", { desc = "Down", expr = true, silent = true })
map({ "n", "x" }, "k", "v:count == 0 ? 'gk' : 'k'", { desc = "Up", expr = true, silent = true })
map({ "n", "x" }, "<Up>", "v:count == 0 ? 'gk' : 'k'", { desc = "Up", expr = true, silent = true })

-- Move to window using the <ctrl> hjkl keys
map("n", "<C-h>", "<C-w>h", { desc = "Go to Left Window", remap = true })
map("n", "<C-j>", "<C-w>j", { desc = "Go to Lower Window", remap = true })
map("n", "<C-k>", "<C-w>k", { desc = "Go to Upper Window", remap = true })
map("n", "<C-l>", "<C-w>l", { desc = "Go to Right Window", remap = true })

map("n", "<leader>wd", "<C-W>c", { desc = "Delete Window", remap = true })

-- 重绘
map(
    "n",
    "<leader>ur",
    "<Cmd>nohlsearch<Bar>diffupdate<Bar>normal! <C-L><CR>",
    { desc = "Redraw / Clear hlsearch / Diff Update" }
)

-- n总是向前搜索，N总是向后搜索，并且展开折叠
map("n", "n", "'Nn'[v:searchforward].'zv'", { expr = true, desc = "Next Search Result" })
map("x", "n", "'Nn'[v:searchforward]", { expr = true, desc = "Next Search Result" })
map("o", "n", "'Nn'[v:searchforward]", { expr = true, desc = "Next Search Result" })
map("n", "N", "'nN'[v:searchforward].'zv'", { expr = true, desc = "Prev Search Result" })
map("x", "N", "'nN'[v:searchforward]", { expr = true, desc = "Prev Search Result" })
map("o", "N", "'nN'[v:searchforward]", { expr = true, desc = "Prev Search Result" })

-- 注释
map("n", "gco", "o<esc>Vcx<esc><cmd>normal gcc<cr>fxa<bs>", { desc = "Add Comment Below" })
map("n", "gcO", "O<esc>Vcx<esc><cmd>normal gcc<cr>fxa<bs>", { desc = "Add Comment Above" })

-- diagnostic
map("n", "<leader>xd", vim.diagnostic.open_float, { desc = "Line Diagnostics" })

-----------------------------------------------------------
map("n", "<Space>", "", opts())
map({ "n", "v" }, "q", "", opts())

-- 用E和B跳到行首行尾
map({ "n", "v" }, "E", "$", opts())
map({ "n", "v" }, "B", "^", opts())
map({ "n", "v" }, "+", "^", opts())

map({ "n" }, "F", "*", opts())

-- 模式切换
-- -- 插入模式下jk,kj退出
-- 由 better-escape 插件处理
-- map("i", "jk", "<Esc>", opts())
-- map("i", "kj", "<Esc>", opts())

-- 移动光标
map("n", "L", "<C-i>", opts("Jump Forward"))
map("n", "H", "<C-o>", opts("Junp Backward"))

-- -- JK光标移动多行
map({ "n", "v" }, "J", "7gj", opts())
map({ "n", "v" }, "K", "7gk", opts())

-- -- 空格 + o/O 插入一行而不进入插入模式
map("n", "<leader>o", "o<Esc>", opts("new line below"))
map("n", "<leader>O", "O<Esc>", opts("new line above"))

-- 目前让 <> 什么都不做
map({ "n", "v" }, "<", "<Nop>")
map({ "n", "v" }, ">", "<Nop>")

-- 移动__窗口
-- -- 大写 W 切换窗口焦点
map({ "n", "v" }, "W", "<C-w>w", opts())
-- --  左右方向键切换标签页,上下方向键什么都不干
map("n", "<Left>", ":tabprevious<CR>", opts())
map("n", "<Right>", ":tabNext<CR>", opts())
map("n", "<Up>", "", opts())
map("n", "<Down>", "", opts())

-- Tabs
map("n", "<leader>tl", "<cmd>tabnext<cr>", opts("Next tab"))
map("n", "<leader>th", "<cmd>tabprevious<cr>", opts("Prev tab"))
map("n", "<leader>td", "<cmd>tabclose<cr>", opts("Close tab"))
