#!/usr/bin/env bash
set -euo pipefail

# 切换当前窗口透明度：不透明 ↔ 默认透明度规则

addr="$(hyprctl -j activewindow 2>/dev/null | jq -r '.address // empty')"
[[ -z "$addr" ]] && exit 0

opacity="$(hyprctl -j activewindow | jq -r '.opacity // 1')"

if awk "BEGIN { exit ($opacity >= 1.0) ? 1 : 0 }"; then
  # 当前半透明 → 设为不透明
  hyprctl dispatch setprop "address:$addr" alpha 1.0 lock >/dev/null
else
  # 当前不透明 → 恢复默认
  hyprctl dispatch setprop "address:$addr" alpha unset >/dev/null
fi
