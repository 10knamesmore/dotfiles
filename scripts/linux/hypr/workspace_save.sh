#!/usr/bin/env bash
set -euo pipefail

# 保存当前所有窗口的 class + workspace 映射到 JSON

cache_dir="$HOME/.cache/hypr"
mkdir -p "$cache_dir"
output="$cache_dir/workspace-layout.json"

hyprctl -j clients 2>/dev/null | jq '[.[] | select(.mapped == true) | {
  class: .class,
  workspace: .workspace.id,
  floating: .floating,
  title: .title
}]' > "$output"

count="$(jq 'length' "$output")"
notify-send -t 2000 "Workspace" "已保存 $count 个窗口的布局"
