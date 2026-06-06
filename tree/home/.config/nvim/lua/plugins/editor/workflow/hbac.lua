-- hbac.nvim - 自动关闭不活跃的 buffer，保持 buffer 列表干净
-- 编辑过或停留较久的 buffer 会被 pin，其余超出阈值后自动关闭
return {
    "axkirillov/hbac.nvim",
    event = "VeryLazy",
    opts = {
        autoclose = true,
        threshold = 8,
    },
}
