---@type mineral.TuiConfig
return {
  animation = {
    ambient_trail = {
      enter = { delay_ms = 80, ease_ms = 740 },
      exit = {},
    },
  },
  theme = {
    base = "#1a1b26",
    mantle = "#16161e",
    crust = "#13131a",
    surface0 = "#292e42",
    surface1 = "#3b4261",
    overlay = "#565f89",
    subtext = "#a9b1d6",
    text = "#c0caf5",
    accent = "#bb9af7",
    accent_2 = "#7aa2f7",
    red = "#f7768e",
    yellow = "#e0af68",
    green = "#9ece6a",
    peach = "#ff9e64",
  },
  spectrum = {
    style = "bars",
    waterfall = { push_ms = 42 },
  },
  progress = {
    track = {
      played_alpha = 0.4,
      buffered_alpha = 0.15,
      unbuffered_alpha = 0.04,
    },
    playhead = {
      text_mix = 0.40,
      trail_columns = 6,
      lead_columns = 2,
    },
  },
  cover = {
    debounce_ms = 32,
    cache = {
      image = 256 * 1024 * 1024,
      protocol = 128 * 1024 * 1024,
    },
  },
  cover_transition = {
    style = "slide",
  },
  waveform = {
    contrast = 3,
  },
  behavior = {
    filter_play_scope = "matches",
    kill_spawned_daemon_on_exit = false,
  },
}
