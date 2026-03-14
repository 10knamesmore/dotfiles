#!/bin/sh

# 基于默认网卡估算当前上传或下载速率。

MODE="$1"
IFACE="$(route -n get default 2>/dev/null | awk '/interface:/{print $2; exit}')"

if [ -z "$IFACE" ]; then
  sketchybar --set "$NAME" label="-- KB/s"
  exit 0
fi

COUNTERS="$(netstat -ibn 2>/dev/null | awk -v iface="$IFACE" '$1 == iface && $7 ~ /^[0-9]+$/ { inb += $7; outb += $10 } END { print inb, outb }')"
CUR_IN="$(printf '%s' "$COUNTERS" | awk '{print $1}')"
CUR_OUT="$(printf '%s' "$COUNTERS" | awk '{print $2}')"

[ -z "$CUR_IN" ] && CUR_IN=0
[ -z "$CUR_OUT" ] && CUR_OUT=0

STATE_FILE="/tmp/sketchybar_${MODE}_${IFACE}.state"
NOW="$(date +%s)"

if [ -f "$STATE_FILE" ]; then
  PREV_TIME="$(awk 'NR==1 {print $1}' "$STATE_FILE")"
  PREV_VALUE="$(awk 'NR==1 {print $2}' "$STATE_FILE")"
else
  PREV_TIME="$NOW"
  if [ "$MODE" = "up" ]; then
    PREV_VALUE="$CUR_OUT"
  else
    PREV_VALUE="$CUR_IN"
  fi
fi

if [ "$MODE" = "up" ]; then
  CUR_VALUE="$CUR_OUT"
else
  CUR_VALUE="$CUR_IN"
fi

DELTA_TIME=$(( NOW - PREV_TIME ))
[ "$DELTA_TIME" -le 0 ] && DELTA_TIME=1
DELTA_VALUE=$(( CUR_VALUE - PREV_VALUE ))
[ "$DELTA_VALUE" -lt 0 ] && DELTA_VALUE=0

RATE_KB="$(awk -v bytes="$DELTA_VALUE" -v secs="$DELTA_TIME" 'BEGIN { printf "%.1f", bytes / 1024 / secs }')"
printf '%s %s\n' "$NOW" "$CUR_VALUE" > "$STATE_FILE"

sketchybar --set "$NAME" label="${RATE_KB} KB/s"
