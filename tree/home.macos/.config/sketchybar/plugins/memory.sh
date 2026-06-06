#!/bin/sh

# 显示内存使用百分比；hover 时显示 used、cached、compressed 和 swap。

collect_memory_stats() {
  page_size="$(vm_stat | awk '/page size of/ {gsub("\\.", "", $8); print $8; exit}')"
  total_mem="$(sysctl -n hw.memsize 2>/dev/null)"

  if [ -z "$page_size" ] || [ -z "$total_mem" ]; then
    return 1
  fi

  pages_active="$(vm_stat | awk '/Pages active/ {gsub("\\.", "", $3); print $3}')"
  pages_wired="$(vm_stat | awk '/Pages wired down/ {gsub("\\.", "", $4); print $4}')"
  pages_purgeable="$(vm_stat | awk '/Pages purgeable/ {gsub("\\.", "", $3); print $3}')"
  pages_file_backed="$(vm_stat | awk '/File-backed pages/ {gsub("\\.", "", $3); print $3}')"
  pages_compressed="$(vm_stat | awk '/Pages occupied by compressor/ {gsub("\\.", "", $5); print $5}')"
  used_bytes=$(( (pages_active + pages_wired + pages_compressed) * page_size ))
  used_percent="$(awk -v used="$used_bytes" -v total="$total_mem" 'BEGIN { printf "%d", (used / total) * 100 + 0.5 }')"
  used_gb="$(awk -v used="$used_bytes" 'BEGIN { printf "%.1f", used / 1024 / 1024 / 1024 }')"
  cached_bytes=$(( (pages_purgeable + pages_file_backed) * page_size ))
  cached_gb="$(awk -v cached="$cached_bytes" 'BEGIN { printf "%.1f", cached / 1024 / 1024 / 1024 }')"
  compressed_gb="$(awk -v compressed_pages="$pages_compressed" -v size="$page_size" 'BEGIN { printf "%.1f", compressed_pages * size / 1024 / 1024 / 1024 }')"

  swap_usage="$(sysctl vm.swapusage 2>/dev/null)"
  swap_used="$(printf '%s\n' "$swap_usage" | awk -F'=|M  |M$' '/used/ {gsub(/^[ \t]+/, "", $2); if ($2 == "") print "--"; else printf "%.1f", $2 / 1024 }')"
  swap_total="$(printf '%s\n' "$swap_usage" | awk -F'=|M  |M$' '/total/ {gsub(/^[ \t]+/, "", $2); if ($2 == "") print "--"; else printf "%.1f", $2 / 1024 }')"

  [ -z "$swap_used" ] && swap_used="--"
  [ -z "$swap_total" ] && swap_total="--"

  MEMORY_USED_PERCENT="$used_percent"
  MEMORY_USED_GB="$used_gb"
  MEMORY_CACHED_GB="$cached_gb"
  MEMORY_COMPRESSED_GB="$compressed_gb"
  MEMORY_SWAP_USED_GB="$swap_used"
  MEMORY_SWAP_TOTAL_GB="$swap_total"
  return 0
}

set_main_item() {
  if ! collect_memory_stats; then
    sketchybar --set "$NAME" label="--%"
    return
  fi

  if [ "$MEMORY_SWAP_USED_GB" = "--" ] || [ "$MEMORY_SWAP_USED_GB" = "0.0" ]; then
    label="${MEMORY_USED_PERCENT}%"
  else
    label="${MEMORY_USED_PERCENT}% ${MEMORY_SWAP_USED_GB}|${MEMORY_SWAP_TOTAL_GB}"
  fi

  sketchybar --set "$NAME" label="$label"
}

show_details() {
  if ! collect_memory_stats; then
    sketchybar --set memory.details.title label="Memory unavailable" \
               --set memory.details.used label="" \
               --set memory.details.cached label="" \
               --set memory.details.compressed label="" \
               --set memory.details.swap label="" \
               --set "$NAME" popup.drawing=on
    return
  fi

  sketchybar --set memory.details.title label="Memory details" \
             --set memory.details.used label="Used: ${MEMORY_USED_GB} GB" \
             --set memory.details.cached label="Cached: ${MEMORY_CACHED_GB} GB" \
             --set memory.details.compressed label="Compressed: ${MEMORY_COMPRESSED_GB} GB" \
             --set memory.details.swap label="Swap: ${MEMORY_SWAP_USED_GB}/${MEMORY_SWAP_TOTAL_GB} GB" \
             --set "$NAME" popup.drawing=on
}

hide_details() {
  sketchybar --set "$NAME" popup.drawing=off
}

case "$SENDER" in
  "mouse.entered")
    show_details
    ;;
  "mouse.exited"|"mouse.exited.global")
    hide_details
    ;;
  *)
    set_main_item
    ;;
esac
