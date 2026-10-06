local M = {}

--- Lualine calls this again on ColorScheme, so mode colors follow the active palette.
function M.theme()
  local variant = (vim.g.colors_name or ""):match("^disco%-(.+)$")
  if not variant then
    return "auto"
  end

  local palette = require("config.disco.palettes")[variant]
  local c = palette.colors
  local theme = {}
  for mode, color in pairs({
    normal = c[palette.accent],
    insert = c.green,
    visual = c.mauve,
    replace = c.red,
    command = c.yellow,
    terminal = c.teal,
  }) do
    theme[mode] = {
      a = { fg = c.crust, bg = color, gui = "bold" },
      b = { fg = c.subtext1, bg = c.surface0 },
      c = { fg = c.subtext0, bg = c.mantle },
      y = { fg = c.subtext1, bg = c.surface0 },
      z = { fg = c.crust, bg = color },
    }
  end
  theme.inactive = {
    a = { fg = c.overlay1, bg = c.mantle },
    b = { fg = c.overlay1, bg = c.mantle },
    c = { fg = c.overlay1, bg = c.mantle },
  }
  return theme
end

return M
