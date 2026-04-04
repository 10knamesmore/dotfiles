return {
    "rcarriga/nvim-dap-ui",
    opts = {
        layouts = {
            {
                elements = {
                    {
                        id = "scopes",
                        size = 0.25,
                    },
                    {
                        id = "breakpoints",
                        size = 0.25,
                    },
                    {
                        id = "stacks",
                        size = 0.25,
                    },
                    {
                        id = "watches",
                        size = 0.25,
                    },
                },
                position = "left",
                size = 80,
            },
            {
                elements = {
                    {
                        id = "repl",
                        size = 0.5,
                    },
                    {
                        id = "console",
                        size = 0.5,
                    },
                },
                position = "bottom",
                size = 10,
            },
        },
    },
}
