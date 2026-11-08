-- Quickshell / QML：语法高亮、Qt 6 语言服务与格式化。
return {
  {
    "nvim-treesitter/nvim-treesitter",
    -- 不写死在这里的话，qmljs.so 就只能靠手动 :TSInstall，换机即失。
    ---@module "nvim-treesitter"
    ---@type TSConfig | {ensure_installed: string[]}
    opts = { ensure_installed = { "qmljs", "qmldir" } },
  },

  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        qmlls = {
          -- mason 没有 qmlls 包，走系统 Qt。
          mason = false,
          -- Arch 的 qt6-declarative 把 qmlls 装进 /usr/lib/qt6/bin 且不进 PATH，
          -- lspconfig 自带的 cmd = {'qmlls'} 起不来，必须给绝对路径。
          -- -E：额外读取 QML_IMPORT_PATH 中的 import 路径。
          cmd = { "/usr/lib/qt6/bin/qmlls", "-E" },
          -- 默认只有 .git。QuickShell 配置目录未必是独立 repo，
          -- 补上 qmlls 自己的配置文件和入口文件作为标记。
          root_markers = { ".qmlls.ini", "shell.qml", ".git" },
        },
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
        qml = { "qmlformat" },
      },
      formatters = {
        qmlformat = {
          command = "/usr/lib/qt6/bin/qmlformat",
        },
      },
    },
  },
}
