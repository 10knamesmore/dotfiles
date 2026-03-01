#!/usr/bin/env bash
set -euo pipefail

# 在布局中对垂直方向做“分帧”调整，避免 resizeactive 看起来过于突兀。
# 用法: resizeactive_animated.sh up|down [total_px] [frames] [sleep_seconds]
dir="${1:-}"
total_px="${2:-50}"
frames="${3:-8}"
sleep_s="${4:-0.008}"

case "$dir" in
    up|k) ;;
    down|j) ;;
    *)
        echo "usage: $0 up|down [total_px] [frames] [sleep_seconds]" >&2
        exit 2
        ;;
esac

# 参数校验：全部使用整数步进，余数分配到前几帧。
if ! [[ "$total_px" =~ ^[0-9]+$ ]] || ! [[ "$frames" =~ ^[0-9]+$ ]] || [ "$frames" -le 0 ]; then
    echo "invalid numeric args" >&2
    exit 2
fi

# 读取活动窗口与当前工作区内窗口列表。
active="$(hyprctl -j activewindow 2>/dev/null || true)"
clients="$(hyprctl -j clients 2>/dev/null || true)"

if [[ -z "$active" || -z "$clients" || "$active" == "null" || "$clients" == "null" ]]; then
    exit 0
fi

# 提取活动窗口几何信息（全局坐标）。
ax="$(echo "$active" | jq -r '.at[0] // empty')"
ay="$(echo "$active" | jq -r '.at[1] // empty')"
aw="$(echo "$active" | jq -r '.size[0] // empty')"
ah="$(echo "$active" | jq -r '.size[1] // empty')"
addr="$(echo "$active" | jq -r '.address // empty')"
wsid="$(echo "$active" | jq -r '.workspace.id // empty')"

if [[ -z "$ax" || -z "$ay" || -z "$aw" || -z "$ah" || -z "$addr" || -z "$wsid" ]]; then
    exit 0
fi

# 判断是否存在上/下邻接窗口（同工作区、平铺窗口、水平投影重叠）。
has_up="$(echo "$clients" | jq -r \
    --arg addr "$addr" \
    --argjson ws "$wsid" \
    --argjson ax "$ax" \
    --argjson ay "$ay" \
    --argjson aw "$aw" '
  any(.[]; 
    (.address != $addr)
    and ((.workspace.id // -999999) == $ws)
    and ((.floating // false) == false)
    and (
      (([($ax + $aw), ((.at[0] // 0) + (.size[0] // 0))] | min)
      - ([$ax, (.at[0] // 0)] | max)) > 0
    )
    and (((.at[1] // 0) + (.size[1] // 0)) <= ($ay + 2))
  )')"

has_down="$(echo "$clients" | jq -r \
    --arg addr "$addr" \
    --argjson ws "$wsid" \
    --argjson ax "$ax" \
    --argjson ay "$ay" \
    --argjson aw "$aw" \
    --argjson ah "$ah" '
  any(.[]; 
    (.address != $addr)
    and ((.workspace.id // -999999) == $ws)
    and ((.floating // false) == false)
    and (
      (([($ax + $aw), ((.at[0] // 0) + (.size[0] // 0))] | min)
      - ([$ax, (.at[0] // 0)] | max)) > 0
    )
    and ((.at[1] // 0) >= (($ay + $ah) - 2))
  )')"

# 方向策略（保证 Ctrl+J / Ctrl+K 在任意位置都相反）：
# 1) 有上邻居时：k 为正步进，j 为负步进（中间/底部窗口）。
# 2) 无上邻居但有下邻居时：k 为负步进，j 为正步进（顶部窗口）。
# 3) 上下都没有邻居时：不做处理。
if [[ "$has_up" != "true" && "$has_down" != "true" ]]; then
    exit 0
fi

if [[ "$dir" == "up" || "$dir" == "k" ]]; then
    if [[ "$has_up" == "true" ]]; then
        step_sign=1
    else
        step_sign=-1
    fi
else
    if [[ "$has_up" == "true" ]]; then
        step_sign=-1
    else
        step_sign=1
    fi
fi

base_step=$((total_px / frames))
remainder=$((total_px % frames))

for i in $(seq 1 "$frames"); do
    step="$base_step"
    if [ "$i" -le "$remainder" ]; then
        step=$((step + 1))
    fi

    [ "$step" -eq 0 ] && continue

    hyprctl dispatch resizeactive "0 $((step_sign * step))" >/dev/null 2>&1 || exit 0
    sleep "$sleep_s"
done
