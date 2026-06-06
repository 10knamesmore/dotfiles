#!/usr/bin/env bash
set -euo pipefail

# 录屏 toggle：使用 OBS Studio
# OBS 通过 obs-cli 或直接启动控制

pid_file="/tmp/hypr-screenrec.pid"

# 如果正在录制 → 停止 OBS 录制
if [[ -f "$pid_file" ]]; then
  # 用 obs-cli 停止录制，或直接关 OBS
  if command -v obs-cli &>/dev/null; then
    obs-cli recording stop 2>/dev/null || true
  else
    pkill -f "obs" 2>/dev/null || true
  fi
  rm -f "$pid_file"
  notify-send -t 3000 "Screen Record" "录制已停止"
  exit 0
fi

# 开始录制：启动 OBS（最小化到托盘并自动录制）
touch "$pid_file"
notify-send -t 3000 "Screen Record" "正在启动 OBS..."
obs --startrecording --minimize-to-tray &
