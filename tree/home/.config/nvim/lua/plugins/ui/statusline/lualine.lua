local colors = {
    black = "#1a1b26",
    white = "#c0caf5",
    red = "#f7768e",
    green = "#9ece6a",
    blue = "#7aa2f7",
    yellow = "#e0af68",
    purple = "#9d7cd8",
    gray = "#a9b1d6",
    cyan = "#10b3cc",
    darkgray = "#2a2e3f",
    lightgray = "#3b4261",
    inactivegray = "#414868",
}

local my_theme = {
    normal = {
        a = { bg = colors.blue, fg = colors.black, gui = "bold" },
        b = { bg = colors.lightgray, fg = colors.white },
        c = { bg = colors.darkgray, fg = colors.gray },
        y = { bg = colors.lightgray, fg = colors.white },
    },
    insert = {
        a = { bg = colors.red, fg = colors.black, gui = "bold" },
        b = { bg = colors.lightgray, fg = colors.white },
        c = { bg = colors.darkgray, fg = colors.white },
        y = { bg = colors.lightgray, fg = colors.red },
    },
    visual = {
        a = { bg = colors.purple, fg = colors.black, gui = "bold" },
        b = { bg = colors.lightgray, fg = colors.white },
        c = { bg = colors.darkgray, fg = colors.white },
        y = { bg = colors.lightgray, fg = colors.purple },
    },
    replace = {
        a = { bg = colors.red, fg = colors.black, gui = "bold" },
        b = { bg = colors.lightgray, fg = colors.white },
        c = { bg = colors.darkgray, fg = colors.white },
    },
    command = {
        a = { bg = colors.yellow, fg = colors.black, gui = "bold" },
        b = { bg = colors.lightgray, fg = colors.white },
        c = { bg = colors.darkgray, fg = colors.white },
        y = { bg = colors.lightgray, fg = colors.yellow },
    },
    inactive = {
        a = { bg = colors.darkgray, fg = colors.gray, gui = "bold" },
        b = { bg = colors.darkgray, fg = colors.gray },
        c = { bg = colors.darkgray, fg = colors.gray },
    },
    terminal = {
        a = { bg = colors.green, fg = colors.black, gui = "bold" },
        b = { bg = colors.lightgray, fg = colors.white },
        c = { bg = colors.darkgray, fg = colors.white },
        y = { bg = colors.lightgray, fg = colors.green },
    },
}

-- 获取根目录与 cwd 的关系,返回目录名或 nil
local function dirname()
    local cwd = utils.path.cwd()
    local root = utils.path.get_root({ normalize = true })
    local name = vim.fs.basename(root)

    if root == cwd then
        -- 根目录即为 cwd
        return name
    elseif root and cwd and root:find(cwd, 1, true) == 1 then
        -- 根目录是 cwd 的子目录
        return name
    elseif root and cwd and cwd:find(root, 1, true) == 1 then
        -- 根目录是 cwd 的父目录
        return name
    else
        -- 根目录与 cwd 无关
        return name
    end
end

return {
    "nvim-lualine/lualine.nvim",
    -- event = "VeryLazy",
    event = function()
        return { "BufReadPost", "BufWritePost", "BufNewFile" }
    end,
    init = function()
        vim.g.lualine_laststatus = vim.o.laststatus
        if vim.fn.argc(-1) > 0 then
            -- set an empty statusline till lualine loads
            vim.o.statusline = " "
        else
            -- hide the statusline on the starter page
            vim.o.laststatus = 0
        end
    end,
    opts = function()
        vim.o.laststatus = vim.g.lualine_laststatus

        local symbols = require("trouble").statusline({
            mode = "symbols",
            groups = {},
            title = false,
            filter = { range = true },
            format = "{kind_icon}{symbol.name:Normal}",
            hl_group = "lualine_c_normal",
        })

        local opts = {
            options = {
                theme = my_theme,
                globalstatus = vim.o.laststatus == 3,
                disabled_filetypes = {
                    statusline = { "dashboard", "alpha", "ministarter", "snacks_dashboard" },
                },

                section_separators = { left = "", right = "" },
                component_separators = { left = "", right = "" },
            },

            sections = {
                lualine_a = { { "mode" } },

                lualine_b = {
                    {
                        function()
                            return "󱉭 " .. dirname()
                        end,
                        cond = function()
                            return true
                        end,
                        color = function()
                            return { fg = Snacks.util.color("Directory") }
                        end,
                    },

                    {
                        "branch",
                        color = function()
                            return { fg = Snacks.util.color("Identifier") }
                        end,
                    },
                },

                lualine_c = {
                    { utils.lualine.pretty_path(), separator = "" },

                    { "filetype", icon_only = true, padding = { left = 0, right = 0 } },

                    -- deprecated 使用 dropbar 替代
                    -- trouble 提供的当前 symbol
                    -- {
                    --     symbols and symbols.get,
                    --
                    --     cond = function()
                    --         return symbols.has()
                    --     end,
                    -- },

                    {
                        "diagnostics",

                        symbols = {
                            error = " ",
                            warn = " ",
                            hint = " ",
                            info = " ",
                        },
                    },
                },

                lualine_x = {
                    -- Copilot 状态
                    utils.lualine.status(utils.icons.kinds.Copilot, function()
                        local clients = package.loaded["copilot"]
                                and vim.lsp.get_clients({ name = "copilot", bufnr = 0 })
                            or {}
                        if #clients > 0 then
                            local status = require("copilot.status").data.status
                            return (status == "InProgress" and "pending") or (status == "Warning" and "error") or "ok"
                        end
                    end),

                    { "searchcount", color = { fg = colors.cyan } },
                    -- 当前命令提示
                    {
                        function()
                            return require("noice").api.status.command.get()
                        end,
                        cond = function()
                            return package.loaded["noice"] and require("noice").api.status.command.has()
                        end,
                        color = function()
                            return { fg = Snacks.util.color("Statement") }
                        end,
                    },

                    -- nvim-dap 插件：显示当前调试状态
                    {
                        function()
                            return "  " .. require("dap").status()
                        end,
                        cond = function()
                            return package.loaded["dap"] and require("dap").status() ~= ""
                        end,
                        color = function()
                            return { fg = Snacks.util.color("Debug") }
                        end,
                    },

                    -- Lazy 插件：有插件可更新时显示更新数量
                    -- {
                    --     require("lazy.status").updates,
                    --     cond = require("lazy.status").has_updates,
                    --     color = function()
                    --         return { fg = Snacks.util.color("Special") }
                    --     end,
                    -- },
                    --

                    {
                        "diff",
                        symbols = {
                            added = " ",
                            modified = " ",
                            removed = " ",
                        },
                        -- 使用 gitsigns 插件提供的缓存 diff 数据源
                        source = function()
                            local gitsigns = vim.b.gitsigns_status_dict
                            if gitsigns then
                                return {
                                    added = gitsigns.added,
                                    modified = gitsigns.changed,
                                    removed = gitsigns.removed,
                                }
                            end
                        end,
                    },
                },

                lualine_y = {
                    {
                        "encoding",
                        separator = "",
                        padding = { left = 1, right = 1 },
                        color = { fg = colors.blue, gui = "italic,bold" },
                    },
                    {
                        "filesize",
                        color = { fg = colors.purple, gui = "italic,bold" },
                    },
                },

                lualine_z = {
                    {
                        "selectioncount",
                        separator = " ",
                        padding = { left = 1, right = 1 },
                        color = { fg = colors.white, gui = "bold" },
                    },
                    {
                        "progress",
                        separator = " ",
                        padding = { left = 0, right = 1 },
                        color = { fg = colors.black, cterm = "italic,bold", gui = "italic,bold" },
                    },
                    {
                        "location",
                        padding = { left = 0, right = 1 },
                        color = { fg = colors.black, gui = "italic,bold" },
                    },
                },
            },
            extensions = { "overseer", "trouble", "toggleterm", "mason", "lazy", "fzf" },
        }

        return opts
    end,
}
