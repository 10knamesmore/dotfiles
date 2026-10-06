-- log-highlight sets fixed RGB colors during syntax loading; resolve them through the active theme.
vim.api.nvim_set_hl(0, "LogGreen", { link = "DiagnosticOk" })
vim.api.nvim_set_hl(0, "LogBlue", { link = "DiagnosticInfo" })
