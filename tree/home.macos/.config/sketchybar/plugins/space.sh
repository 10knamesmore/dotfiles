#!/bin/sh

# 高亮当前 Mission Control 空间，并弱化未选中的空间。

if [ "$SELECTED" = "true" ]; then
  sketchybar --set "$NAME" background.drawing=on icon.color=0xff24273a
else
  sketchybar --set "$NAME" background.drawing=off icon.color=0xff8087a2
fi
