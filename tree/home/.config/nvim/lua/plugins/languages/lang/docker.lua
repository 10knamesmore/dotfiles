return {
  {
    "nvim-treesitter/nvim-treesitter",
    opts = { ensure_installed = { "dockerfile" } },
  },
  {
    "mason.nvim",
    opts = { ensure_installed = { "hadolint" } },
  },
  {
    "nvimtools/none-ls.nvim",
    optional = true,
    opts = function(_, opts)
      local nls = require("null-ls")
      opts.sources = vim.list_extend(opts.sources or {}, {
        nls.builtins.diagnostics.hadolint,
      })
    end,
  },
  {
    "mfussenegger/nvim-lint",
    optional = true,
    opts = {
      linters_by_ft = {
        dockerfile = { "hadolint" },
      },
    },
  },
  {
    "neovim/nvim-lspconfig",
    -- docker_compose_language_service 的 filetypes 只有 "yaml.docker-compose"，
    -- 而 nvim 对 compose 文件本体只判到 "yaml" —— 不登记这个复合 ft，server 装了也永不 attach。
    -- 复合 ft 是安全的：yamlls 的 filetypes 已含 yaml.docker-compose，
    -- treesitter 的 get_lang("yaml.docker-compose") 也照样解析成 yaml。
    init = function()
      vim.filetype.add({
        pattern = {
          ["[Dd]ocker%-?[Cc]ompose%.ya?ml"] = "yaml.docker-compose",
          ["[Dd]ocker%-?[Cc]ompose%..*%.ya?ml"] = "yaml.docker-compose",
          ["compose%.ya?ml"] = "yaml.docker-compose",
          ["compose%..*%.ya?ml"] = "yaml.docker-compose",
        },
      })
    end,
    opts = {
      servers = {
        dockerls = {},
        docker_compose_language_service = {},
      },
    },
  },
}
