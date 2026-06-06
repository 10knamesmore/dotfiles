#!/bin/sh

# 显示当前聚焦窗口标题，长标题不做滚动。

WINDOW_INFO="$(yabai -m query --windows --window 2>/dev/null | jq -r 'select(.id != null) | "\(.app)  \(.title)"')"

if [ -z "$WINDOW_INFO" ]; then
  WINDOW_INFO="Desktop"
fi

sketchybar --set "$NAME" label="$WINDOW_INFO"
