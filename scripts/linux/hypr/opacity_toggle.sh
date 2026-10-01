#!/usr/bin/env bash
set -euo pipefail

# 切换当前窗口透明度：不透明 ↔ 默认透明度规则

addr="$(hyprctl -j activewindow 2>/dev/null | jq -r '.address // empty')"
[[ -z "$addr" ]] && exit 0

opacity="$(hyprctl -j activewindow | jq -r '.opacity // 1')"

if awk "BEGIN { exit ($opacity >= 1.0) ? 1 : 0 }"; then
  # 当前半透明 → 设为不透明，并锁定不被窗口规则覆盖
  hyprctl dispatch "hl.dsp.window.set_prop({ prop = 'opacity', value = '1.0', window = 'address:$addr' })" >/dev/null
  hyprctl dispatch "hl.dsp.window.set_prop({ prop = 'opacity_override', value = '1', window = 'address:$addr' })" >/dev/null
else
  # 当前不透明 → 解锁覆盖，回到默认透明度规则
  hyprctl dispatch "hl.dsp.window.set_prop({ prop = 'opacity_override', value = '0', window = 'address:$addr' })" >/dev/null
fi
