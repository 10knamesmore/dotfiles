#!/bin/sh

# 显示内存使用百分比；仅当 swap 非 0 时追加显示 used|total。

PAGE_SIZE="$(vm_stat | awk '/page size of/ {gsub("\\.", "", $8); print $8; exit}')"
TOTAL_MEM="$(sysctl -n hw.memsize)"

if [ -z "$PAGE_SIZE" ] || [ -z "$TOTAL_MEM" ]; then
  sketchybar --set "$NAME" label="--%"
  exit 0
fi

PAGES_ACTIVE="$(vm_stat | awk '/Pages active/ {gsub("\\.", "", $3); print $3}')"
PAGES_WIRED="$(vm_stat | awk '/Pages wired down/ {gsub("\\.", "", $4); print $4}')"
PAGES_COMPRESSED="$(vm_stat | awk '/Pages occupied by compressor/ {gsub("\\.", "", $5); print $5}')"
USED_BYTES=$(( (PAGES_ACTIVE + PAGES_WIRED + PAGES_COMPRESSED) * PAGE_SIZE ))
USED_PERCENT="$(awk -v used="$USED_BYTES" -v total="$TOTAL_MEM" 'BEGIN { printf "%d", (used / total) * 100 + 0.5 }')"

SWAP_USAGE="$(sysctl vm.swapusage 2>/dev/null)"
SWAP_USED="$(printf '%s\n' "$SWAP_USAGE" | awk -F'=|M  |M$' '/used/ {gsub(/^[ \t]+/, "", $2); if ($2 == "") print "--"; else printf "%.1f", $2 / 1024 }')"
SWAP_TOTAL="$(printf '%s\n' "$SWAP_USAGE" | awk -F'=|M  |M$' '/total/ {gsub(/^[ \t]+/, "", $2); if ($2 == "") print "--"; else printf "%.1f", $2 / 1024 }')"

[ -z "$SWAP_USED" ] && SWAP_USED="--"
[ -z "$SWAP_TOTAL" ] && SWAP_TOTAL="--"

if [ "$SWAP_USED" = "--" ] || [ "$SWAP_USED" = "0.0" ]; then
  LABEL="${USED_PERCENT}%"
else
  LABEL="${USED_PERCENT}% ${SWAP_USED}|${SWAP_TOTAL}"
fi

sketchybar --set "$NAME" label="$LABEL"
