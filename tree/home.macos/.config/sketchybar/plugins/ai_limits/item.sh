#!/bin/sh
# 把标准化额度 JSON 映射到 W/5h 剩余百分比主标签和完整 hover popup。
# Usage: item.sh <codex|kimi|opencode>
# 依赖：sketchybar, jq，以及同目录对应的采集器
# 读取环境变量：NAME、SENDER（SketchyBar 注入）及采集器声明的凭据变量
# 写入环境变量：无

PROVIDER="${1:-}"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

case "$PROVIDER" in
  codex)
    DISPLAY_NAME="Codex"
    DETAIL_SLOTS=4
    ;;
  kimi)
    DISPLAY_NAME="Kimi"
    DETAIL_SLOTS=4
    ;;
  opencode)
    DISPLAY_NAME="OpenCode"
    DETAIL_SLOTS=4
    ;;
  *)
    exit 1
    ;;
esac

ITEM_NAME="${NAME:-ai.$PROVIDER}"
DETAIL_PREFIX="ai.$PROVIDER.details"

show_details() {
  sketchybar --set "$ITEM_NAME" popup.drawing=on
}

hide_details() {
  sketchybar --set "$ITEM_NAME" popup.drawing=off
}

# 每次失败都覆盖主标签并清空未用行，避免把旧额度误当成当前数据。
render_error() {
  local message="$1"
  local slot

  sketchybar --set "$ITEM_NAME" label="$DISPLAY_NAME --" \
             --set "$DETAIL_PREFIX.title" label="$DISPLAY_NAME limits" \
             --set "$DETAIL_PREFIX.1" label="$message" drawing=on

  slot=2
  while [ "$slot" -le "$DETAIL_SLOTS" ]; do
    sketchybar --set "$DETAIL_PREFIX.$slot" label="" drawing=off
    slot=$((slot + 1))
  done
}

refresh_usage() {
  local payload
  local status
  local main
  local message
  local slot
  local detail

  payload="$("$SCRIPT_DIR/$PROVIDER.sh" 2>/dev/null)"
  if ! printf '%s\n' "$payload" | jq -e 'type == "object" and (.status | type == "string")' >/dev/null 2>&1; then
    render_error "Invalid collector response"
    return
  fi

  status="$(printf '%s\n' "$payload" | jq -r '.status')"
  if [ "$status" != "ok" ]; then
    message="$(printf '%s\n' "$payload" | jq -r '.message // "Usage unavailable"')"
    render_error "$message"
    return
  fi

  main="$(printf '%s\n' "$payload" | jq -r '.main')"
  sketchybar --set "$ITEM_NAME" label="$DISPLAY_NAME $main" \
             --set "$DETAIL_PREFIX.title" label="$DISPLAY_NAME limits"

  slot=1
  while [ "$slot" -le "$DETAIL_SLOTS" ]; do
    detail="$(printf '%s\n' "$payload" | jq -r --argjson index "$((slot - 1))" '.details[$index] // empty')"
    if [ -n "$detail" ]; then
      sketchybar --set "$DETAIL_PREFIX.$slot" label="$detail" drawing=on
    else
      sketchybar --set "$DETAIL_PREFIX.$slot" label="" drawing=off
    fi
    slot=$((slot + 1))
  done
}

case "${SENDER:-routine}" in
  mouse.entered)
    show_details
    ;;
  mouse.exited|mouse.exited.global)
    hide_details
    ;;
  *)
    refresh_usage
    ;;
esac
