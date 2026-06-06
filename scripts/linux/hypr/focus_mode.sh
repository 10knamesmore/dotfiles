#!/usr/bin/env bash
set -euo pipefail

# 专注模式 toggle：gaps 归零、增强 dim、全不透明焦点窗口

state_file="/tmp/hypr-focus-mode"

if [[ -f "$state_file" ]]; then
  # 退出专注模式 → 恢复默认值（同 appearances.conf）
  hyprctl keyword general:gaps_in 3 >/dev/null
  hyprctl keyword general:gaps_out "5,10,10,10" >/dev/null
  hyprctl keyword decoration:dim_strength 0.15 >/dev/null
  hyprctl keyword decoration:active_opacity 0.98 >/dev/null
  hyprctl keyword decoration:inactive_opacity 0.85 >/dev/null
  rm -f "$state_file"
  notify-send -t 2000 "Focus Mode" "已退出"
else
  # 进入专注模式
  hyprctl keyword general:gaps_in 0 >/dev/null
  hyprctl keyword general:gaps_out 0 >/dev/null
  hyprctl keyword decoration:dim_strength 0.4 >/dev/null
  hyprctl keyword decoration:active_opacity 1.0 >/dev/null
  hyprctl keyword decoration:inactive_opacity 0.7 >/dev/null
  touch "$state_file"
  notify-send -t 2000 "Focus Mode" "已进入"
fi
