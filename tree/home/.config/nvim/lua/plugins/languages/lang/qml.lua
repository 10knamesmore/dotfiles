-- QML / Qt Quick（QuickShell 配置就是这个）
--
-- 三个零件本体/生态都已现成，之前只是没接线：
--   * filetype  —— nvim 自带 .qml → qml，且 ftplugin/qml.vim 已把 commentstring 设成 // %s
--                  （所以 ts-comments 不用登记 qml）
--   * treesitter —— qmljs 是 tier 2 parser，nvim-treesitter 的 filetypes 表已映射好 qml → qmljs
--   * formatter  —— conform 内置 qmlformat 定义，Arch 的 qt6-declarative 把它装到了 /usr/bin
-- 唯一需要人工干预的是 qmlls 的路径，见下。
return {
  {
    "nvim-treesitter/nvim-treesitter",
    -- 不写死在这里的话，qmljs.so 就只能靠手动 :TSInstall，换机即失。
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
          -- -E：从 QML_IMPORT_PATH 读 import 路径，否则 `import Quickshell` 全是 unresolved。
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
    opts = {
      formatters_by_ft = {
        qml = { "qmlformat" },
      },
    },
  },
}
