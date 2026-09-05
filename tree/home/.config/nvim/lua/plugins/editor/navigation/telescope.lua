local ignore_patterns = {
  "node_modules",
  "target/", -- 锚成目录段，避免误伤含 target 的文件名
  "%.venv/",
  "%.git/", -- %. 转义点，/ 防 .github/.gitignore 被误杀
  -- ↓ 通用缓存噪音
  "/__pycache__/",
  "%.DS_Store", -- macOS
}

return {
  {
    "nvim-telescope/telescope.nvim",
    init = function()
      -- 首次调用通用选择菜单时加载 Telescope；扩展初始化后会替换此入口。
      vim.ui.select = function(...)
        require("telescope").load_extension("ui-select")
        return vim.ui.select(...)
      end
    end,
    dependencies = {
      "nvim-telescope/telescope-ui-select.nvim",
      {
        -- native fzf in C
        "nvim-telescope/telescope-fzf-native.nvim",
        build = "make",
      },
    },
    opts = function()
      local actions = require("telescope.actions")
      local themes = require("telescope.themes")
      local vimgrep_arguments = vim.list_extend(vim.deepcopy(require("telescope.config").values.vimgrep_arguments), {
        "--hidden",
        "--glob",
        "!.git",
      })
      return {
        defaults = {
          vimgrep_arguments = vimgrep_arguments,
          default_mappings = {
            n = {
              ["?"] = actions.which_key,
              ["q"] = actions.close,
              ["<Tab>"] = actions.move_selection_next,
              ["<S-Tab>"] = actions.move_selection_previous,
              ["<Esc>"] = actions.close,
              ["j"] = actions.move_selection_next,
              ["k"] = actions.move_selection_previous,
              ["<CR>"] = actions.select_default,
              ["l"] = actions.select_default,
              ["gg"] = actions.move_to_top,
              ["G"] = actions.move_to_bottom,
              ["<C-u>"] = actions.preview_scrolling_up,
              ["<C-d>"] = actions.preview_scrolling_down,
              ["<C-Up>"] = actions.cycle_history_prev,
              ["<C-Down>"] = actions.cycle_history_prev,
              ["<PageUp>"] = actions.results_scrolling_up,
              ["<PageDown>"] = actions.results_scrolling_down,
              ["m"] = actions.center,
              ["f"] = actions.to_fuzzy_refine,
            },
            i = {
              ["<Tab>"] = actions.move_selection_next,
              ["<S-Tab>"] = actions.move_selection_previous,
              ["<CR>"] = actions.select_default,
              ["<Up>"] = actions.move_selection_previous,
              ["<Down>"] = actions.move_selection_next,
              ["<C-u>"] = actions.preview_scrolling_up,
              ["<C-d>"] = actions.preview_scrolling_down,
              ["<C-Up>"] = actions.cycle_history_prev,
              ["<C-Down>"] = actions.cycle_history_prev,
              ["<PageUp>"] = actions.results_scrolling_up,
              ["<PageDown>"] = actions.results_scrolling_down,
            },
          },
        },
        pickers = {
          current_buffer_fuzzy_find = {
            sorting_strategy = "ascending",
          },
          buffers = {
            initial_mode = "normal",
            mappings = {
              n = {
                ["<Tab>"] = actions.toggle_selection + actions.move_selection_next,
                ["<S-Tab>"] = actions.toggle_selection + actions.move_selection_previous,
                ["d"] = actions.delete_buffer,
                ["<leader>"] = actions.toggle_selection,
                ["l"] = actions.select_default,
                ["v"] = actions.select_vertical,
                ["s"] = actions.select_horizontal,
              },
            },
            dynamic_preview_title = true,
            layout_config = {
              preview_width = 0.6,
            },
          },
          marks = {
            initial_mode = "normal",
            mappings = {
              n = {
                ["d"] = actions.delete_mark,
                ["<leader>"] = actions.toggle_selection,
                ["l"] = actions.select_default,
                ["v"] = actions.select_vertical,
                ["s"] = actions.select_horizontal,
              },
            },
            layout_config = {
              preview_width = 0.6,
            },
          },
          find_files = {
            file_ignore_patterns = ignore_patterns,
          },
          live_grep = {
            file_ignore_patterns = ignore_patterns,
          },
          grep_string = {
            initial_mode = "normal",
            file_ignore_patterns = ignore_patterns,
          },
          lsp_document_symbols = {
            layout_config = {
              preview_width = 0.7,
            },
          },
          lsp_dynamic_workspace_symbols = {
            layout_config = {
              preview_width = 0.6,
            },
          },
          oldfiles = {
            initial_mode = "normal",
          },
          jumplist = {
            initial_mode = "normal",
          },
          colorscheme = {
            initial_mode = "normal",
          },
          registers = {
            initial_mode = "normal",
          },
        },
        extensions = {
          ["ui-select"] = {
            themes.get_dropdown({}),
          },
          -- native C fzf
          fzf = {
            fuzzy = true,
            override_generic_sorter = true,
            override_file_sorter = true,
            case_mode = "smart_case",
          },
        },
      }
    end,
    config = function(_, opts)
      local telescope = require("telescope")
      telescope.setup(opts)
      telescope.load_extension("ui-select")
      -- native C fzf
      pcall(telescope.load_extension, "fzf")
    end,
    keys = function()
      return {
        {
          "f",
          function()
            require("telescope.builtin").current_buffer_fuzzy_find()
          end,
          desc = "search in current buffer",
        },
        {
          "<leader>s:",
          function()
            require("telescope.builtin").command_history()
          end,
          desc = "Command History",
        },
        {
          "<leader>sw",
          function()
            require("telescope.builtin").grep_string()
          end,
          mode = { "n", "v" },
          desc = "search current word",
        },
        {
          "<leader>b",
          function()
            require("telescope.builtin").buffers({
              sort_mru = true,
              sort_lastused = true,
              ignore_current_buffer = true,
            })
          end,
          desc = "Buffers",
        },
        {
          "<leader>sa",
          function()
            require("telescope.builtin").autocommands()
          end,
          desc = "Auto Commands",
        },
        {
          "<leader>sc",
          function()
            require("telescope.builtin").commands()
          end,
          desc = "Commands",
        },
        {
          "<leader>sk",
          function()
            require("telescope.builtin").keymaps()
          end,
          desc = "Keymaps",
        },
        {
          "<leader>sf",
          function()
            require("telescope.builtin").filetypes()
          end,
          desc = "Change Current Filetypes",
        },
        {
          "<leader>sm",
          function()
            require("telescope.builtin").marks()
          end,
          desc = "Marks",
        },
        {
          "<leader>so",
          function()
            require("telescope.builtin").vim_options()
          end,
          desc = "Options",
        },
        {
          "<leader>sh",
          function()
            require("telescope.builtin").oldfiles()
          end,
          desc = "History Files",
        },
        {
          "<leader>sr",
          function()
            require("telescope.builtin").registers()
          end,
          desc = "registers",
        },
        {
          "<leader>sj",
          function()
            require("telescope.builtin").jumplist({
              trim_text = true,
            })
          end,
          desc = "Jump List",
        },
        {
          "<leader>sC",
          function()
            require("telescope.builtin").colorscheme({ enable_preview = true, ignore_builtins = true })
          end,
          desc = "Colorscheme",
        },
      }
    end,
  },
}
