return {
    {
        "mason-org/mason.nvim",
        opts = {
            ensure_installed = {
                "tinymist",
            },
        },
    },
    {
        "nvim-treesitter/nvim-treesitter",
        opts = {
            ensure_installed = { "typst" },
        },
    },

    {
        "neovim/nvim-lspconfig",
        opts = {
            servers = {
                tinymist = {
                    keys = {
                        {
                            "<leader>cP",
                            function()
                                local buf_name = vim.api.nvim_buf_get_name(0)
                                local file_name = vim.fn.fnamemodify(buf_name, ":t")
                                utils.lsp.execute({
                                    command = "tinymist.pinMain",
                                    arguments = { buf_name },
                                })
                                utils.info("Tinymist: Pinned " .. file_name)
                            end,
                            desc = "Pin main file",
                        },
                    },
                    single_file_support = true, -- Fixes LSP attachment in non-Git directories
                    settings = {
                        formatterMode = "typstyle",
                        projectResolution = "lockDatabase",
                        lint = {
                            enable = true,
                        },
                    },
                },
            },
        },
    },

    {
        "stevearc/conform.nvim",
        optional = true,
        opts = {
            formatters_by_ft = {
                typst = { "typstyle", lsp_format = "prefer" },
            },
        },
    },

    {
        "chomosuke/typst-preview.nvim",
        cmd = { "TypstPreview", "TypstPreviewToggle", "TypstPreviewUpdate" },
        keys = {
            {
                "<leader>cp",
                ft = "typst",
                "<cmd>TypstPreviewToggle<cr>",
                desc = "Toggle Typst Preview",
            },
        },
        opts = {
            dependencies_bin = {
                tinymist = "tinymist",
            },
            get_main_file = function(path_of_buffer)
                local bufdir = vim.fs.dirname(path_of_buffer)

                -- 先找到项目根目录
                local root_markers = vim.fs.find({ "typst.toml", ".git" }, { path = bufdir, upward = true })

                local search_root = bufdir
                if #root_markers > 0 then
                    search_root = vim.fs.dirname(root_markers[1])
                end

                -- 在项目根目录内向上查找 main.typ
                local main_files = vim.fs.find("main.typ", {
                    path = bufdir,
                    upward = true,
                    stop = vim.fs.dirname(search_root), -- 到项目根的父目录停止
                    type = "file",
                })

                if #main_files > 0 then
                    return main_files[1]
                end
                return path_of_buffer
            end,
        },
    },

    {
        "folke/ts-comments.nvim",
        opts = {
            lang = {
                typst = { "// %s", "/* %s */" },
            },
        },
    },
}
