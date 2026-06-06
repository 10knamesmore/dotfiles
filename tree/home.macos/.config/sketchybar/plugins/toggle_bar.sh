#!/bin/sh

# 切换 sketchybar 显隐，并同步更新 yabai 为顶部栏预留的空间。

HIDDEN="$(sketchybar --query bar | jq -r '.hidden')"

if [ "$HIDDEN" = "on" ] || [ "$HIDDEN" = "true" ]; then
    sketchybar --bar hidden=off
    yabai -m config external_bar all:42:0 >/dev/null 2>&1
    sketchybar --update
else
    sketchybar --bar hidden=on
    yabai -m config external_bar off:0:0 >/dev/null 2>&1
fi
