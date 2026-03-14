#!/bin/sh

# 显示当前 yabai space 的布局类型，便于在状态栏里看到平铺模式。

LAYOUT="$(yabai -m query --spaces --space 2>/dev/null | jq -r '.type // empty')"

case "$LAYOUT" in
  bsp) LABEL="bsp" ;;
  stack) LABEL="stack" ;;
  float) LABEL="float" ;;
  *) LABEL="n/a" ;;
esac

sketchybar --set "$NAME" label="$LABEL"
