return {
  "nvim-mini/mini.align",
  version = false,
  event = { "BufReadPost", "BufWritePost", "BufNewFile" },
  opts = function()
    -- 读取用户输入的一行文本；取消（<Esc>/<C-c>）或直接回车时返回 nil，表示不改变当前设置
    local read_input = function(prompt)
      local ok, input = pcall(vim.fn.input, { prompt = "(mini.align) " .. prompt .. ": " })
      if not ok or input == nil or input == "" then
        return nil
      end
      return input
    end

    -- No need to copy this inside `setup()`. Will be used automatically.
    local opts = {
      -- Module mappings. Use `''` (empty string) to disable one.
      mappings = {
        start = "",
        start_with_preview = "ga",
      },

      modifiers = {
        -- 按原样匹配输入的字符或字符序列（如 "=>"、"::"、"--"）
        -- 需要 Lua pattern 时用大写 'S'；内置 's' 会把输入当 pattern，输入 ( [ % 会报错，输入 . 匹配任意字符
        s = function(_, opts)
          local input = read_input("输入要对齐的字符序列")
          if input == nil then
            return
          end
          opts.split_pattern = vim.pesc(input)
        end,

        S = function(_, opts)
          local input = read_input("输入 split Lua pattern")
          if input == nil then
            return
          end
          opts.split_pattern = input
        end,

        j = function(_, opts)
          local next_side = {
            left = "center",
            center = "right",
            right = "none",
            none = "left",
          }
          opts.justify_side = next_side[opts.justify_side] or "left"
        end,

        [":"] = function(steps, opts)
          opts.split_pattern = ":"
          opts.justify_side = "right"
          table.insert(steps.pre_justify, require("mini.align").gen_step.trim("both"))
          opts.merge_delimiter = " "
        end,
      },
      -- Modifiers changing alignment steps and/or options
      -- modifiers = {
      --   -- Main option modifiers
      --   ['s'] = --<function: enter split pattern>,
      --   ['j'] = --<function: choose justify side>,
      --   ['m'] = --<function: enter merge delimiter>,
      --
      --   -- Modifiers adding pre-steps
      --   ['f'] = --<function: filter parts by entering Lua expression>,
      --   ['i'] = --<function: ignore some split matches>,
      --   ['p'] = --<function: pair parts>,
      --   ['t'] = --<function: trim parts>,
      --
      --   -- Delete some last pre-step
      --   ['<BS>'] = --<function: delete some last pre-step>,
      --
      --   -- Special configurations for common splits
      --   ['='] = --<function: enhanced setup for '='>,
      --   [','] = --<function: enhanced setup for ','>,
      --   ['|'] = --<function: enhanced setup for '|'>,
      --   [' '] = --<function: enhanced setup for ' '>,
      -- },

      -- Default options controlling alignment process
      options = {
        split_pattern = "",
        justify_side = "left",
        merge_delimiter = "",
      },

      -- Default steps performing alignment (if `nil`, default is used)
      steps = {
        pre_split = {},
        split = nil,
        pre_justify = {},
        justify = nil,
        pre_merge = {},
        merge = nil,
      },

      -- Whether to disable showing non-error feedback
      -- This also affects (purely informational) helper messages shown after
      -- idle time if user input is required.
      silent = false,
    }

    return opts
  end,
}
