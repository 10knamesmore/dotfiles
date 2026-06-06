#!/usr/bin/env bash
set -euo pipefail

# 伪全屏 toggle。0.55 起 scrolling 的 maximize(mode 1) 被改成单向 expel、不可逆
#   (src/layout/algorithm/tiled/scrolling/ScrollingAlgorithm.cpp::requestFullscreen)。
# 改用 fullscreen_state：internal=FULLSCREEN 走可逆的全屏分支(铺满+记录列宽还原、不 expel)，
# client=NONE 让应用无感(fakefullscreen)。按 .fullscreen 决定进/出。

win="$(hyprctl -j activewindow 2>/dev/null || true)"
is_fullscreen="$(echo "$win" | jq -r '.fullscreen // 0')"

if [[ "$is_fullscreen" == "0" || "$is_fullscreen" == "false" ]]; then
  hyprctl dispatch "hl.dsp.window.fullscreen_state({ internal = 2, client = 0, action = 'set' })"
else
  hyprctl dispatch "hl.dsp.window.fullscreen_state({ internal = 0, client = 0, action = 'set' })"
fi
