#!/usr/bin/env bash
set -euo pipefail

dir="${1:-}"
case "$dir" in
  l|left) dir="l" ;;
  r|right) dir="r" ;;
  u|up|k) dir="u" ;;
  d|down|j) dir="d" ;;
  *)
    echo "usage: $0 <l|r|u|d>" >&2
    exit 2
    ;;
esac

monitors_json="$(hyprctl -j monitors 2>/dev/null || true)"
if [[ -z "$monitors_json" || "$monitors_json" == "null" ]]; then
  exit 1
fi

# 优先选择当前聚焦显示器；如果没有则使用第一个显示器。
current_name="$(echo "$monitors_json" | jq -r '[.[] | select(.focused==true)][0].name // .[0].name // empty')"
if [[ -z "$current_name" ]]; then
  exit 1
fi

# 计算当前显示器中心点坐标。
read -r cx cy < <(echo "$monitors_json" | jq -r --arg n "$current_name" '
  .[] | select(.name==$n) | "\(.x + (.width/2)) \(.y + (.height/2))"' | head -n1)
if [[ -z "${cx:-}" || -z "${cy:-}" ]]; then
  exit 1
fi

# 在指定方向上选择最近的目标显示器。
target="$(echo "$monitors_json" | jq -r --arg n "$current_name" --arg d "$dir" --argjson cx "$cx" --argjson cy "$cy" '
  [.[]
   | select(.name != $n)
   | . as $m
   | ($m.x + ($m.width/2)) as $mx
   | ($m.y + ($m.height/2)) as $my
   | ($mx - $cx) as $dx
   | ($my - $cy) as $dy
   | select(
      ($d=="l" and $dx < 0) or
      ($d=="r" and $dx > 0) or
      ($d=="u" and $dy < 0) or
      ($d=="d" and $dy > 0)
     )
   | {
      name: $m.name,
      score: ($dx*$dx + $dy*$dy)
     }
  ]
  | sort_by(.score)
  | .[0].name // empty')"

if [[ -z "$target" ]]; then
  exit 1
fi

hyprctl dispatch movewindow "mon:${target}" >/dev/null
