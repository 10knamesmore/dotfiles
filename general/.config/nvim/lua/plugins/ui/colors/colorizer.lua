-- 把 hex 用色块表现
return {
    "catgoose/nvim-colorizer.lua",
    event = "BufReadPre",
    cmd = { "ColorizerToggle" },
    opts = function()
        ---@type colorizer.SetupOptions
        return {
            filetypes = { "*" }, -- 针对哪些文件类型启用高亮，"*" 表示所有文件类型
            buftypes = {}, -- 针对哪些 buffer 类型启用高亮，留空表示所有 buffer 类型
            user_commands = true, -- 启用插件提供的用户命令（如 :ColorizerToggle 等）
            lazy_load = false, -- 是否懒加载高亮功能，false 表示立即加载当前缓冲区

            options = {
                parsers = {
                    names = {
                        enable = false, -- 启用颜色名字（如 "blue", "red"）的高亮
                        lowercase = true, -- 匹配小写的名字（如 "blue"）
                        camelcase = true, -- 匹配驼峰命名（如 "Blue"）
                        uppercase = false, -- 不匹配全大写（如 "BLUE"）
                        strip_digits = false, -- 是否忽略带数字的颜色名（如 "red4"）
                        custom = false, -- 自定义颜色名字映射（例如 { sky = "#87ceeb" }），false 表示关闭
                    },

                    hex = {
                        enable = true,
                        rgb = true, -- 支持 #RGB 格式（如 #abc）
                        rgba = true, -- 支持 #RGBA 格式（如 #abcd）
                        rrggbb = true, -- 支持 #RRGGBB 格式（如 #aabbcc）
                        rrggbbaa = false, -- 关闭 #RRGGBBAA 格式（带 alpha 通道的8位 hex）
                        aarrggbb = true, -- 开启 0xAARRGGBB 格式 (为了 yabai JankyBorders)
                    },

                    rgb = { enable = false }, -- 不启用 CSS 的 rgb()、rgba() 函数支持
                    hsl = { enable = false }, -- 不启用 CSS 的 hsl()、hsla() 函数支持
                    oklch = { enable = false },

                    css = false, -- 是否启用所有 CSS 特性（包括名字、RGB、RGBA 等）
                    css_fn = false, -- 是否启用所有 CSS 函数（rgb/hsl/oklch）

                    tailwind = {
                        enable = false, -- 关闭 TailwindCSS 的颜色高亮
                        lsp = false,
                        update_names = false,
                    },

                    sass = {
                        enable = false,
                        parsers = { css = true }, -- sass 文件也使用 css 的语法解析器
                    },
                },

                display = {
                    mode = "background", -- 使用背景色高亮（可选值：'background'、'foreground'、'virtualtext'）
                    virtualtext = {
                        char = "■", -- 用于 virtual text 模式时显示的符号
                        position = "after", -- 对应旧版 virtualtext_inline = true
                        hl_mode = "foreground", -- virtual text 的颜色来自于前景色
                    },
                },

                always_update = false, -- 是否在 buffer 不处于焦点时仍更新颜色（适用于 cmp 补全等情况）

                hooks = {
                    should_highlight_line = false,
                },
            },
        }
    end,
}
