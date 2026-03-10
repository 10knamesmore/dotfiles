#!/usr/bin/env bash
set -euo pipefail

# scrolling 布局下使用 fakefullscreen，占满屏幕但不进入应用真全屏。
layout="$(hyprctl -j getoption general:layout 2>/dev/null | jq -r '.str // empty' || true)"

case "$layout" in
  scrolling)
    hyprctl dispatch fullscreen 1 
    ;;
  *)
    hyprctl dispatch fullscreen 0
    ;;
esac
