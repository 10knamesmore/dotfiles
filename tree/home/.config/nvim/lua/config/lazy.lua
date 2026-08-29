local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not (vim.uv or vim.loop).fs_stat(lazypath) then
  local lazyrepo = "https://github.com/folke/lazy.nvim.git"
  local out = vim.fn.system({ "git", "clone", "--filter=blob:none", "--branch=stable", lazyrepo, lazypath })
  if vim.v.shell_error ~= 0 then
    vim.api.nvim_echo({
      { "Failed to clone lazy.nvim:\n", "ErrorMsg" },
      { out, "WarningMsg" },
      { "\nPress any key to exit..." },
    }, true, {})
    vim.fn.getchar()
    os.exit(1)
  end
end
vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
  spec = {
    -- import/override with your plugins
    -- { import = "plugins" },
    { import = "plugins.ai" },
    { import = "plugins.editor" },
    { import = "plugins.ui" },
    { import = "plugins.languages" },
  },
  defaults = {
    lazy = true,
    version = false, -- always use the latest git commit
    -- version = "*", -- try installing the latest stable version for plugins that support semver
  },
  install = { colorscheme = { "tokyonight", "habamax" } },
  -- 自动检查会在启动路径逐个探测插件并创建 git 子进程；lockfile 已固定版本，
  -- 更新检查由 :Lazy check 显式触发。
  checker = { enabled = false },
  performance = {
    rtp = {
      -- disable some rtp plugins
      disabled_plugins = {
        "gzip",
        -- "matchit",
        -- "matchparen",
        -- 文件浏览由 mini.files/snacks 提供；远端文件和 gx 由 Neovim 内置 runtime 处理。
        "netrwPlugin",
        "tarPlugin",
        "tutor",
        "zipPlugin",
      },
    },
  },
  ui = {
    border = "rounded",
  },
})
