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
})
