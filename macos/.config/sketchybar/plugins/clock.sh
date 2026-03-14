#!/bin/sh

# 以 24 小时制显示中间时钟。

sketchybar --set "$NAME" label="$(date '+%H:%M:%S')"
