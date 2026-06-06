-- 辅助函数：创建一个 autocmd group，并自动在创建时清空同名组
-- clear = true 可以防止重复定义 autocmd
local function augroup(name)
    return vim.api.nvim_create_augroup("mygroup_" .. name, { clear = true })
end

-- =========================================
-- 自动检查文件变化（例如在外部修改后重新加载文件）
vim.api.nvim_create_autocmd({ "FocusGained", "TermClose", "TermLeave" }, {
    group = augroup("checktime"), -- 属于 checktime 组
    callback = function()
        if vim.o.buftype ~= "nofile" then -- 排除临时缓冲区
            vim.cmd("checktime") -- 检查文件是否修改
        end
    end,
})

-- =========================================
-- 高亮复制的文本
vim.api.nvim_create_autocmd("TextYankPost", {
    group = augroup("highlight_yank"), -- highlight_yank 组
    callback = function()
        (vim.hl or vim.highlight).on_yank() -- 高亮复制区域
    end,
})

-- =========================================
-- 窗口大小变化时自动调整分屏
vim.api.nvim_create_autocmd({ "VimResized" }, {
    group = augroup("resize_splits"), -- resize_splits 组
    callback = function()
        local current_tab = vim.fn.tabpagenr()
        vim.cmd("tabdo wincmd =") -- 平均分配所有窗口大小
        vim.cmd("tabnext " .. current_tab) -- 回到原来的标签页
    end,
})

-- =========================================
-- 打开 buffer 时跳转到上次光标位置
vim.api.nvim_create_autocmd("BufReadPost", {
    group = augroup("last_loc"), -- last_loc 组
    callback = function(event)
        local exclude = { "gitcommit" } -- 排除某些文件类型
        local buf = event.buf
        if vim.tbl_contains(exclude, vim.bo[buf].filetype) or vim.b[buf].lazyvim_last_loc then
            return
        end
        vim.b[buf].lazyvim_last_loc = true
        local mark = vim.api.nvim_buf_get_mark(buf, '"') -- 获取上次光标位置
        local lcount = vim.api.nvim_buf_line_count(buf)
        if mark[1] > 0 and mark[1] <= lcount then
            pcall(vim.api.nvim_win_set_cursor, 0, mark) -- 设置光标
        end
    end,
})

-- =========================================
-- 特定文件类型可用 <q> 快速关闭
vim.api.nvim_create_autocmd("FileType", {
    group = augroup("close_with_q"), -- close_with_q 组
    pattern = { -- 这些文件类型启用 <q> 关闭
        "PlenaryTestPopup",
        "checkhealth",
        "dbout",
        "gitsigns-blame",
        "grug-far",
        "help",
        "lspinfo",
        "neotest-output",
        "neotest-output-panel",
        "neotest-summary",
        "notify",
        "qf",
        "spectre_panel",
        "startuptime",
        "tsplayground",
    },
    callback = function(event)
        vim.bo[event.buf].buflisted = false -- 不加入缓冲区列表
        vim.schedule(function()
            vim.keymap.set("n", "q", function()
                vim.cmd("close")
                pcall(vim.api.nvim_buf_delete, event.buf, { force = true })
            end, {
                buffer = event.buf,
                silent = true,
                desc = "Quit buffer",
            })
        end)
    end,
})

-- =========================================
-- man 文件不列入缓冲区
vim.api.nvim_create_autocmd("FileType", {
    group = augroup("man_unlisted"),
    pattern = { "man" },
    callback = function(event)
        vim.bo[event.buf].buflisted = false
    end,
})

-- =========================================
-- JSON 文件修正 conceallevel
vim.api.nvim_create_autocmd({ "FileType" }, {
    group = augroup("json_conceal"),
    pattern = { "json", "jsonc", "json5" },
    callback = function()
        vim.opt_local.conceallevel = 0
    end,
})

-- =========================================
-- 保存文件前自动创建不存在的目录
vim.api.nvim_create_autocmd({ "BufWritePre" }, {
    group = augroup("auto_create_dir"),
    callback = function(event)
        if event.match:match("^%w%w+:[\\/][\\/]") then -- 排除 URL
            return
        end
        local file = vim.uv.fs_realpath(event.match) or event.match
        vim.fn.mkdir(vim.fn.fnamemodify(file, ":p:h"), "p") -- 创建目录
    end,
})

-- =========================================
-- 自定义文件类型识别：.alias 和 .zshfunc 识别为 sh
vim.api.nvim_create_autocmd({ "BufRead" }, {
    group = augroup("ft"),
    pattern = { ".alias", ".zsh*" },
    callback = function()
        vim.bo.filetype = "bash"
    end,
})

-- Dockerfile 文件识别
vim.api.nvim_create_autocmd({ "BufRead" }, {
    group = augroup("ft"),
    pattern = "Dockerfile*",
    callback = function()
        vim.bo.filetype = "dockerfile"
    end,
})

-- =========================================
-- CSV 文件打开时禁用换行
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
    pattern = "*.csv",
    callback = function()
        vim.opt_local.wrap = false
    end,
})

-- =========================================
-- JS/TS/HTML 文件缩进设为 2
vim.api.nvim_create_autocmd("FileType", {
    group = augroup("indent_2"),
    pattern = {
        "javascript",
        "typescript",
        "javascriptreact",
        "typescriptreact",
        "html",
        "css",
        "scss",
        "json",
        "jsonc",
        "yaml",
        -- "vue",
        "svelte",
    },
    callback = function()
        vim.opt_local.shiftwidth = 2
        vim.opt_local.tabstop = 2
        vim.opt_local.softtabstop = 2
    end,
})
