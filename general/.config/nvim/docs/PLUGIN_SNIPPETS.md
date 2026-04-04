# 快速插件安装代码片段

## 📦 即用型插件配置

将以下配置直接复制到对应的 `lua/plugins/` 目录下。

---

## 核心增强插件

### lua/plugins/editor/surround.lua
```lua
return {
    "kylechui/nvim-surround",
    version = "*",
    event = "VeryLazy",
    config = function()
        require("nvim-surround").setup({
            keymaps = {
                insert = "<C-g>s",
                insert_line = "<C-g>S",
                normal = "ys",
                normal_cur = "yss",
                normal_line = "yS",
                normal_cur_line = "ySS",
                visual = "S",
                visual_line = "gS",
                delete = "ds",
                change = "cs",
                change_line = "cS",
            },
        })
    end,
}
```

### lua/plugins/editor/spider.lua
```lua
return {
    "chrisgrieser/nvim-spider",
    lazy = true,
    opts = {
        skipInsignificantPunctuation = true,
        consistentOperatorPending = false,
        subwordMovement = true,
    },
    keys = {
        {
            "w",
            "<cmd>lua require('spider').motion('w')<CR>",
            mode = { "n", "o", "x" },
            desc = "Spider-w",
        },
        {
            "e",
            "<cmd>lua require('spider').motion('e')<CR>",
            mode = { "n", "o", "x" },
            desc = "Spider-e",
        },
        {
            "b",
            "<cmd>lua require('spider').motion('b')<CR>",
            mode = { "n", "o", "x" },
            desc = "Spider-b",
        },
    },
}
```

### lua/plugins/editor/spectre.lua
```lua
return {
    "nvim-pack/nvim-spectre",
    build = false,
    cmd = "Spectre",
    opts = { open_cmd = "noswapfile vnew" },
    keys = {
        {
            "<leader>sr",
            function()
                require("spectre").toggle()
            end,
            desc = "Replace in Files (Spectre)",
        },
        {
            "<leader>sw",
            function()
                require("spectre").open_visual({ select_word = true })
            end,
            desc = "Search current word",
        },
        {
            "<leader>sw",
            function()
                require("spectre").open_visual()
            end,
            mode = "v",
            desc = "Search current word",
        },
        {
            "<leader>sp",
            function()
                require("spectre").open_file_search({ select_word = true })
            end,
            desc = "Search on current file",
        },
    },
}
```

---

## UI 增强插件

### lua/plugins/ui/indent-blankline.lua
```lua
return {
    "lukas-reineke/indent-blankline.nvim",
    event = "LazyFile",
    opts = {
        indent = {
            char = "│",
            tab_char = "│",
        },
        scope = {
            enabled = true,
            show_start = false,
            show_end = false,
        },
        exclude = {
            filetypes = {
                "help",
                "alpha",
                "dashboard",
                "neo-tree",
                "Trouble",
                "trouble",
                "lazy",
                "mason",
                "notify",
                "toggleterm",
                "lazyterm",
            },
        },
    },
    main = "ibl",
}
```

### lua/plugins/ui/scrollbar.lua
```lua
return {
    "petertriho/nvim-scrollbar",
    event = "BufReadPost",
    opts = {
        handle = {
            color = "#3b4261",
        },
        marks = {
            Search = { color = "#ff9e64" },
            Error = { color = "#f7768e" },
            Warn = { color = "#e0af68" },
            Info = { color = "#0db9d7" },
            Hint = { color = "#1abc9c" },
            Misc = { color = "#9d7cd8" },
        },
        excluded_filetypes = {
            "prompt",
            "TelescopePrompt",
            "noice",
            "neo-tree",
            "dashboard",
            "alpha",
            "lazy",
            "mason",
            "DressingInput",
        },
        handlers = {
            gitsigns = true,
            search = true,
        },
    },
}
```

### lua/plugins/ui/dropbar.lua
```lua
return {
    "Bekaboo/dropbar.nvim",
    event = "VeryLazy",
    opts = {
        general = {
            enable = function(buf, win)
                return vim.fn.win_gettype(win) == ""
                    and vim.bo[buf].buftype == ""
                    and not vim.api.nvim_win_get_config(win).zindex
            end,
        },
        icons = {
            kinds = {
                symbols = LazyVim.config.icons.kinds,
            },
        },
    },
}
```

### lua/plugins/ui/navic.lua
```lua
return {
    "SmiteshP/nvim-navic",
    lazy = true,
    init = function()
        vim.g.navic_silence = true
        LazyVim.lsp.on_attach(function(client, buffer)
            if client.supports_method("textDocument/documentSymbol") then
                require("nvim-navic").attach(client, buffer)
            end
        end)
    end,
    opts = function()
        return {
            separator = " 󰁔 ",
            highlight = true,
            depth_limit = 5,
            icons = LazyVim.config.icons.kinds,
            lazy_update_context = true,
        }
    end,
}
```

---

## 开发工具插件

### lua/plugins/editor/refactoring.lua
```lua
return {
    "ThePrimeagen/refactoring.nvim",
    dependencies = {
        "nvim-lua/plenary.nvim",
        "nvim-treesitter/nvim-treesitter",
    },
    event = "BufRead",
    config = function()
        require("refactoring").setup({
            prompt_func_return_type = {
                go = false,
                java = false,
                cpp = false,
                c = false,
                h = false,
                hpp = false,
                cxx = false,
            },
            prompt_func_param_type = {
                go = false,
                java = false,
                cpp = false,
                c = false,
                h = false,
                hpp = false,
                cxx = false,
            },
            printf_statements = {},
            print_var_statements = {},
        })
    end,
    keys = {
        {
            "<leader>re",
            function()
                require("refactoring").refactor("Extract Function")
            end,
            mode = "x",
            desc = "Extract Function",
        },
        {
            "<leader>rf",
            function()
                require("refactoring").refactor("Extract Function To File")
            end,
            mode = "x",
            desc = "Extract Function To File",
        },
        {
            "<leader>rv",
            function()
                require("refactoring").refactor("Extract Variable")
            end,
            mode = "x",
            desc = "Extract Variable",
        },
        {
            "<leader>ri",
            function()
                require("refactoring").refactor("Inline Variable")
            end,
            mode = { "n", "x" },
            desc = "Inline Variable",
        },
        {
            "<leader>rI",
            function()
                require("refactoring").refactor("Inline Function")
            end,
            mode = "n",
            desc = "Inline Function",
        },
        {
            "<leader>rb",
            function()
                require("refactoring").refactor("Extract Block")
            end,
            desc = "Extract Block",
        },
        {
            "<leader>rbf",
            function()
                require("refactoring").refactor("Extract Block To File")
            end,
            desc = "Extract Block To File",
        },
    },
}
```

### lua/plugins/editor/project.lua
```lua
return {
    "ahmedkhalf/project.nvim",
    opts = {
        detection_methods = { "pattern", "lsp" },
        patterns = { ".git", "Makefile", "package.json", "Cargo.toml", "go.mod", "pyproject.toml" },
        silent_chdir = false,
        scope_chdir = "global",
    },
    event = "VeryLazy",
    config = function(_, opts)
        require("project_nvim").setup(opts)
        LazyVim.on_load("telescope.nvim", function()
            require("telescope").load_extension("projects")
        end)
    end,
    keys = {
        { "<leader>fp", "<Cmd>Telescope projects<CR>", desc = "Projects" },
    },
}
```


### lua/plugins/editor/ts-node-action.lua
```lua
return {
    "ckolkey/ts-node-action",
    dependencies = { "nvim-treesitter" },
    opts = {},
    keys = {
        {
            "<leader>cn",
            function()
                require("ts-node-action").node_action()
            end,
            desc = "Trigger Node Action",
        },
    },
}
```

### lua/plugins/tools/rest.lua
```lua
return {
    "rest-nvim/rest.nvim",
    ft = "http",
    dependencies = { "nvim-lua/plenary.nvim" },
    config = function()
        require("rest-nvim").setup({
            result_split_horizontal = false,
            result_split_in_place = false,
            skip_ssl_verification = false,
            encode_url = true,
            highlight = {
                enabled = true,
                timeout = 150,
            },
            result = {
                show_url = true,
                show_http_info = true,
                show_headers = true,
                formatters = {
                    json = "jq",
                    html = function(body)
                        return vim.fn.system({ "tidy", "-i", "-q", "-" }, body)
                    end,
                },
            },
        })
    end,
    keys = {
        { "<leader>rr", "<Plug>RestNvim", desc = "Run REST request", ft = "http" },
        { "<leader>rp", "<Plug>RestNvimPreview", desc = "Preview REST request", ft = "http" },
        { "<leader>rl", "<Plug>RestNvimLast", desc = "Re-run last REST request", ft = "http" },
    },
}
```

---

## 实用杂项插件

### lua/plugins/editor/matchup.lua
```lua
return {
    "andymass/vim-matchup",
    event = "BufReadPost",
    config = function()
        vim.g.matchup_matchparen_offscreen = { method = "popup" }
        vim.g.matchup_matchparen_deferred = 1
        vim.g.matchup_matchparen_hi_surround_always = 1
    end,
}
```

### lua/plugins/editor/sleuth.lua
```lua
return {
    "tpope/vim-sleuth",
    event = "BufReadPre",
}
```

