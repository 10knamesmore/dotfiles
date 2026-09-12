local default_type_check_mode = "standard"
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

return {
  {
    "nvim-treesitter/nvim-treesitter",
    ---@module "nvim-treesitter"
    ---@type TSConfig | {ensure_installed: string[]}
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
    "stevearc/conform.nvim",
    optional = true,
    ---@module "conform"
    ---@type conform.setupOpts
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
}
