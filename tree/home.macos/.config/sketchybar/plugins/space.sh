#!/bin/sh

# 高亮当前 Mission Control 空间，并弱化未选中的空间。

if [ "$SELECTED" = "true" ]; then
  sketchybar --animate tanh 9 --set "$NAME" background.drawing=on icon.color=0xff1e1e2e
else
  sketchybar --animate tanh 9 --set "$NAME" background.drawing=off icon.color=0xff7f849c
fi
