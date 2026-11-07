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
  -- 后加载 options.lua 会覆盖插件 init() 设置的选项。
  require("config.options")
  require("config.keymaps")
  require("config.lazy")
  require("config.autocmds")
end
