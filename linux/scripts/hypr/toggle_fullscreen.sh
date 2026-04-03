#!/usr/bin/env bash
set -euo pipefail

# scrolling 布局下使用 fakefullscreen，占满屏幕但不进入应用真全屏。
# 同时在进入全屏时将 gaps 设为 0，退出时恢复。

layout="$(hyprctl -j getoption general:layout 2>/dev/null | jq -r '.str // empty' || true)"
win="$(hyprctl -j activewindow 2>/dev/null || true)"
is_fullscreen="$(echo "$win" | jq -r '.fullscreen // 0')"

case "$layout" in
  scrolling)
    hyprctl dispatch fullscreen 1
    ;;
  *)
    hyprctl dispatch fullscreen 0
    ;;
esac

if [[ "$is_fullscreen" == "0" || "$is_fullscreen" == "false" ]]; then
  # 进入全屏 → gaps 归零
  hyprctl keyword general:gaps_in 0 >/dev/null
  hyprctl keyword general:gaps_out 0 >/dev/null
else
  # 退出全屏 → 恢复默认 gaps
  hyprctl keyword general:gaps_in 3 >/dev/null
  hyprctl keyword general:gaps_out "5,10,10,10" >/dev/null
fi
