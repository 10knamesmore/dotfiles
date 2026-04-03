#!/usr/bin/env bash
set -euo pipefail

# 显示器模式切换：dual / external / laptop

mode="${1:-}"

case "$mode" in
  dual)
    hyprctl keyword monitor "eDP-1,2560x1600@60,640x2160,1" >/dev/null
    hyprctl keyword monitor "DP-3,3840x2160@60,0x0,1" >/dev/null
    notify-send -t 2000 "Monitor" "双屏模式"
    ;;
  external)
    hyprctl keyword monitor "eDP-1,disabled" >/dev/null
    hyprctl keyword monitor "DP-3,3840x2160@60,0x0,1" >/dev/null
    notify-send -t 2000 "Monitor" "仅外接显示器"
    ;;
  laptop)
    hyprctl keyword monitor "DP-3,disabled" >/dev/null
    hyprctl keyword monitor "eDP-1,2560x1600@60,0x0,1" >/dev/null
    notify-send -t 2000 "Monitor" "仅笔记本屏幕"
    ;;
  *)
    echo "usage: $0 <dual|external|laptop>" >&2
    exit 2
    ;;
esac
