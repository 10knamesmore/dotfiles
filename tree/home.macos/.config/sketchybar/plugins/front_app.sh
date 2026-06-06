#!/bin/sh

# 用当前聚焦应用的名称更新标签。

if [ "$SENDER" = "front_app_switched" ]; then
  sketchybar --set "$NAME" label="$INFO"
fi
