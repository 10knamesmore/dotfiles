-- code action 相关：灯泡提示 + diff 预览
return {
    -- 当光标所在行有可用 code action 时在 sign column 显示灯泡图标
    {
        "kosayoda/nvim-lightbulb",
        event = "LspAttach",
        opts = {
            --- sign/virtual_text/float/number/line 等 handler 的默认优先级
            --- 数值越大越靠前显示
            priority = 10,

            --- 当 buffer 失去焦点时是否隐藏灯泡
            hide_in_unfocused_buffer = false,

            --- 是否自动链接高亮组
            --- LightBulbSign       -> DiagnosticSignInfo
            --- LightBulbFloatWin   -> DiagnosticFloatingInfo
            --- LightBulbVirtualText-> DiagnosticVirtualTextInfo
            --- LightBulbNumber     -> DiagnosticSignInfo
            --- LightBulbLine       -> CursorLine
            link_highlights = true,

            --- 配置校验级别: "auto" | "always" | "never"
            --- "auto" 仅在 setup 时校验, "always" 每次 update 也校验
            validate_config = "auto",

            --- 只监听特定类型的 code action, nil 表示全部
            --- 例: { "quickfix", "refactor.rewrite" }
            --- 参考: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeActionKind
            action_kinds = nil,

            --- 是否启用 code lens 支持
            --- 当光标位置有可执行的 code lens 时, 图标从 text 变为 lens_text
            code_lenses = true,

            --- 1. Sign column 显示
            sign = {
                enabled = true,
                --- sign column 中显示的文字 (1-2 字符)
                text = "💡",
                --- 有 code lens 时显示的文字
                lens_text = "🔎",
                --- 高亮组
                hl = "LightBulbSign",
            },

            --- 2. 虚拟文本显示
            virtual_text = {
                enabled = true,
                text = "💡",
                lens_text = "🔎",
                --- 位置: "eol" (行尾) | "overlay" | "right_align" | 数字(固定列)
                pos = "eol",
                hl = "LightBulbVirtualText",
                --- 高亮混合模式: "combine" | "replace" | "blend"
                hl_mode = "combine",
            },

            --- 3. 浮动窗口显示
            float = {
                enabled = false,
                text = "💡",
                lens_text = "🔎",
                hl = "LightBulbFloatWin",
                --- 浮动窗口选项, 参考 :h nvim_open_win
                win_opts = {
                    focusable = false,
                },
            },

            --- 4. 状态文本 (可通过 NvimLightbulb.get_status_text() 获取, 用于 statusline)
            status_text = {
                enabled = false,
                text = "💡",
                lens_text = "🔎",
                --- 没有 code action 时显示的文本
                text_unavailable = "",
            },

            --- 5. 行号列高亮
            number = {
                enabled = false,
                hl = "LightBulbNumber",
            },

            --- 6. 整行高亮
            line = {
                enabled = false,
                hl = "LightBulbLine",
            },

            --- 自动命令配置
            autocmd = {
                --- 是否自动创建 autocmd 来更新灯泡
                --- 关闭后需手动调用 NvimLightbulb.update_lightbulb()
                enabled = true,
                --- 设置 vim 的 updatetime (影响 CursorHold 触发时间)
                --- 设为负数则不修改 updatetime
                updatetime = 200,
                --- 触发灯泡更新的事件
                events = { "CursorHold", "CursorHoldI" },
                --- 文件匹配模式
                pattern = { "*" },
            },

            --- 忽略规则
            ignore = {
                --- 忽略特定 LSP 客户端, 例: { "null-ls", "lua_ls" }
                clients = {},
                --- 忽略特定文件类型, 例: { "neo-tree", "lua" }
                ft = {},
                --- 是否忽略没有 kind 的 code action
                actions_without_kind = false,
            },

            --- 自定义过滤函数, 在 ignore 和 action_kinds 之后执行
            --- 返回 true 保留, false 过滤
            ---@type (fun(client_name:string, result:lsp.CodeAction|lsp.Command):boolean)|nil
            filter = nil,
        },
    },

    -- code action 前先预览 diff（替代原生 vim.lsp.buf.code_action）
    {
        "aznhe21/actions-preview.nvim",
        event = "LspAttach",
        opts = function()
            local hl = require("actions-preview.highlight")
            return {
                --- 后端优先级列表, 按顺序尝试, 使用第一个可用的
                --- 支持: "telescope" | "minipick" | "snacks" | "nui"
                backend = { "nui", "telescope", "snacks" },

                --- telescope 后端配置 (传给 telescope.pick)
                --- nil 使用 telescope 默认配置
                telescope = nil,

                --- nui 后端配置
                nui = {
                    --- 布局方向: "col" (上下) | "row" (左右)
                    dir = "row",
                    --- 自定义按键映射, nil 使用默认
                    keymap = nil,
                    --- 窗口布局
                    layout = {
                        position = "60%",
                        size = {
                            width = "60%",
                            height = "90%",
                        },
                        min_width = 40,
                        min_height = 10,
                        relative = "editor",
                    },
                    --- 预览窗口配置
                    preview = {
                        size = "60%",
                        border = {
                            style = "rounded",
                            padding = { 0, 1 },
                        },
                    },
                    --- 选择列表配置
                    select = {
                        size = "40%",
                        border = {
                            style = "rounded",
                            padding = { 0, 1 },
                        },
                    },
                },

                --- snacks 后端配置
                ---@type snacks.picker.Config
                snacks = {
                    layout = { preset = "default" },
                },

                --- diff 配置
                diff = {
                    --- diff 上下文行数 (匹配行周围显示的未修改行数)
                    ctxlen = 3,
                },

                --- diff 高亮命令 (使用外部工具渲染更漂亮的 diff)
                --- 需要对应工具已安装, 留空则使用 nvim 内置高亮
                --- 可选:
                ---   hl.delta()          -- 使用 delta (推荐, 需 sudo pacman -S git-delta)
                ---   hl.diff_so_fancy()  -- 使用 diff-so-fancy (需 sudo pacman -S diff-so-fancy)
                ---   hl.diff_highlight() -- 使用 diff-highlight (git 自带)
                highlight_command = {
                    hl.delta(),
                },
            }
        end,
        keys = {
            {
                "<leader>ca",
                function()
                    require("actions-preview").code_actions()
                end,
                mode = { "n", "x" },
                desc = "Code Action (Preview)",
            },
        },
    },
}
