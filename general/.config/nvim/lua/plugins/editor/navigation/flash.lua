-- 提供光标跳转的功能
-- s 光标跳转
-- S treesitter支持的光标跳转,同时visual选定
return {
    "folke/flash.nvim",
    event = "VeryLazy",
    ---@type Flash.Config
    opts = {
        -- 可用的跳转标签字符列表
        labels = "asdfghjklqwertyuiopzxcvbnm",
        search = {
            -- 在所有窗口中搜索/跳转
            multi_window = true,
            -- 搜索方向
            forward = true,
            -- 为 false 时，仅在给定方向中查找匹配
            wrap = true,
            ---@type Flash.Pattern.Mode
            -- 每种模式都会考虑 ignorecase 和 smartcase。
            -- * exact：精确匹配
            -- * search：正则搜索
            -- * fuzzy：模糊搜索
            -- * fun(str)：返回模式的自定义函数
            --   例如，仅匹配单词开头：
            --   mode = function(str)
            --     return "\\<" .. str
            --   end,
            mode = "exact",
            -- 类似于 incsearch 的行为（边输入边搜索）
            incremental = false,
            -- 排除的 filetype 和自定义窗口过滤器
            ---@type (string|fun(win:window))[]
            exclude = {
                "notify",
                "cmp_menu",
                "noice",
                "flash_prompt",
                function(win)
                    -- 排除不可聚焦的窗口
                    return not vim.api.nvim_win_get_config(win).focusable
                end,
            },
            -- 可选的触发字符，在使用跳转标签前必须先输入。不推荐设置，除非你知道自己在做什么
            trigger = "",
            -- 模式最大长度。若达到该长度则不再跳过标签；超过该长度则会直接跳转或终止搜索
            max_length = false, ---@type number|false
        },
        jump = {
            -- 将跳转位置保存到 jumplist 中
            jumplist = true,
            -- 跳转位置
            pos = "start", ---@type "start" | "end" | "range"
            -- 将模式添加到搜索历史
            history = false,
            -- 将模式添加到搜索寄存器
            register = false,
            -- 跳转后清除高亮
            nohlsearch = false,
            -- 仅有一个匹配项时自动跳转
            autojump = false,
            -- 可以通过设置 `inclusive` 强制包含/不包含跳转，默认根据模式自动设置
            inclusive = nil, ---@type boolean?
            -- 跳转偏移（不用于范围跳转）
            -- 0：默认
            -- 1：当 pos == "end" 且 pos < 当前光标位置时
            offset = nil, ---@type number
        },
        label = {
            -- 允许使用大写标签
            uppercase = false,
            -- 排除要忽略的标签字符（区分大小写）
            exclude = "",
            -- 当前窗口中为第一个匹配添加标签。始终可以使用 <CR> 跳转到第一个匹配
            current = true,
            -- 在匹配后显示标签
            after = true, ---@type boolean|number[]
            -- 在匹配前显示标签
            before = false, ---@type boolean|number[]
            -- 标签 extmark 的显示位置
            style = "overlay", ---@type "eol" | "overlay" | "right_align" | "inline"
            -- Flash 会尝试复用已经分配的标签位置；默认只复用小写标签
            reuse = "lowercase", ---@type "lowercase" | "all" | "none"
            -- 当前窗口中优先标记靠近光标的目标
            distance = true,
            -- 显示标签的最小模式长度；对自定义标签器无效
            min_pattern_length = 0,
            -- 启用彩虹色标签高亮；适合可视化 Treesitter 范围
            rainbow = {
                enabled = false,
                -- 范围 1 到 9 的数字
                shade = 1,
            },
            -- 使用 format 自定义标签显示方式，应返回一个 [text, highlight] 元组的列表
            ---@class Flash.Format
            ---@field state Flash.State
            ---@field match Flash.Match
            ---@field hl_group string
            ---@field after boolean
            ---@type fun(opts:Flash.Format): string[][]
            format = function(opts)
                return { { opts.match.label, opts.hl_group } }
            end,
        },
        highlight = {
            -- 使用 FlashBackdrop 高亮显示背景
            backdrop = true,
            -- 高亮匹配结果
            matches = true,
            -- extmark 优先级
            priority = 5000,
            groups = {
                match = "FlashMatch",
                current = "FlashCurrent",
                backdrop = "FlashBackdrop",
                label = "FlashLabel",
            },
        },
        -- 选中标签后执行的操作，默认为根据模式跳转
        ---@type fun(match:Flash.Match, state:Flash.State)|nil
        action = nil,
        -- 启动 flash 时使用的初始模式
        pattern = "",
        -- 为 true 时尝试延续上一次搜索
        continue = false,
        -- 设置为函数以动态更改配置
        config = nil, ---@type fun(opts:Flash.Config)|nil
        -- 可为特定模式重写默认配置
        -- 使用方法：require("flash").jump({mode = "forward"})
        ---@type table<string, Flash.Config>
        modes = {
            -- 通过 `/` 或 `?` 搜索时启用 flash
            search = {
                -- 为 true 时，在普通搜索中默认启用 flash
                -- 使用 require("flash").toggle() 可随时切换
                enabled = false,
                highlight = { backdrop = false },
                jump = { history = true, register = true, nohlsearch = true },
                search = {
                    -- `forward` 会自动设置为搜索方向
                    -- `mode` 总为 `search`
                    -- 启用 incsearch 时，`incremental` 为 true
                },
            },
            -- 通过 `f`、`F`、`t`、`T`、`;` 和 `,` 启用 flash
            char = {
                enabled = false,
                -- 为 ftFT 动作启用动态配置
                config = function(opts)
                    -- 在操作等待模式中自动隐藏 flash
                    opts.autohide = opts.autohide or (vim.fn.mode(true):find("no") and vim.v.operator == "y")

                    -- 未启用、使用计数、或录制/执行寄存器时禁用跳转标签
                    opts.jump_labels = opts.jump_labels
                        and vim.v.count == 0
                        and vim.fn.reg_executing() == ""
                        and vim.fn.reg_recording() == ""

                    -- 仅在操作等待模式中显示跳转标签
                    -- opts.jump_labels = vim.v.count == 0 and vim.fn.mode(true):find("o")
                end,
                -- 跳转后若未使用标签则隐藏
                autohide = false,
                -- 显示跳转标签
                jump_labels = true,
                -- 设为 false 则只在当前行使用
                multi_line = true,
                -- 使用跳转标签时不使用这些按键
                -- 避免跳转后再输入这些按键
                label = { exclude = "hjkliardc" },
                -- 默认启用所有按键映射；如需禁用可从列表中移除
                -- 如需更换按键，可自定义映射，如 { [";"] = "L", [","] = H }
                -- keys = { "f", "F", "t", "T", ";", "," },
                keys = {},
                ---@alias Flash.CharActions table<string, "next" | "prev" | "right" | "left">
                -- `prev` 和 `next` 的方向由动作决定
                -- `left` 和 `right` 始终表示左右
                char_actions = function(motion)
                    -- return {
                    --     [";"] = "next", -- 可设为 "right" 表示总是向右
                    --     [","] = "prev", -- 可设为 "left" 表示总是向左
                    --     -- clever-f 风格
                    --     [motion:lower()] = "next",
                    --     [motion:upper()] = "prev",
                    --     -- jump2d 风格：相同大小写为 next，反之为 prev
                    --     -- [motion] = "next",
                    --     -- [motion:match("%l") and motion:upper() or motion:lower()] = "prev",
                    -- }
                    return {}
                end,
                search = { wrap = false },
                highlight = { backdrop = true },
                jump = {
                    register = false,
                    -- 使用跳转标签时，设为 true 则仅有一个匹配项时自动跳转或执行动作
                    autojump = false,
                },
            },
            -- treesitter 选择时使用的选项
            -- `require("flash").treesitter()`
            treesitter = {
                labels = "abcdefghijklmnopqrstuvwxyz",
                jump = { pos = "range", autojump = true },
                search = { incremental = false },
                label = { before = true, after = true, style = "inline" },
                highlight = {
                    backdrop = false,
                    matches = false,
                },
            },
            treesitter_search = {
                jump = { pos = "range" },
                search = { multi_window = true, wrap = true, incremental = false },
                remote_op = { restore = true },
                label = { before = true, after = true, style = "inline" },
            },
            -- 远程 flash 使用的选项
            remote = {
                remote_op = { restore = true, motion = true },
            },
        },
        -- 用于显示提示框的浮动窗口配置（用于常规跳转）
        -- `require("flash").prompt()` 始终可用于获取提示文本
        prompt = {
            enabled = true,
            prefix = { { "⚡", "FlashPromptIcon" } },
            win_config = {
                relative = "editor",
                width = 1, -- 小于等于 1 表示编辑器宽度的百分比
                height = 1,
                row = -1, -- 负数表示从底部偏移
                col = 0, -- 负数表示从右侧偏移
                zindex = 1000,
            },
        },
        -- 远程操作等待模式配置
        remote_op = {
            -- 操作完成后恢复窗口视图和光标位置
            restore = false,
            -- 若 jump.pos = "range" 则忽略此设置
            -- true：始终进入新动作
            -- false：使用窗口光标和跳转目标
            -- nil：远程窗口时为 true，当前窗口为 false
            motion = false,
        },
    },
    keys = {
        {
            "s",
            mode = { "n", "x", "o" },
            function()
                require("flash").jump()
            end,
            desc = "Flash",
        },
        {
            "S",
            mode = { "n", "x", "o" },
            function()
                require("flash").treesitter({
                    label = {
                        rainbow = {
                            enabled = true,
                            shade = 6,
                        },
                    },
                })
            end,
            desc = "Flash Treesitter",
        },
    },
}
