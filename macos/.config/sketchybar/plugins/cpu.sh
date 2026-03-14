#!/bin/sh

# 采样最近一次 CPU 使用率，并显示 user + system 的总占用百分比。

USAGE="$(top -l 2 -n 0 | awk '/CPU usage/ { line=$0 } END { sub(/.*CPU usage: /, "", line); sub(/ idle.*/, "", line); gsub(/%/, "", line); split(line, parts, ", "); user=parts[1]+0; sys=parts[2]+0; printf "%d", user + sys + 0.5 }')"

[ -z "$USAGE" ] && USAGE="--"

sketchybar --set "$NAME" label="${USAGE}%"
