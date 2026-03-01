#!/usr/bin/env bash
set -euo pipefail

mode="${1:-}"   # shift | ctrl
key="${2:-}"    # h|j|k|l

if [[ -z "$mode" || -z "$key" ]]; then
  echo "usage: $0 <shift|ctrl> <h|j|k|l>" >&2
  exit 2
fi

# 将 vim 方向键映射为 Hyprland 方向参数。
case "$key" in
  h) dir="l" ;;
  j) dir="d" ;;
  k) dir="u" ;;
  l) dir="r" ;;
  *)
    echo "invalid key: $key" >&2
    exit 2
    ;;
esac

# 解析脚本目录，确保可以稳定调用同目录下其他脚本。
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 读取当前布局；hyprctl 不可用或报错时回退为空字符串。
layout="$(hyprctl -j getoption general:layout 2>/dev/null | jq -r '.str // empty' || true)"

case "$mode" in
  shift)
    case "$layout" in
      dwindle)
        # dwindle 下保持原有 move+swap 行为。
        "$script_dir/move_or_swap.sh" "$dir"
        ;;
      scrolling)
        # scrolling 下先在当前布局内移动。
        # 如果窗口坐标没变（说明撞到边缘），则按方向移动到相邻显示器。
        win_before="$(hyprctl -j activewindow 2>/dev/null || true)"
        old_x="$(echo "$win_before" | jq -r '.at[0] // empty')"
        old_y="$(echo "$win_before" | jq -r '.at[1] // empty')"

        hyprctl dispatch movewindow "$dir" >/dev/null 2>&1 || true

        win_after="$(hyprctl -j activewindow 2>/dev/null || true)"
        new_x="$(echo "$win_after" | jq -r '.at[0] // empty')"
        new_y="$(echo "$win_after" | jq -r '.at[1] // empty')"

        if [[ -n "$old_x" && -n "$old_y" && "$old_x" == "$new_x" && "$old_y" == "$new_y" ]]; then
          "$script_dir/move_window_to_monitor.sh" "$dir" || true
        fi
        ;;
      *)
        # 未知布局：使用稳妥的默认行为。
        "$script_dir/move_or_swap.sh" "$dir"
        ;;
    esac
    ;;
  ctrl)
    case "$layout" in
      scrolling)
        # scrolling：左右用 colresize，上下用动画 resizeactive。
        case "$key" in
          h) hyprctl dispatch layoutmsg "colresize -0.05" ;;
          l) hyprctl dispatch layoutmsg "colresize +0.05" ;;
          j) "$script_dir/resizeactive_animated.sh" down ;;
          k) "$script_dir/resizeactive_animated.sh" up ;;
        esac
        ;;
      dwindle)
        # dwindle：使用传统像素级 resizeactive。
        case "$key" in
          h) hyprctl dispatch resizeactive "-40 0" ;;
          l) hyprctl dispatch resizeactive "40 0" ;;
          j) hyprctl dispatch resizeactive "0 40" ;;
          k) hyprctl dispatch resizeactive "0 -40" ;;
        esac
        ;;
      *)
        # 回退到现有默认行为。
        case "$key" in
          h) hyprctl dispatch layoutmsg "colresize -0.05" ;;
          l) hyprctl dispatch layoutmsg "colresize +0.05" ;;
          j) "$script_dir/resizeactive_animated.sh" down ;;
          k) "$script_dir/resizeactive_animated.sh" up ;;
        esac
        ;;
    esac
    ;;
  *)
    echo "invalid mode: $mode" >&2
    exit 2
    ;;
esac
