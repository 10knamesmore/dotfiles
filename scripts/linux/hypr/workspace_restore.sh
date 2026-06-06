#!/usr/bin/env bash
set -euo pipefail

# 恢复窗口到保存的工作区布局（按 class 匹配当前运行的窗口）

cache_dir="$HOME/.cache/hypr"
layout_file="$cache_dir/workspace-layout.json"

if [[ ! -f "$layout_file" ]]; then
  notify-send -t 3000 "Workspace" "没有找到保存的布局"
  exit 1
fi

saved="$(cat "$layout_file")"
clients="$(hyprctl -j clients 2>/dev/null)"
moved=0

# 对保存的每个条目，找到匹配的运行窗口并移动到目标工作区
echo "$saved" | jq -c '.[]' | while IFS= read -r entry; do
  class="$(echo "$entry" | jq -r '.class')"
  target_ws="$(echo "$entry" | jq -r '.workspace')"

  [[ -z "$class" || "$class" == "null" || -z "$target_ws" || "$target_ws" == "null" ]] && continue
  # 跳过 special workspace（负数 id）
  [[ "$target_ws" -lt 0 ]] 2>/dev/null && continue

  # 找到第一个匹配 class 且不在目标工作区的窗口
  addr="$(echo "$clients" | jq -r --arg c "$class" --argjson ws "$target_ws" '
    [.[] | select(.class == $c and .workspace.id != $ws)] | .[0].address // empty
  ')"

  if [[ -n "$addr" ]]; then
    hyprctl dispatch movetoworkspacesilent "$target_ws,address:$addr" >/dev/null 2>&1 || true
    # 从候选列表中移除已处理的窗口，避免重复移动
    clients="$(echo "$clients" | jq --arg a "$addr" '[.[] | select(.address != $a)]')"
    moved=$((moved + 1))
  fi
done

notify-send -t 2000 "Workspace" "布局已恢复"
