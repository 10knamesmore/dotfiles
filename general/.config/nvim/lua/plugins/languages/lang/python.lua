local default_type_check_mode = "standard"
local python_root_markers = {
    "pyproject.toml",
    "ruff.toml",
    ".ruff.toml",
    "setup.py",
    "setup.cfg",
    ".git",
}
local warned_invalid_type_check_mode = false

local python_inlay_hints = {
    variableTypes = true,
    functionReturnTypes = true,
    callArgumentNames = true,
}

local valid_type_check_modes = {
    off = true,
    basic = true,
    standard = true,
    strict = true,
}

---@alias PythonTypeCheckMode "off"|"basic"|"standard"|"strict"
---@class PythonAnalysisSettings
---@field autoImportCompletions boolean
---@field useLibraryCodeForTypes boolean
---@field typeCheckingMode PythonTypeCheckMode
---@field inlayHints table<string, boolean>

--- 当 `vim.g.python_type_check_mode` 非法时，仅提示一次并回退到默认值。
---@param mode string|nil 用户配置的检查强度值
local function warn_invalid_type_check_mode(mode)
    if warned_invalid_type_check_mode or mode == nil then
        return
    end

    warned_invalid_type_check_mode = true
    vim.schedule(function()
        vim.notify(
            string.format(
                "Invalid vim.g.python_type_check_mode=%q. Falling back to %q.",
                tostring(mode),
                default_type_check_mode
            ),
            vim.log.levels.WARN
        )
    end)
end

--- 解析 basedpyright 使用的类型检查强度。
---@return PythonTypeCheckMode
local function get_type_check_mode()
    local mode = vim.g.python_type_check_mode
    if type(mode) == "string" and valid_type_check_modes[mode] then
        return mode
    end

    warn_invalid_type_check_mode(mode)
    return default_type_check_mode
end

--- 构造 basedpyright 的 Python 分析配置。
---@return PythonAnalysisSettings
local function get_python_analysis_settings()
    return {
        autoImportCompletions = true,
        useLibraryCodeForTypes = true,
        typeCheckingMode = get_type_check_mode(),
        inlayHints = vim.deepcopy(python_inlay_hints),
    }
end

--- 从项目根目录下常见虚拟环境目录中查找 Python 解释器。
---@return string|nil
local function get_project_python()
    local root = vim.fs.root(0, python_root_markers) or vim.uv.cwd()
    if not root then
        return nil
    end

    for _, dirname in ipairs({ ".venv", "venv", "env", ".env" }) do
        local candidate = vim.fs.joinpath(root, dirname, "bin", "python")
        if vim.fn.executable(candidate) == 1 then
            return candidate
        end
    end
end

--- 为调试和测试解析当前应使用的 Python 解释器。
---@return string
local function resolve_python()
    local ok, venv_selector = pcall(require, "venv-selector")
    if ok then
        local python = venv_selector.python()
        if type(python) == "string" and python ~= "" and vim.fn.executable(python) == 1 then
            return python
        end
    end

    local project_python = get_project_python()
    if project_python then
        return project_python
    end

    return vim.fn.exepath("python3") ~= "" and vim.fn.exepath("python3") or "python3"
end

return {
    {
        "nvim-treesitter/nvim-treesitter",
        opts = { ensure_installed = { "ninja", "rst" } },
    },

    {
        "neovim/nvim-lspconfig",
        opts = {
            servers = {
                pyright = { enabled = false },
                ruff_lsp = { enabled = false },
                basedpyright = {
                    settings = {
                        basedpyright = {
                            disableOrganizeImports = true,
                        },
                        python = {
                            analysis = get_python_analysis_settings(),
                        },
                    },
                },
                ruff = {
                    init_options = {
                        settings = {
                            logLevel = "error",
                        },
                    },
                    keys = {
                        {
                            "<leader>co",
                            utils.lsp.action["source.organizeImports"],
                            desc = "Organize Imports",
                        },
                        {
                            "<leader>cD",
                            utils.lsp.action["source.fixAll"],
                            desc = "Fix All Diagnostics",
                        },
                    },
                },
            },
            setup = {
                ruff = function()
                    Snacks.util.lsp.on({ name = "ruff" }, function(_, client)
                        client.server_capabilities.hoverProvider = false
                    end)
                end,
            },
        },
    },

    {
        "mason-org/mason.nvim",
        optional = true,
        opts = function(_, opts)
            opts.ensure_installed = opts.ensure_installed or {}
            if not vim.tbl_contains(opts.ensure_installed, "debugpy") then
                table.insert(opts.ensure_installed, "debugpy")
            end
        end,
    },

    {
        "stevearc/conform.nvim",
        optional = true,
        opts = {
            formatters_by_ft = {
                python = { "ruff_organize_imports", "ruff_format" },
            },
        },
    },

    {
        "nvim-neotest/neotest",
        optional = true,
        dependencies = {
            "nvim-neotest/neotest-python",
        },
        opts = {
            adapters = {
                ["neotest-python"] = {},
            },
        },
    },

    {
        "mfussenegger/nvim-dap",
        optional = true,
        dependencies = {
            "mfussenegger/nvim-dap-python",
            keys = {
                {
                    "<leader>dPt",
                    function()
                        require("dap-python").test_method()
                    end,
                    desc = "Debug Method",
                    ft = "python",
                },
                {
                    "<leader>dPc",
                    function()
                        require("dap-python").test_class()
                    end,
                    desc = "Debug Class",
                    ft = "python",
                },
            },
            config = function()
                local dap_python = require("dap-python")
                dap_python.setup("debugpy-adapter")
                dap_python.resolve_python = resolve_python
            end,
        },
    },

    {
        "linux-cultist/venv-selector.nvim",
        cmd = "VenvSelect",
        ft = "python",
        opts = {
            options = {
                picker = "snacks",
                enable_cached_venvs = true,
                cached_venv_automatic_activation = true,
                require_lsp_activation = true,
                notify_user_on_venv_activation = true,
                override_notify = false,
            },
        },
    },

    {
        "hrsh7th/nvim-cmp",
        optional = true,
        opts = function(_, opts)
            opts.auto_brackets = opts.auto_brackets or {}
            table.insert(opts.auto_brackets, "python")
        end,
    },

    {
        "jay-babu/mason-nvim-dap.nvim",
        optional = true,
        opts = {
            handlers = {
                python = function() end,
            },
        },
    },
}
