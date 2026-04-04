_G.utils = require("utils")
vim.g.mapleader = " "
vim.g.maplocalleader = "\\"
vim.g.copilot_enabled = false

if vim.g.vscode then
    require("code.config.options")
    require("code.config.keymaps")
else
    if vim.g.neovide then
        require("config.neovide").setup()
    end
    require("config.lazy")
    require("config.keymaps")
    require("config.options")
    require("config.autocmds")
end
