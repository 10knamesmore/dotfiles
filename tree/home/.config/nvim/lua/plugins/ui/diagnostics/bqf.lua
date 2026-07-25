-- nvim-bqf - 增强 quickfix 窗口：预览、模糊搜索、签名操作
return {
  "kevinhwang91/nvim-bqf",
  ft = "qf",
  ---@module "bqf"
  ---@type BqfConfig
  ---@diagnostic disable-next-line: missing-fields
  opts = {
    ---@diagnostic disable-next-line: missing-fields
    preview = {
      winblend = 0,
    },
  },
}
