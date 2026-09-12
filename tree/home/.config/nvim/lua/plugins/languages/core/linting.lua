-- Linting 核心插件
-- 提供代码检查功能

return {
  -- nvim-lint - 异步调用语言特定的 linter
  {
    "mfussenegger/nvim-lint",
    event = { "BufReadPost", "BufNewFile", "BufWritePre" },
    opts = {
      -- 触发 linter 的事件
      events = { "BufWritePost", "BufReadPost", "InsertLeave" },
      linters_by_ft = {
        fish = { "fish" },
        sh = { "shellcheck" },
        bash = { "shellcheck" },
        yaml = { "yamllint" },
        dockerfile = { "hadolint" },
        -- 使用 "*" 对所有文件类型运行 linter
        -- ['*'] = { 'global linter' },
        -- 使用 "_" 作为没有配置 linter 的文件类型的fallback
        -- ['_'] = { 'fallback linter' },
      },
      -- 自定义 linter 配置
      ---@type table<string,table>
      linters = {
        -- 示例：只在有 selene.toml 文件时使用 selene
        -- selene = {
        --   condition = function(ctx)
        --     return vim.fs.find({ "selene.toml" }, { path = ctx.filename, upward = true })[1]
        --   end,
        -- },
      },
    },
    config = function(_, opts)
      local lint = require("lint")
      for name, linter in pairs(opts.linters) do
        if type(linter) == "table" and type(lint.linters[name]) == "table" then
          lint.linters[name] = vim.tbl_deep_extend("force", lint.linters[name], linter)
          if type(linter.prepend_args) == "table" then
            lint.linters[name].args = lint.linters[name].args or {}
            vim.list_extend(lint.linters[name].args, linter.prepend_args)
          end
        else
          lint.linters[name] = linter
        end
      end
      lint.linters_by_ft = opts.linters_by_ft

      --- 为函数创建防抖包装，避免频繁触发 lint。
      ---@param ms integer
      ---@param fn function
      ---@return function
      local function debounce(ms, fn)
        local timer = vim.uv.new_timer()
        return function()
          timer:start(ms, 0, function()
            timer:stop()
            vim.schedule_wrap(fn)()
          end)
        end
      end

      vim.api.nvim_create_autocmd(opts.events, {
        group = vim.api.nvim_create_augroup("nvim-lint", { clear = true }),
        callback = debounce(100, function()
          lint.try_lint()
        end),
      })
    end,
  },
}
