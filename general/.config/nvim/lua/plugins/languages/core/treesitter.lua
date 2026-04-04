-- Treesitter 核心插件配置
-- 提供语法高亮、缩进、折叠等功能

return {
    -- nvim-treesitter - 核心解析器
    {
        "nvim-treesitter/nvim-treesitter",
        branch = "main",
        version = false, -- 最新版本太旧且在 Windows 上不工作
        build = function()
            local ok, TS = pcall(require, "nvim-treesitter")
            if not ok or not TS.get_installed then
                local Util = require("utils")
                Util.error("Please restart Neovim and run `:TSUpdate` to use `nvim-treesitter` **main** branch.")
                return
            end
            -- 确保使用最新的 treesitter util
            package.loaded["utils.treesitter"] = nil
            local Util = require("utils")
            Util.treesitter.build(function()
                TS.update(nil, { summary = true })
            end)
        end,
        event = { "BufReadPost", "BufNewFile", "BufWritePre", "VeryLazy" },
        cmd = { "TSUpdate", "TSInstall", "TSLog", "TSUninstall" },
        opts_extend = { "ensure_installed" },
        ---@class TSConfig
        opts = {
            -- 启用功能
            indent = { enable = true },
            highlight = { enable = true },
            folds = { enable = true },

            -- 确保安装的 parser
            ensure_installed = {
                "bash",
                "c",
                "diff",
                "html",
                "javascript",
                "jsdoc",
                "json",
                "jsonc",
                "lua",
                "luadoc",
                "luap",
                "markdown",
                "markdown_inline",
                "printf",
                "python",
                "query",
                "regex",
                "toml",
                "tsx",
                "typescript",
                "vim",
                "vimdoc",
                "xml",
                "yaml",
            },
        },
        ---@param opts TSConfig
        config = function(_, opts)
            local ok, TS = pcall(require, "nvim-treesitter")
            if not ok then
                return
            end

            -- 设置 treesitter
            TS.setup(opts)

            local Util = require("utils")
            Util.treesitter.get_installed(true) -- 初始化已安装的语言列表

            -- 安装缺失的 parser
            local install = vim.tbl_filter(function(lang)
                return not Util.treesitter.have(lang)
            end, opts.ensure_installed or {})

            if #install > 0 then
                Util.treesitter.build(function()
                    TS.install(install, { summary = true }):await(function()
                        Util.treesitter.get_installed(true) -- 刷新已安装列表
                    end)
                end)
            end

            -- 根据文件类型设置功能
            vim.api.nvim_create_autocmd("FileType", {
                group = vim.api.nvim_create_augroup("treesitter_config", { clear = true }),
                callback = function(ev)
                    local ft, lang = ev.match, vim.treesitter.language.get_lang(ev.match)
                    if not Util.treesitter.have(ft) then
                        return
                    end

                    ---@param feat string
                    ---@param query string
                    local function enabled(feat, query)
                        local f = opts[feat] or {}
                        return f.enable ~= false
                            and not (type(f.disable) == "table" and vim.tbl_contains(f.disable, lang))
                            and Util.treesitter.have(ft, query)
                    end

                    -- 语法高亮
                    if enabled("highlight", "highlights") then
                        pcall(vim.treesitter.start, ev.buf)
                    end

                    -- 缩进
                    if enabled("indent", "indents") then
                        Util.set_default("indentexpr", "v:lua.require('utils').treesitter.indentexpr()")
                    end

                    -- 折叠
                    if enabled("folds", "folds") then
                        if Util.set_default("foldmethod", "expr") then
                            Util.set_default("foldexpr", "v:lua.require('utils').treesitter.foldexpr()")
                        end
                    end
                end,
            })
        end,
    },

    -- nvim-treesitter-textobjects - 文本对象支持
    {
        "nvim-treesitter/nvim-treesitter-textobjects",
        branch = "main",
        event = "VeryLazy",
        opts = {
            move = {
                enable = true,
                set_jumps = true, -- 是否在跳转列表中设置跳转
                -- buffer-local 按键映射
                keys = {
                    goto_next_start = {
                        ["]f"] = "@function.outer",
                        ["]c"] = "@class.outer",
                        ["]a"] = "@parameter.inner",
                    },
                    goto_next_end = { ["]F"] = "@function.outer", ["]C"] = "@class.outer", ["]A"] = "@parameter.inner" },
                    goto_previous_start = {
                        ["[f"] = "@function.outer",
                        ["[c"] = "@class.outer",
                        ["[a"] = "@parameter.inner",
                    },
                    goto_previous_end = {
                        ["[F"] = "@function.outer",
                        ["[C"] = "@class.outer",
                        ["[A"] = "@parameter.inner",
                    },
                },
            },
        },
        config = function(_, opts)
            local ok, TS = pcall(require, "nvim-treesitter-textobjects")
            if not ok or not TS.setup then
                local Util = require("utils")
                Util.error("Please use `:Lazy` and update `nvim-treesitter`")
                return
            end
            TS.setup(opts)

            local function attach(buf)
                local ft = vim.bo[buf].filetype
                local Util = require("utils")
                if not (vim.tbl_get(opts, "move", "enable") and Util.treesitter.have(ft, "textobjects")) then
                    return
                end
                ---@type table<string, table<string, string>>
                local moves = vim.tbl_get(opts, "move", "keys") or {}

                for method, keymaps in pairs(moves) do
                    for key, query in pairs(keymaps) do
                        local desc = query:gsub("@", ""):gsub("%..*", "")
                        desc = desc:sub(1, 1):upper() .. desc:sub(2)
                        desc = (key:sub(1, 1) == "[" and "Prev " or "Next ") .. desc
                        desc = desc .. (key:sub(2, 2) == key:sub(2, 2):upper() and " End" or " Start")
                        -- 在 diff 模式下不覆盖 ]c 和 [c
                        if not (vim.wo.diff and key:find("[cC]")) then
                            vim.keymap.set({ "n", "x", "o" }, key, function()
                                require("nvim-treesitter-textobjects.move")[method](query, "textobjects")
                            end, {
                                buffer = buf,
                                desc = desc,
                                silent = true,
                            })
                        end
                    end
                end
            end

            vim.api.nvim_create_autocmd("FileType", {
                group = vim.api.nvim_create_augroup("treesitter_textobjects", { clear = true }),
                callback = function(ev)
                    attach(ev.buf)
                end,
            })
            vim.tbl_map(attach, vim.api.nvim_list_bufs())
        end,
    },

    -- nvim-ts-autotag - 自动闭合 HTML/JSX 标签
    {
        "windwp/nvim-ts-autotag",
        event = { "BufReadPost", "BufNewFile", "BufWritePre" },
        opts = {},
    },
}
