_G.utils = require("utils")
vim.g.mapleader = " "
vim.g.maplocalleader = "\\"

if vim.g.vscode then
  require("code.config.options")
  require("code.config.keymaps")
else
  if vim.g.neovide then
    require("config.neovide").setup()
  end
  -- options 必须早于 lazy：lazy.setup() 内部同步跑所有插件的 init()，
  -- 那里设的选项会被后执行的 options.lua 静默回滚（ufo 的 foldcolumn 曾这么丢过）。
  require("config.options")
  require("config.keymaps")
  require("config.lazy")
  require("config.autocmds")
end
