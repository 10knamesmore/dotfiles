-- 为函数添加注释
-- <leader>cn
return {
    "danymat/neogen",
    cmd = "Neogen",
    keys = {
        {
            "<leader>cn",
            function()
                require("neogen").generate()
            end,
            desc = "生成注释 (Neogen)",
        },
    },
    opts = function()
        local i = require("neogen.types.template").item

        local custom_rust = {
            -- ========== 函数注释 - 无参数时 ==========
            { nil, "/ $1", { no_results = true, type = { "func" } } },
            { nil, "/ ", { no_results = true, type = { "func" } } },
            { nil, "/ # Return:", { no_results = true, type = { "func" } } },
            { nil, "/   $1", { no_results = true, type = { "func" } } },
            { nil, "/ ", { no_results = true, type = { "func" } } },
            { nil, "/ # Error:", { no_results = true, type = { "func" } } },
            { nil, "/   when $1 , return $1", { no_results = true, type = { "func" } } },
            { nil, "/ ", { no_results = true, type = { "func" } } },
            { nil, "/ # Examples:", { no_results = true, type = { "func" } } },
            { nil, "/ ```rust", { no_results = true, type = { "func" } } },
            { nil, "/ $1", { no_results = true, type = { "func" } } },
            { nil, "/ ```", { no_results = true, type = { "func" } } },
            { nil, "/ # Notes:", { no_results = true, type = { "func" } } },
            { nil, "/   $1", { no_results = true, type = { "func" } } },

            -- ========== 函数注释 - 有参数时 ==========
            { nil, "/ $1", { type = { "func" } } },
            { nil, "/ ", { type = { "func" } } },
            { nil, "/ # Params:", { type = { "func" } } },
            { i.Parameter, "/   - `%s`: $1", { type = { "func" } } },
            { nil, "/ ", { type = { "func" } } },
            { nil, "/ # Return:", { type = { "func" } } },
            { nil, "/   $1", { type = { "func" } } },
            { nil, "/ ", { type = { "func" } } },
            { nil, "/ # Error:", { type = { "func" } } },
            { nil, "/   when $1 , return $1", { type = { "func" } } },
            { nil, "/ ", { type = { "func" } } },
            { nil, "/ # Examples:", { type = { "func" } } },
            { nil, "/ ```rust", { type = { "func" } } },
            { nil, "/ $1", { type = { "func" } } },
            { nil, "/ ```", { type = { "func" } } },
            { nil, "/ ", { type = { "func" } } },
            { nil, "/ # Notes:", { type = { "func" } } },
            { nil, "/   $1", { type = { "func" } } },

            -- ========== 结构体/特征注释 - 无字段时 ==========
            { nil, "/ $1", { no_results = true, type = { "class" } } },
            { nil, "/ ", { no_results = true, type = { "class" } } },
            { nil, "/ # Examples:", { no_results = true, type = { "class" } } },
            { nil, "/ ```rust", { no_results = true, type = { "class" } } },
            { nil, "/ $1", { no_results = true, type = { "class" } } },
            { nil, "/ ```", { no_results = true, type = { "class" } } },

            -- ========== 结构体/特征注释 - 有字段时 ==========
            { nil, "/ $1", { type = { "class" } } },
            { nil, "/ ", { type = { "class" } } },
            { nil, "/ # Fields:", { type = { "class" } } },
            { i.Parameter, "/   - `%s`: $1", { type = { "class" } } },
            { nil, "/ ", { type = { "class" } } },
            { nil, "/ # Examples:", { type = { "class" } } },
            { nil, "/ ```rust", { type = { "class" } } },
            { nil, "/ $1", { type = { "class" } } },
            { nil, "/ ```", { type = { "class" } } },

            -- ========== 文件级注释 ==========
            { nil, "! $1", { no_results = true, type = { "file" } } },
            { nil, "! ", { no_results = true, type = { "file" } } },
            { nil, "! # 模块说明", { no_results = true, type = { "file" } } },
            { nil, "!   $1", { no_results = true, type = { "file" } } },
        }

        local opts = {
            enabled = true,
            input_after_comment = true, -- 自动跳转到插入模式
            snippet_engine = "nvim",
            enable_placeholders = true, -- 启用占位符

            -- 自定义占位符文本
            -- placeholders_text = {
            --     ["description"] = "[TODO:描述]",
            --     ["tparam"] = "[TODO:参数类型]",
            --     ["parameter"] = "[TODO:参数]",
            --     ["return"] = "[TODO:返回值]",
            --     ["class"] = "[TODO:类]",
            --     ["throw"] = "[TODO:异常]",
            --     ["varargs"] = "[TODO:可变参数]",
            --     ["type"] = "[TODO:类型]",
            --     ["attribute"] = "[TODO:属性]",
            --     ["args"] = "[TODO:参数]",
            --     ["kwargs"] = "[TODO:关键字参数]",
            -- },

            -- 占位符高亮（可设为 "None" 禁用）
            placeholders_hl = "DiagnosticHint",

            -- 语言特定配置
            languages = {
                rust = {
                    template = {
                        annotation_convention = "custom_rust",
                        use_default_comment = true,
                        custom_rust = custom_rust,
                    },
                },
            },
        }
        return opts
    end,
}
