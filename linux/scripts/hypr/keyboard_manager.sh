#!/usr/bin/env bash
set -euo pipefail

# 外接键盘插拔时，禁用/启用笔记本内置键盘
# 由 udev 规则调用：keyboard_manager.sh enable | disable
#
# udev 规则安装方法：
#   sudo cp /path/to/99-keyboard-inhibit.rules /etc/udev/rules.d/
#   sudo udevadm control --reload-rules

LAPTOP_KB_NAME="AT Translated Set 2 keyboard"

# 通过设备名动态查找 sysfs inhibited 路径
find_inhibit_path() {
  for name_file in /sys/class/input/input*/name; do
    if [[ "$(cat "$name_file" 2>/dev/null)" == "$LAPTOP_KB_NAME" ]]; then
      local p="$(dirname "$name_file")/inhibited"
      [[ -f "$p" ]] && echo "$p" && return 0
    fi
  done
  return 1
}

action="${1:-}"

case "$action" in
  disable)
    path="$(find_inhibit_path)" || exit 0
    echo 1 > "$path"
    ;;
  enable)
    path="$(find_inhibit_path)" || exit 0
    echo 0 > "$path"
    ;;
  *)
    echo "usage: $0 <enable|disable>" >&2
    exit 2
    ;;
esac
