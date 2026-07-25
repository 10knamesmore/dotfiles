-- mini.icons - 图标支持
return {
  -- 必须和 languages/lang/typescript.lua 里那个 mini.icons fragment 用同一个 URL。
  -- 两处写不同 owner 时 lazy 按 fragment 顺序取最后一个赢；一旦顺序翻转，
  -- git.origin 任务会认为远端变了，fs.clean 掉整个目录重新 clone。
  -- 磁盘上现在是 nvim-mini（新 owner），以它为准。
  "nvim-mini/mini.icons",
  lazy = true,
  opts = {
    file = {
      [".keep"] = { glyph = "󰊢", hl = "MiniIconsGrey" },
      ["devcontainer.json"] = { glyph = "", hl = "MiniIconsAzure" },
    },
    filetype = {
      dotenv = { glyph = "", hl = "MiniIconsYellow" },
    },
  },
  init = function()
    package.preload["nvim-web-devicons"] = function()
      require("mini.icons").mock_nvim_web_devicons()
      return package.loaded["nvim-web-devicons"]
    end
  end,
}
