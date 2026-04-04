-- 定义仪表盘各个部分的配置
--- @type snacks.dashboard.Section
local dashboard_sections = {
    {
        ttl = 0,
        enabled = function()
            return vim.o.lines > 75 and vim.o.columns > 110
        end,
        section = "terminal",
        cmd = 'ascii-image-converter --color -H50 "' .. vim.fn.stdpath("config") .. '/data/mutsumi.jpg"',
        height = 50,
        padding = 2,
        align = "center",
    },
    {
        pane = 1,
        icon = " ", -- 图标
        title = "Recent Files", -- 标题
        section = "recent_files", -- 显示最近打开的文件
        indent = 2, -- 缩进
        padding = 1, -- 边距
        gap = 1,
    },
    {
        pane = 1,
        icon = " ",
        title = "Projects", -- 显示项目列表
        section = "projects",
        indent = 2,
        padding = 1,
        gap = 1,
    },
    { section = "keys", gap = 1, padding = 1 }, -- 显示按键绑定，设置间距与边距
    { section = "startup" }, -- 启动状态
}

-- 插件定义与主配置导出
return {
    "folke/snacks.nvim",
    priority = 1000, -- 优先加载
    lazy = false, -- 启动时立即加载插件
    opts = function()
        ---@type snacks.Config
        local opts = {
            animate = { enabled = true, fps = 180 }, -- 启用动画效果，帧率为 180
            bigfile = { enabled = false }, -- 启用大文件处理优化, 交给bigfil.lua处理
            dashboard = {
                enabled = true,
                sections = dashboard_sections,
                width = 100,
                preset = {
                    keys = {
                        { icon = "", key = "g", desc = "LazyGit", action = "<leader>gg" },
                        { icon = "", key = "q", desc = "Quit", action = ":qa" },
                        {
                            icon = "✎",
                            key = "s",
                            desc = "New Scratch",
                            action = function()
                                require("snacks").scratch.open()
                            end,
                        },
                        {
                            icon = "✎",
                            key = "S",
                            desc = "Search Scratch",
                            action = function()
                                require("snacks").scratch.select()
                            end,
                        },
                    },
                },
            }, -- 启用仪表盘功能并加载定义的各部分
            dim = {
                scope = {
                    min_size = 10,
                    siblings = true,
                },
                animate = {
                    duration = {
                        step = 15,
                        total = 150,
                    },
                },
            },
            explorer = { enabled = false, replace_netrw = false }, -- 文件浏览器功能禁用
            input = { enabled = true }, -- 启用输入增强（如 float 弹窗输入）
            profiler = { enabled = false },
            indent = {
                indent = {
                    enabled = true, -- 启用缩进线
                },
                scope = {
                    enabled = true, -- 启用作用域缩进高亮
                    underline = false, -- 不使用下划线
                    hl = { -- 定义每层缩进的高亮组
                        "SnacksIndent1",
                        "SnacksIndent2",
                        "SnacksIndent3",
                        "SnacksIndent4",
                        "SnacksIndent5",
                        "SnacksIndent6",
                        "SnacksIndent7",
                        "SnacksIndent8",
                    },
                },
                chunk = {
                    enabled = true, -- 启用块级缩进高亮
                    hl = {
                        "SnacksIndent1",
                        "SnacksIndent2",
                        "SnacksIndent3",
                        "SnacksIndent4",
                        "SnacksIndent5",
                        "SnacksIndent6",
                        "SnacksIndent7",
                        "SnacksIndent8",
                    },
                    char = {
                        corner_top = "╭", -- 块顶部符号
                        corner_bottom = "╰", -- 块底部符号
                    },
                },
            },
            notifier = { enabled = false }, -- 禁用，交给 noice + nvim-notify 处理
            quickfile = { enabled = true }, -- 启用快速文件访问
            scroll = { enabled = false },
            scratch = {
                enabled = true,
                name = "草稿",
                ft = "markdown",
                win = {
                    width = 0.6,
                    min_width = 100,
                    height = 0.8,
                    bo = { buftype = "", buflisted = false, bufhidden = "hide", swapfile = false },
                    minimal = false,
                    noautocmd = false,
                    -- position = "right",
                    zindex = 20,
                    wo = { winhighlight = "NormalFloat:Normal" },
                    footer_keys = true,
                    border = "rounded",
                    resize = true,
                },
            }, -- 启用 scratch buffer，并命名为“草稿”
            statuscolumn = { enabled = false }, -- 启用状态列（通常用于显示行号、git 标记等）
            words = { enabled = false }, -- 单词增强功能禁用
            terminal = {
                win = {
                    keys = {
                        -- 终端导航辅助（从 core/util.lua 合并）
                        nav_h = {
                            "<C-h>",
                            function(self)
                                return self:is_floating() and "<c-h>"
                                    or vim.schedule(function()
                                        vim.cmd.wincmd("h")
                                    end)
                            end,
                            desc = "Go to Left Window",
                            expr = true,
                            mode = "t",
                        },
                        nav_j = {
                            "<C-j>",
                            function(self)
                                return self:is_floating() and "<c-j>"
                                    or vim.schedule(function()
                                        vim.cmd.wincmd("j")
                                    end)
                            end,
                            desc = "Go to Lower Window",
                            expr = true,
                            mode = "t",
                        },
                        nav_k = {
                            "<C-k>",
                            function(self)
                                return self:is_floating() and "<c-k>"
                                    or vim.schedule(function()
                                        vim.cmd.wincmd("k")
                                    end)
                            end,
                            desc = "Go to Upper Window",
                            expr = true,
                            mode = "t",
                        },
                        nav_l = {
                            "<C-l>",
                            function(self)
                                return self:is_floating() and "<c-l>"
                                    or vim.schedule(function()
                                        vim.cmd.wincmd("l")
                                    end)
                            end,
                            desc = "Go to Right Window",
                            expr = true,
                            mode = "t",
                        },
                        hide_slash = { "<C-/>", "hide", desc = "Hide Terminal", mode = { "t", "n" } },
                        hide_underscore = { "<c-_>", "hide", desc = "which_key_ignore", mode = { "t", "n" } },
                    },
                },
            },
            zen = {
                toggles = {
                    dim = false,
                },
                win = {
                    width = 160,
                },
            },
        }

        return opts
    end,
    keys = function()
        require("snacks").toggle.diagnostics():map("<leader>ud")
        require("snacks").toggle.dim():map("<leader>uD")
        require("snacks").toggle.zen():map("<leader>uz")
        local keys = {
            {
                ".",
                function()
                    require("snacks").scratch.open()
                end,
                desc = "Scratch",
            },
            {
                "<leader>s.",
                function()
                    Snacks.picker.scratch({ focus = "list" })
                end,
                desc = "Select Scratch Buffer",
            },

            {
                "<leader>gg",
                function()
                    require("snacks").lazygit()
                end,
                desc = "Lazygit (cwd)",
                mode = "n",
            },
            {
                "<leader>gc",
                function()
                    require("snacks").picker.git_log()
                end,
                desc = "Git commits",
                mode = "n",
            },
            {
                "<leader>gs",
                function()
                    require("snacks").picker.git_status()
                end,
                desc = "Git Status",
                mode = "n",
            },
            {
                "<leader>gS",
                function()
                    require("snacks").picker.git_stash()
                end,
                desc = "Git stash",
                mode = "n",
            },
            {
                "<leader>gl",
                function()
                    require("snacks").picker.git_log_line()
                end,
                desc = "Git log line",
                mode = "n",
            },
            {
                "<leader>gf",
                function()
                    require("snacks").picker.git_log_file()
                end,
                desc = "Git Current File History",
                mode = "n",
            },
            {
                "<leader>gB",
                function()
                    require("snacks").gitbrowse()
                end,
                desc = "git open remote",
                mode = "n",
            },

            {
                "<leader>n",
                function()
                    require("snacks").notifier.show_history()
                end,
                desc = "Notification History",
            },
            {
                "<c-/>",
                function()
                    require("snacks").terminal()
                end,
                desc = "Terminal (Root Dir)",
                mode = { "n", "t" },
            },
        }
        return keys
    end,
}
