-- mini.pairs - 自动配对括号/引号
return {
    "echasnovski/mini.pairs",
    event = "VeryLazy",
    opts = {
        modes = { insert = true, command = true, terminal = false },
        skip_next = [=[[%w%%%'%[%"%.%`%$]]=],
        skip_ts = { "string" },
        skip_unbalanced = true,
        markdown = true,
    },
}
