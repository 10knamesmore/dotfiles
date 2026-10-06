return {
  {
    "folke/tokyonight.nvim",
    lazy = false,
    priority = 1000,
    opts = {
      transparent = false,
      style = "moon",
      styles = {
        functions = { italic = true, bold = true },
        keywords = { italic = true, bold = true },
      },
      on_highlights = function(highlights, colors)
        highlights["String"].italic = true
        highlights["Constant"].italic = true
        highlights["Constant"].bold = true
        highlights["@keyword.import.rust"] = { link = "@keyword" }
        highlights["@lsp.type.struct.rust"] = { fg = colors.blue1 }
        highlights["@lsp.type.enum.rust"] = { fg = colors.cyan, italic = true, bold = true }
        highlights["@tag.builtin.vue"] = { fg = colors.blue }
        highlights["@tag.vue"] = { fg = colors.red }
        highlights["@lsp.type.component.vue"] = { fg = colors.red }
        highlights["@tag.template.vue"] = { fg = colors.purple, bold = true }
        highlights["@tag.script.vue"] = { fg = colors.yellow, bold = true }
        highlights["@tag.style.vue"] = { fg = colors.green, bold = true }
        highlights["@spell"] = { italic = true }
        highlights["BlinkCmpSource"] = { fg = colors.fg, italic = true, bold = true }
        highlights["DiagnosticOk"] = { fg = colors.green }
        highlights["LogBlue"] = { link = "DiagnosticInfo" }
        highlights["LogGreen"] = { link = "DiagnosticOk" }
      end,
    },
    config = function(_, opts)
      require("tokyonight").setup(opts)
      vim.cmd.colorscheme("disco-whirling")
    end,
  },
}
