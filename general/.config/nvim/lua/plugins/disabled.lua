return {
    {
        "CopilotC-Nvim/CopilotChat.nvim",
        branch = "main",
        cmd = "CopilotChat", -- 通过 :CopilotChat 命令懒加载
        enabled = false,
        opts = function()
            -- 获取当前用户名，用于聊天头部显示
            local user = vim.env.USER or "User"
            user = user:sub(1, 1):upper() .. user:sub(2)

            --- @type CopilotChat.config.Config
            return {
                model = "gpt-4o",
                auto_insert_mode = false, -- 聊天窗口不自动进入插入模式
                language = "中文",
                system_prompt = [[
你是一名专业的软件工程师, 负责为用户提供高质量的技术支持和代码帮助.
你的回答应当全面, 完整, 专业, 准确, 深入, 并且符合上下文.
你的回答的每一部分不能只有一句话简单说明, 应当系统, 仔细地给出全面有效的回复
在回答时遇到可能过长回答时, 你不能只给出部分回答并要求用户给出继续的指令, 你应当直接, 完整地给出回答.
]],
                resources = {}, -- 默认通过 # 传过去的, 什么都不传
                remember_as_sticky = true, -- 把 models, tools, resources system_prompt 记为 sticky

                headers = {
                    user = "  " .. user .. " ", -- 用户头部图标和名称
                    assistant = "  Copilot ", -- AI 助手头部图标
                    tool = "󰊳  Tool ", -- 工具头部图标
                },
                window = {
                    width = 0.4,
                },

                log_level = "info",

                prompts = {
                    Translate = {
                        prompt = "请将选中的内容翻译成中文, 如果格式不是md, 则转化成markdown",
                        system_prompt = [[
你是一名专业的软件工程师, 同时精通多国语言翻译.
你应当直接给出翻译后的内容, 不需要任何额外说明.
不论选中内容有多长, 你都应当直接, 完整, 专业, 准确地给出全部翻译后的内容.
]],
                        remember_as_sticky = true,
                        resources = { "buffer:active", "selection" },
                    },
                    Explain = {
                        prompt = "请根据整个文件内容, 详细解释选择代码. ",
                        system_prompt = [[
你是一名专业的软件工程师, 你的任务是解释代码的逻辑流程和功能.
你的回答会被用于帮助其他高级开发者快速了解代码片段的作用, 因此你回答的目的是让开发者在没有阅读代码的情况下, 也能理解代码的逻辑和功能.
你的输出应当分为三个部分: 
    1. 逻辑流程(先用markdown自然语言描述流程, 再考虑用unicode描述流程图), 
    2. 功能(什么情况下执行, 输入是什么, 输出是什么), 
    3. 选中在整体上起到的作用。
你输出的内容应当尽可能详细, 专业, 完整, 全面
]],

                        remember_as_sticky = true,
                        resources = {
                            "selection",
                            "buffer:active",
                        },
                    },

                    Review = {
                        prompt = "请审查所选代码。",
                        system_prompt = [[
你是一名专注于提升代码质量和可维护性的代码审查员。


请严格按照以下格式列出你发现的每个问题, severity取这一行最严重的问题的情况：
line=<行号>:severity <问题描述>
或
line=<起始行>-<结束行>:severity <问题描述>
比如
"""
line=117:INFO 命名callback不够具体，建议根据实际用途使用更具描述性的名称;response和source参数未加类型注释，影响可读性;缺少对callback函数的注释说明其作用和参数含义。
"""

其中severity是一个枚举类型，包含以下成员：
severity : {
  ERROR,
  WARN,
  INFO,
  HINT,
}
其中
ERROR：严重错误，影响程序正确性或存在安全隐患，如未处理的异常、明显的逻辑错误、资源泄漏、SQL注入等安全问题。
WARN：潜在问题，可能导致bug或维护困难，如复杂表达式、过深嵌套、性能隐患、错误处理不完善、违反SOLID原则等。
INFO：与代码逻辑无关但影响可读性或规范性的问题，如命名不规范、注释缺失/错误、风格不一致、代码重复、格式问题, 脚本语言缺少typehint等。
HINT：轻微建议或可选优化，如微小的风格改进、可读性提升建议、非强制性重构等。


请检查以下方面：
- 命名不清晰或不符合规范
- 注释质量（缺失, 错误）
- 需要简化的复杂表达式
- 过深的嵌套或复杂的控制流
- 风格或格式不一致, 或与当前语言的idom不符
- 代码重复或冗余
- 潜在的性能问题
- 错误处理缺失
- 安全隐患
- 违反SOLID原则

同一行的多个问题请用分号分隔。
最后以：“**`如需清除缓冲区高亮，请提出其他问题。`**”结尾。

如果未发现问题，请确认代码编写良好并说明原因。
]],
                        callback = function(response, source)
                            local diagnostics = {}
                            for line in response.content:gmatch("[^\r\n]+") do
                                if line:find("^line=") then
                                    local start_line, end_line, message, severity
                                    -- 匹配 line=10:WARN 问题描述 或 line=10-12:ERROR 问题描述
                                    local single_match, sev, message_match = line:match("^line=(%d+):([A-Z]+)%s+(.*)$")
                                    if not single_match then
                                        local start_match, end_match, sev2, m_message_match =
                                            line:match("^line=(%d+)-(%d+):([A-Z]+)%s+(.*)$")
                                        if start_match and end_match then
                                            start_line = tonumber(start_match)
                                            end_line = tonumber(end_match)
                                            severity = sev2
                                            message = m_message_match
                                        end
                                    else
                                        start_line = tonumber(single_match)
                                        end_line = start_line
                                        severity = sev
                                        message = message_match
                                    end

                                    if start_line and end_line and severity then
                                        -- 映射 severity 字符串到 vim.diagnostic.severity
                                        local sev_map = {
                                            ERROR = vim.diagnostic.severity.ERROR,
                                            WARN = vim.diagnostic.severity.WARN,
                                            INFO = vim.diagnostic.severity.INFO,
                                            HINT = vim.diagnostic.severity.HINT,
                                        }
                                        -- TODO: 将message作为字符串, 用分号分割成数组 messages
                                        -- TODO: 遍历messages, 都insert
                                        for msg in message:gmatch("([^;]+)") do
                                            table.insert(diagnostics, {
                                                lnum = start_line - 1,
                                                end_lnum = end_line - 1,
                                                col = 0,
                                                message = vim.trim(msg),
                                                severity = sev_map[severity] or vim.diagnostic.severity.WARN,
                                                source = "Copilot Review",
                                            })
                                        end
                                    end
                                end
                            end
                            vim.diagnostic.set(
                                vim.api.nvim_create_namespace("copilot-chat-diagnostics"),
                                source.bufnr,
                                diagnostics
                            )
                        end,

                        resources = { "selection", "buffer:active" },
                    },

                    Fix = {
                        prompt = "这段代码存在问题。请指出问题并重写修复后的代码，解释原问题及你的修复方案。",
                        system_prompt = [[
你是一名专业的软件工程师，擅长代码审查与修复。你的任务是：

1. 仔细分析用户选中的代码，指出其中存在的所有问题（包括语法错误、逻辑缺陷、性能隐患、可读性问题等），并用简明扼要的语言逐条列出。
2. 针对每个问题，详细解释其原因及可能带来的影响。
3. 在充分理解上下文的基础上，重写修复后的完整代码，确保其功能正确、风格规范、易于维护。
4. 说明你的修复方案和优化思路，包括为何这样修改，以及修改后带来的好处。
5. 回答应当系统、全面、专业，不能遗漏任何明显问题或修复细节。

引用文件内容时，请用
line:123-234 你的回答
的形式回复。

如有代码片段，请直接给出，不需征询用户意见，并用代码块返回。

请尽可能详细地回答问题。
]],
                        remember_as_sticky = true,
                        resources = { "selection", "buffer:active" },
                    },

                    Optimize = {
                        prompt = "请优化所选代码以提升性能和可读性，并说明你的优化策略及其好处。",
                        resources = { "selection", "buffer:active" },
                        system_prompt = [[
你是一名专业的软件工程师，擅长代码优化与重构。你的任务是：

1. 仔细分析用户选中的代码，结合整体文件内容，发现所有可优化的点，包括但不限于性能提升、可读性增强、结构简化、资源利用、边界处理等。
2. 针对每个优化点，详细说明你的优化策略、修改方案及其带来的好处（如性能提升、易维护、减少错误等）。
3. 直接给出优化后的完整代码片段，并用代码块返回，无需征询用户意见。
4. 回答应当系统、全面、专业，不能遗漏任何明显的优化机会或细节。
5. 如有引用文件内容，请用
line:123-234 你的回答
的形式回复。
6. 优化建议和代码修改需有助于提升代码质量、团队协作效率和长期可维护性。
]],
                    },

                    Docs = {
                        prompt = "请为所选代码添加文档注释并添加尽可能详细的typehints。",
                        system_prompt = [[
你是一名专业的软件工程师，精通主流编程语言的文档注释和类型标注规范。你的任务是：

1. 为用户选中的代码添加详细、规范的文档注释，注释内容应涵盖函数/类/模块的用途、参数说明、返回值、异常、边界情况等。
2. 补充尽可能详细的类型注解（typehints），遵循当前语言的最佳实践和主流风格。
3. 注释和类型标注需有助于提升代码可读性、可维护性和团队协作效率。
4. 如代码已有部分注释或类型标注，请补充完善而非重复。
5. 输出应为完整的代码片段，直接修改原代码并用代码块返回，无需额外说明。
6. 回答应当全面、专业、准确，不能遗漏任何明显的注释或类型标注细节。

你的回答应当分为两部分:
1. 用 ```代码块 包裹的 完整代码片段, 包含注释和typehints
2. 你在代码中添加的注释和typehints的说明, 包含你遵循的规范.

我的rust文档规范如下:
- 注释结构分为函数、结构体/特征、文件级三类，且区分有无参数/字段的情况。
- 函数注释（无参数）包含：
  - 简要描述
  - Return（返回值）
  - Error（错误情况及返回值）
  - Examples（示例，代码块用 ```rust 包裹）
  - Notes（补充说明）

- 函数注释（有参数）包含：
  - 简要描述
  - Params（参数列表，格式为 /   - `参数名`: 描述）
  - Return
  - Error
  - Examples
  - Notes

- 结构体/特征注释（无字段）包含：
  - 简要描述
  - Examples（代码块）

- 结构体/特征注释（有字段）包含：
  - 简要描述
  - Fields（字段列表，格式为 /   - `字段名`: 描述）
  - Examples

- 文件级注释以 ! 开头，包含模块说明。
]],
                        remember_as_sticky = true,
                        resources = { "selection", "buffer:active" },
                    },

                    Tests = {
                        prompt = "请为我的代码生成测试用例。",
                        resources = { "selection", "buffer:active" },
                    },

                    Commit = {
                        prompt = "请根据 commitizen 规范为更改编写提交信息。标题保持在 50 个字符以内，正文每行不超过 72 个字符。以 gitcommit 代码块格式输出.",
                        resources = {
                            "gitdiff:staged",
                        },
                    },
                },
            }
        end,
        keys = {
            -- CopilotChat 的快捷键映射
            { "<c-s>", "<CR>", ft = "CopilotChat", desc = "提交问题", remap = true },
            { "<leader>a", "", desc = "+ai", mode = { "n", "x" } }, -- AI 分组前缀
            {
                "<leader>aa",
                function()
                    return require("CopilotChat").toggle() -- 切换聊天窗口显示/隐藏
                end,
                desc = "切换 CopilotChat",
                mode = { "v", "n", "x" },
            },
            {
                "<leader>ax",
                function()
                    return require("CopilotChat").reset() -- 清空聊天历史
                end,
                desc = "清空 CopilotChat",
                mode = { "n", "x" },
            },
            {
                "<leader>aq",
                function()
                    -- 弹出输入框进行快速提问
                    Snacks.input({
                        prompt = "快速提问: ",
                        win = {
                            title_pos = "left",
                            relative = "cursor",
                            col = -5,
                        },
                    }, function(input)
                        local system_prompt = [[
你是一名专业的软件工程师, 负责为用户提供高质量的技术支持和代码帮助。
你的回答应当全面、完整、专业、准确、深入, 并且符合上下文。
你的回答的每一部分不能只有一句话简单说明, 应当系统、仔细地给出全面有效的回复。
在回答时遇到可能过长回答时, 你不能只给出部分回答并要求用户给出继续的指令, 你应当直接、完整地给出回答。
当你引用文件内容时, 为了让我更快找到上下文, 请用
line:123-234 你的回答
的形式回复。
当你可以给出代码片段时, 请直接给出不需要听取我的意见, 并用代码块的形式返回。
给出代码块不代表你需要停止回答, 你可以继续补充说明。
请尽可能详细地回答我的问题。
]]
                        if input ~= "" then
                            require("CopilotChat").reset()
                            require("CopilotChat").ask(input, {
                                system_prompt = system_prompt,
                                remember_as_sticky = true,
                                resources = { "selection", "buffer:active" }, -- 如果有选中内容，则将其作为资源传递给 AI
                            })
                        end
                    end)
                end,
                desc = "AI 快速提问（带上下文提示）",
                mode = { "v", "n", "x" },
            },
            {
                "<leader>ap",
                function()
                    require("CopilotChat").select_prompt() -- 显示提示操作
                end,
                desc = "选择预设 promts",
                mode = { "v", "n", "x" },
            },
            {
                "<leader>am",
                function()
                    require("CopilotChat").select_model() -- 选择 AI 提供商
                end,
                desc = "选择模型",
                mode = { "n" },
            },
            {
                "<leader>ao",
                function()
                    require("CopilotChat").open({ resources = {} })
                end,
                desc = "打开 CopilotChat 窗口",
                mode = { "v", "n", "x" },
            },
        },
        config = function(_, opts)
            local chat = require("CopilotChat")

            -- 进入聊天 buffer 时隐藏行号
            vim.api.nvim_create_autocmd("BufEnter", {
                pattern = "copilot-chat",
                callback = function()
                    -- TODO
                    vim.wo.signcolumn = "no"
                    vim.wo.number = false
                end,
            })

            chat.setup(opts) -- 用配置初始化插件
        end,
    },
    {
        import = "nvim-neo-tree/neo-tree.nvim",
        enabled = false,
    },
    {
        "akinsho/bufferline.nvim",
        event = "VeryLazy",
        enabled = false,
        keys = {
            { "<leader>bp", "<Cmd>BufferLineTogglePin<CR>", desc = "Toggle Pin" },
            -- { "<leader>bP", "<Cmd>BufferLineGroupClose ungrouped<CR>", desc = "Delete Non-Pinned Buffers" },
            { "<leader>bl", "<Cmd>BufferLineCloseRight<CR>", desc = "删除右侧的 buffer" },
            { "<leader>bh", "<Cmd>BufferLineCloseLeft<CR>", desc = "删除左侧的 buffer" },
            { "<S-h>", "<cmd>BufferLineCyclePrev<cr>", desc = "Prev Buffer" },
            { "<S-l>", "<cmd>BufferLineCycleNext<cr>", desc = "Next Buffer" },
            -- { "[b", "<cmd>BufferLineCyclePrev<cr>", desc = "Prev Buffer" },
            -- { "]b", "<cmd>BufferLineCycleNext<cr>", desc = "Next Buffer" },
            { "[B", "<cmd>BufferLineMovePrev<cr>", desc = "Move buffer prev" },
            { "]B", "<cmd>BufferLineMoveNext<cr>", desc = "Move buffer next" },
        },
        opts = {
            options = {
                close_command = function(n)
                    Snacks.bufdelete(n)
                end,
                right_mouse_command = function(n)
                    -- Snacks.bufdelete(n)
                end,
                diagnostics = "nvim_lsp",
                always_show_bufferline = false,
                diagnostics_indicator = function(_, _, diag)
                    local icons = utils.config.icons.diagnostics
                    local ret = (diag.error and icons.Error .. diag.error .. " " or "")
                        .. (diag.warning and icons.Warn .. diag.warning or "")
                    return vim.trim(ret)
                end,
                indicator = {
                    icon = "▎", -- this should be omitted if indicator style is not 'icon'
                    style = "icon",
                },
                offsets = {
                    {
                        filetype = "neo-tree",
                        text = "Neo-tree",
                        highlight = "Directory",
                        text_align = "left",
                    },
                    {
                        filetype = "snacks_layout_box",
                    },
                },
                hover = {
                    enabled = true,
                    delay = 200,
                    reveal = { "close" },
                },
                -- separator_style = "slant" | "slope" | "thick" | "thin" | { "any", "any" },
                separator_style = "slope",
                ---@param opts bufferline.IconFetcherOpts
                get_element_icon = function(opts)
                    return utils.config.icons.ft[opts.filetype]
                end,
                sort_by = function(buffer_a, buffer_b)
                    -- -- add custom logic
                    -- local modified_a = vim.fn.getftime(buffer_a.path)
                    -- local modified_b = vim.fn.getftime(buffer_b.path)
                    -- return modified_a > modified_b
                    return buffer_a.id < buffer_b.id
                end,
            },
        },
        config = function(_, opts)
            require("bufferline").setup(opts)
            -- Fix bufferline when restoring a session
            vim.api.nvim_create_autocmd({ "BufAdd", "BufDelete" }, {
                callback = function()
                    vim.schedule(function()
                        pcall(nvim_bufferline)
                    end)
                end,
            })
        end,
    },
    {
        "rmagatti/goto-preview", -- 插件名称：用于“预览跳转”功能，如函数定义/引用的浮动窗口预览
        enabled = false,
        dependencies = { "rmagatti/logger.nvim" }, -- 插件依赖项（记录日志用）
        event = "BufEnter", -- 在进入缓冲区时懒加载该插件

        config = true, -- 启用自动加载 setup 配置（必须为 true，否则 issue #88 会导致失效）

        opts = {
            width = 240, -- 浮动窗口的宽度（字符宽）
            height = 100, -- 浮动窗口的高度（行数）
            border = { "↖", "─", "┐", "│", "┘", "─", "└", "│" }, -- 浮动窗口的边框样式，依次为 ↖ 上 右上 右 右下 下 左下 左 左
            default_mappings = true, -- 是否使用默认快捷键（例如：`gpd`, `gpt` 等）
            debug = false, -- 是否输出调试信息（通常设为 false）
            opacity = nil, -- 窗口透明度，范围 0~100，100 是完全透明（nil 表示默认不透明）
            resizing_mappings = false, -- 是否启用方向键调整浮动窗口大小
            post_open_hook = nil, -- 在浮动窗口打开后执行的钩子函数（可自定义）
            post_close_hook = nil, -- 在浮动窗口关闭后执行的钩子函数（可自定义）

            references = {
                provider = "telescope", -- 引用预览列表所使用的 UI 组件：支持 telescope/fzf_lua/snacks/mini_pick/default
                telescope = require("telescope.themes").get_dropdown({ hide_preview = false }),
                -- telescope 的具体配置：使用下拉样式，且不隐藏右侧 preview 区域
            },

            focus_on_open = true, -- 打开浮动窗口时自动聚焦（获得输入焦点）
            dismiss_on_move = false, -- 移动光标时是否自动关闭浮动窗口（false 表示保持打开）
            force_close = true, -- 是否使用 `force` 方式关闭浮动窗口（对应 vim.api.nvim_win_close 的第二参数）
            bufhidden = "wipe", -- 设置浮动窗口中的 buffer 的隐藏行为为 wipe，详见 :h bufhidden
            stack_floating_preview_windows = true, -- 是否允许浮动窗口叠加（多个窗口堆叠显示）
            same_file_float_preview = true, -- 当前文件内的符号引用是否也使用浮动窗口打开
            preview_window_title = {
                enable = true, -- 是否显示窗口标题（显示文件名）
                position = "left", -- 标题显示在窗口的左边
            },
            zindex = 1, -- 浮动窗口的 z-index 层级（用于叠加窗口排序）
            vim_ui_input = true, -- 是否使用 goto-preview 的浮动窗口替代 `vim.ui.input`（可用于插件交互）
        },
    },
    {
        "MagicDuck/grug-far.nvim",
        enabled = false,
    },
}
