#!/usr/bin/env bash
# 按当前布局移动窗口或调整尺寸。
# Usage: layout_dispatch.sh <shift|ctrl> <h|j|k|l>
# 依赖：hyprctl、jq；通过继承的 Hyprland 环境连接当前会话。
# 未处理的命令失败时停止，避免继续操作窗口。
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
        win_before="$(hyprctl -j activewindow 2>/dev/null || true)"

        if [[ "$key" == h || "$key" == l ]] && jq -e '.floating == false' <<< "$win_before" >/dev/null; then
          # 堆叠时先拆成相邻独立列；独占一列时并入邻列底部。
          case "$key" in
            h) column_direction="prev" ;;
            l) column_direction="next" ;;
          esac
          move_result="$(hyprctl dispatch "hl.dsp.layout('consume_or_expel $column_direction')")"
          # 无相邻列时保留跨显示器移动；成功时不以屏幕坐标判断，避免视口滚动造成误判。
          if [[ "$move_result" == "ok" ]]; then
            # consume_or_expel 不发送布局事件；完成后通知顶栏刷新窗口几何。
            hyprctl dispatch "hl.dsp.event('window-layout-changed')" >/dev/null
          else
            "$script_dir/move_window_to_monitor.sh" "$dir" || true
          fi
          exit 0
        fi

        # 上下移动和浮动窗口沿用方向移动；坐标未变时尝试相邻显示器。
        old_x="$(echo "$win_before" | jq -r '.at[0] // empty')"
        old_y="$(echo "$win_before" | jq -r '.at[1] // empty')"

        hyprctl dispatch "hl.dsp.window.move({ direction = '$dir' })" >/dev/null 2>&1 || true

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
          h) hyprctl dispatch "hl.dsp.layout('colresize -0.05')" ;;
          l) hyprctl dispatch "hl.dsp.layout('colresize +0.05')" ;;
          j) "$script_dir/resizeactive_animated.sh" down ;;
          k) "$script_dir/resizeactive_animated.sh" up ;;
        esac
        ;;
      dwindle)
        # dwindle：使用传统像素级 resizeactive。
        case "$key" in
          h) hyprctl dispatch "hl.dsp.window.resize({ x = -40, y = 0, relative = true })" ;;
          l) hyprctl dispatch "hl.dsp.window.resize({ x = 40,  y = 0, relative = true })" ;;
          j) hyprctl dispatch "hl.dsp.window.resize({ x = 0,   y = 40, relative = true })" ;;
          k) hyprctl dispatch "hl.dsp.window.resize({ x = 0,   y = -40, relative = true })" ;;
        esac
        ;;
      *)
        # 回退到现有默认行为。
        case "$key" in
          h) hyprctl dispatch "hl.dsp.layout('colresize -0.05')" ;;
          l) hyprctl dispatch "hl.dsp.layout('colresize +0.05')" ;;
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
