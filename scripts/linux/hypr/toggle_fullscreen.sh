#!/usr/bin/env bash
# 切换当前 Hyprland 窗口的可逆伪全屏状态。
# Usage: toggle_fullscreen.sh
# 依赖：hyprctl、jq
# 环境变量：无
# 未定义变量或未处理的命令失败时立即退出。
set -euo pipefail

# scrolling 的 maximize(mode 1) 会单向 expel 窗口，不能用于 toggle。
# fullscreen_state 的 internal=FULLSCREEN 会记录列宽并可逆恢复；client=NONE 不通知应用。
# 当前 activewindow.fullscreen 决定进入或退出。

win="$(hyprctl -j activewindow 2>/dev/null || true)"
is_fullscreen="$(echo "$win" | jq -r '.fullscreen // 0')"

if [[ "$is_fullscreen" == "0" || "$is_fullscreen" == "false" ]]; then
  hyprctl dispatch "hl.dsp.window.fullscreen_state({ internal = 2, client = 0, action = 'set' })"
else
  hyprctl dispatch "hl.dsp.window.fullscreen_state({ internal = 0, client = 0, action = 'set' })"
fi
