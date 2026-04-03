#!/usr/bin/env bash
set -euo pipefail

# Quick Note：如果已有 quick-note 窗口则聚焦，否则新建

note_dir="$HOME/notes"
mkdir -p "$note_dir"

existing="$(hyprctl -j clients 2>/dev/null | jq -r '.[] | select(.class == "quick-note") | .address' | head -1)"

if [[ -n "$existing" ]]; then
  hyprctl dispatch focuswindow "address:$existing" >/dev/null
else
  kitty --class quick-note -e nvim "$note_dir/quick-$(date +%Y%m%d).md" &
fi
