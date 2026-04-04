local keymap = vim.keymap.set
local opts = { noremap = true, silent = true }

--- 生成 VSCode 命令调用表达式。
---@param command string
---@return string
local codeAction = function(command)
    return "<Cmd>lua require('vscode').action('" .. command .. "')<Cr>"
end

keymap("n", "<Space>", "", opts)
vim.g.mapleader = " "
vim.g.maplocalleader = " "

-- ========== Esc 多功能配置 ==========
-- 在 VSCode 中，esc 应该：
-- 1. 清除搜索高亮（nohlsearch）
-- 2. 关闭 snippet（如果有的话）
-- 3. 返回 normal 模式
keymap({ "i", "n", "s" }, "<esc>", function()
    vim.cmd("nohlsearch")
    -- VSCode 没有 vim.snippet，所以跳过
    -- 调用 VSCode 的关闭小部件命令
    vim.fn.VSCodeNotify("closeFindWidget")
    vim.fn.VSCodeNotify("closeParameterHints")
    codeAction("workbench.action.closeSideBar")
    return "<esc>"
end, { expr = true, desc = "Escape and Clear hlsearch" })

-- ========== 基础移动 ==========
keymap({ "n", "v" }, "E", "$", opts)
keymap({ "n", "v" }, "B", "^", opts)
keymap({ "n", "v" }, "J", "7j", opts)
keymap({ "n", "v" }, "K", "7k", opts)
keymap({ "n" }, "F", "*", opts)

-- ========== 插入空行（不进入插入模式）==========
keymap("n", "<leader>o", "o<Esc>", opts)
keymap("n", "<leader>O", "O<Esc>", opts)

-- ========== Jumplist 导航 ==========
-- H/L 由 keybindings.json 处理，调用 VSCode 命令
-- 这里不需要映射，因为我们希望使用 VSCode 的 jumplist

-- ========== 侧边栏和辅助栏 ==========
-- leader+e 聚焦文件树（由 keybindings.json 处理智能切换）
keymap("n", "<leader>e", codeAction("workbench.files.action.focusFilesExplorer"), opts)
keymap("n", "W", codeAction("workbench.action.focusSideBar"), opts)
-- leader+a 切换辅助栏（由 keybindings.json 处理智能聚焦）

-- ========== Buffer 管理 ==========
keymap("n", "<leader>b", codeAction("workbench.action.showAllEditors"), opts)

-- ========== 窗口管理 ==========
keymap("n", "<leader>wv", codeAction("workbench.action.splitEditor"), opts)
keymap("n", "<leader>ws", codeAction("workbench.action.splitEditorOrthogonal"), opts)
keymap("n", "<leader>wd", codeAction("workbench.action.closeActiveEditor"), opts)

-- Ctrl+hjkl 切换窗口焦点（由 keybindings.json 处理）
keymap("n", "<C-h>", codeAction("workbench.action.focusLeftGroup"), opts)
keymap("n", "<C-l>", codeAction("workbench.action.focusRightGroup"), opts)
keymap("n", "<C-k>", codeAction("workbench.action.focusAboveGroup"), opts)
keymap("n", "<C-j>", codeAction("workbench.action.focusBelowGroup"), opts)

-- ========== LSP 功能 ==========
-- gd/gr/gi/gy 由 keybindings.json 处理
keymap("n", "gd", codeAction("editor.action.revealDefinition"), opts)
keymap("n", "gr", codeAction("editor.action.goToReferences"), opts)
keymap("n", "gi", codeAction("editor.action.goToImplementation"), opts)
keymap("n", "gy", codeAction("editor.action.goToTypeDefinition"), opts)

-- ========== 文件和搜索 ==========
keymap("n", "<leader><space>", codeAction("workbench.action.quickOpen"), opts)
keymap("n", "<leader>/", codeAction("workbench.action.findInFiles"), opts)
keymap("n", "s", codeAction("actions.find"), opts)
keymap("n", "<leader>s:", codeAction("workbench.action.showCommands"), opts)

-- ========== 调试 ==========
keymap("n", "<leader>db", codeAction("editor.debug.action.toggleBreakpoint"), opts)

-- ========== 移动行（视觉模式）==========
keymap("v", "<A-j>", ":m '>+1<cr>gv=gv", opts)
keymap("v", "<A-k>", ":m '<-2<cr>gv=gv", opts)
