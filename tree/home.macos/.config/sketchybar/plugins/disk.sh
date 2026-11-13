#!/bin/sh

# 显示根 APFS 容器当前占用百分比。

TOTAL_BYTES="$(diskutil info / 2>/dev/null | awk -F'[()]' '
  /Container Total Space:/ {
    value = $2
    gsub(/[^0-9]/, "", value)
    print value
    exit
  }
')"
FREE_BYTES="$(diskutil info / 2>/dev/null | awk -F'[()]' '
  /Container Free Space:/ {
    value = $2
    gsub(/[^0-9]/, "", value)
    print value
    exit
  }
')"

if [ -n "$TOTAL_BYTES" ] && [ -n "$FREE_BYTES" ]; then
  USAGE="$(awk -v total="$TOTAL_BYTES" -v free="$FREE_BYTES" '
    BEGIN { printf "%d%%", ((total - free) / total) * 100 + 0.5 }
  ')"
else
  USAGE="--%"
fi

sketchybar --set "$NAME" label="$USAGE"
