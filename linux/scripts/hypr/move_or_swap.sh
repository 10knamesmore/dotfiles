#!/usr/bin/env bash

opposite() {
    case "$1" in
        l|left) echo r ;;
        r|right) echo l ;;
        u|up) echo d ;;
        d|down) echo u ;;
    esac
}

dir="$1"

# 获取当前窗口信息
win=$(hyprctl -j activewindow)
old_x=$(echo "$win" | jq '.at[0]')
old_y=$(echo "$win" | jq '.at[1]')

# 尝试移动窗口
hyprctl dispatch movewindow "$dir"

sleep 0.02

# 再检查位置
win2=$(hyprctl -j activewindow)
new_x=$(echo "$win2" | jq '.at[0]')
new_y=$(echo "$win2" | jq '.at[1]')

# 如果位置相同，说明无法移动 → 执行 swap
if [ "$old_x" = "$new_x" ] && [ "$old_y" = "$new_y" ]; then
    hyprctl dispatch swapwindow "$dir"
fi
