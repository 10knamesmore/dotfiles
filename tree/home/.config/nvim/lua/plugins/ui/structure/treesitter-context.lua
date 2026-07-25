return {
  "nvim-treesitter/nvim-treesitter-context",
  event = { "BufReadPost", "BufWritePost", "BufNewFile" },
  ---@module "treesitter-context"
  ---@return TSContext.UserConfig
  opts = function()
    return { enable = true, multiwindow = true, mode = "cursor", max_lines = 3 }
  end,
}
