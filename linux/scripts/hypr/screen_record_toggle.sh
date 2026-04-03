#!/usr/bin/env bash
set -euo pipefail

# 录屏 toggle：使用 wl-screenrec，支持 --region 选区录制

pid_file="/tmp/hypr-screenrec.pid"
output_dir="$HOME/Videos"
mkdir -p "$output_dir"

# 如果正在录制 → 停止
if [[ -f "$pid_file" ]]; then
  pid="$(cat "$pid_file")"
  if kill -0 "$pid" 2>/dev/null; then
    kill -INT "$pid"
    wait "$pid" 2>/dev/null || true
    rm -f "$pid_file"
    notify-send -t 3000 "Screen Record" "录制已停止"
    exit 0
  else
    # PID 已失效，清理
    rm -f "$pid_file"
  fi
fi

# 开始录制
output_file="$output_dir/screenrec-$(date +%Y%m%d-%H%M%S).mp4"

if [[ "${1:-}" == "--region" ]]; then
  # 选区录制：先用 slurp 选区
  geometry="$(slurp 2>/dev/null)" || exit 0
  wl-screenrec -g "$geometry" -f "$output_file" &
else
  wl-screenrec -f "$output_file" &
fi

echo $! > "$pid_file"
notify-send -t 3000 "Screen Record" "录制中... → $output_file"
