return {
  "olimorris/codecompanion.nvim",
  version = "^19.0.0",
  cmd = { "CodeCompanion", "CodeCompanionChat", "CodeCompanionActions" },
  keys = {
    { "<leader>aa", "<cmd>CodeCompanionActions<cr>", mode = { "n", "x" }, desc = "Actions" },
    { "<leader>ac", "<cmd>CodeCompanionChat Toggle<cr>", mode = "n", desc = "Toggle Chat" },
    { "<leader>ac", "<cmd>CodeCompanionChat Add<cr>", mode = "x", desc = "Add to Chat" },
  },
  dependencies = {
    "nvim-lua/plenary.nvim",
  },
  opts = {
    interactions = {
      chat = {
        adapter = "codex",
      },
    },
    adapters = {
      acp = {
        codex = function()
          return require("codecompanion.adapters").extend("codex", {
            defaults = {
              auth_method = "chat-gpt",
            },
          })
        end,
      },
    },
  },
}
