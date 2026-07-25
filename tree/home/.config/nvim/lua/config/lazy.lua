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
  -- 关掉自动检查更新：start() 第一步是同步的 fast_check()，对 85 个插件逐个
  -- Git.info + get_target，实测 ~50ms（比整个 lazy 启动还大一倍）；之后每次启动
  -- 还要 spawn 约 85 个 git 子进程，到期时是 85 个不限并发的 git fetch。
  -- 而 notify=false 让结果只出现在 :Lazy 面板里，等于付了钱没拿货。
  -- 版本由 lazy-lock.json 钉着，需要时手动 :Lazy check。
  checker = { enabled = false },
  performance = {
    rtp = {
      -- disable some rtp plugins
      disabled_plugins = {
        "gzip",
        -- "matchit",
        -- "matchparen",
        -- netrw 注册 47 条 autocmd，而文件浏览已归 mini.files/snacks，
        -- 远端文件在 0.12 由内置 runtime/plugin/net.lua 接管；
        -- gx 也不靠它（来自 vim/_core/defaults.lua 的内置映射）。
        "netrwPlugin",
        "tarPlugin",
        -- "tohtml" 在 0.12 已从 runtime/plugin 移走（改为 pack/dist/opt/nvim.tohtml），
        -- 写在这里是空操作，删掉免得看起来还在生效。
        "tutor",
        "zipPlugin",
      },
    },
  },
  ui = {
    border = "rounded",
  },
})
