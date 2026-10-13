#!/bin/sh

# 活动工作区使用紫色数字和底部短线，其余数字弱化显示。

if [ "$SELECTED" = "true" ]; then
  sketchybar --animate tanh 9 --set "$NAME" background.drawing=on icon.color=0xffcba6f7
else
  sketchybar --animate tanh 9 --set "$NAME" background.drawing=off icon.color=0xff7f849c
fi
