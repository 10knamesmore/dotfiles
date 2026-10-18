-- 由 Matugen 根据壁纸生成。
hl.config({
  general = {
    col = {
      active_border = {
        colors = { "rgba({{colors.primary.dark.hex_stripped}}ee)", "rgba({{colors.tertiary.dark.hex_stripped}}ee)" },
        angle = 45,
      },
      inactive_border = "rgba({{colors.outline_variant.dark.hex_stripped}}66)",
    },
  },
  decoration = {
    shadow = {
      color = { colors = { "rgba({{colors.surface_container_lowest.dark.hex_stripped}}ee)", "rgba({{colors.surface_container.dark.hex_stripped}}cc)" }, angle = 45 },
      color_inactive = "rgba({{colors.surface_container_lowest.dark.hex_stripped}}99)",
    },
  },
  group = {
    col = {
      border_active = "rgba({{colors.primary.dark.hex_stripped}}ee)",
      border_inactive = "rgba({{colors.outline_variant.dark.hex_stripped}}66)",
      border_locked_active = "rgba({{colors.primary.dark.hex_stripped}}ee)",
      border_locked_inactive = "rgba({{colors.outline_variant.dark.hex_stripped}}66)",
    },
    groupbar = {
      col = {
        active = "rgba({{colors.primary_container.dark.hex_stripped}}ff)",
        inactive = "rgba({{colors.surface_container_high.dark.hex_stripped}}ff)",
        locked_active = "rgba({{colors.primary_container.dark.hex_stripped}}ff)",
        locked_inactive = "rgba({{colors.surface_container_high.dark.hex_stripped}}ff)",
      },
      text_color = "rgba({{colors.on_primary_container.dark.hex_stripped}}ff)",
      text_color_inactive = "rgba({{colors.on_surface_variant.dark.hex_stripped}}ff)",
      text_color_locked_active = "rgba({{colors.on_primary_container.dark.hex_stripped}}ff)",
      text_color_locked_inactive = "rgba({{colors.on_surface_variant.dark.hex_stripped}}ff)",
    },
  },
})
