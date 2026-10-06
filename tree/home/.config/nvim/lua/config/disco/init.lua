local M = {}

-- Supply the base palette before Tokyo Night derives diff, terminal, popup, and plugin colors.
local function theme_palette(c)
  return {
    bg = c.base,
    bg_dark = c.mantle,
    bg_dark1 = c.crust,
    bg_highlight = c.surface0,
    fg = c.text,
    fg_dark = c.subtext0,
    fg_gutter = c.overlay0,
    comment = c.overlay1,
    dark3 = c.overlay0,
    dark5 = c.overlay2,
    blue = c.blue,
    blue0 = c.surface2,
    blue1 = c.sapphire,
    blue2 = c.sky,
    blue5 = c.sky,
    blue6 = c.teal,
    blue7 = c.surface1,
    cyan = c.sky,
    green = c.green,
    green1 = c.teal,
    green2 = c.green,
    magenta = c.lavender,
    magenta2 = c.pink,
    orange = c.peach,
    purple = c.mauve,
    red = c.red,
    red1 = c.red,
    teal = c.teal,
    terminal_black = c.surface2,
    yellow = c.yellow,
    git = { add = c.green, change = c.sapphire, delete = c.red },
  }
end

--- Apply a Disco Elysium palette using Tokyo Night's syntax and plugin integrations.
---@param variant "whirling"|"sunset"|"coast"|"pale"
function M.load(variant)
  local palette = require("config.disco.palettes")[variant]
  local style = "disco-" .. variant
  -- A separate style also gives each palette its own Tokyo Night cache.
  require("tokyonight.colors").styles[style] = function()
    return theme_palette(palette.colors)
  end
  require("tokyonight").load({
    style = style,
    transparent = false,
    terminal_colors = true,
    styles = {
      comments = { italic = true },
      functions = { italic = true, bold = true },
      keywords = { italic = true, bold = true },
    },
    on_highlights = function(highlights)
      local custom = require("config.disco.highlights").get(palette.colors, palette.colors[palette.accent])
      for group, value in pairs(custom) do
        highlights[group] = value
      end
    end,
  })
  vim.g.colors_name = style
end

return M
