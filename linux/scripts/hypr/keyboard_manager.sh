#!/usr/bin/env bash
set -euo pipefail

# 外接键盘插拔时，禁用/启用笔记本内置键盘
# 由 udev 规则调用：keyboard_manager.sh enable | disable
#
# udev 规则安装方法：
#   sudo cp /path/to/99-keyboard-inhibit.rules /etc/udev/rules.d/
#   sudo udevadm control --reload-rules

# 需要 inhibit 的所有内置键盘设备名（完整匹配）
LAPTOP_KB_NAMES=(
  "AT Translated Set 2 keyboard"
  "ITE Tech. Inc. ITE Device(8296) Keyboard"
)

# 对所有匹配的内置键盘执行 enable/disable
set_inhibit() {
  local val="$1"  # 0 或 1
  for kb_name in "${LAPTOP_KB_NAMES[@]}"; do
    for name_file in /sys/class/input/input*/name; do
      if [[ "$(cat "$name_file" 2>/dev/null)" == "$kb_name" ]]; then
        local p="$(dirname "$name_file")/inhibited"
        [[ -f "$p" ]] && echo "$val" > "$p"
      fi
    done
  done
}

# 已知外接键盘 vendor:product 列表
EXTERNAL_KEYBOARDS=(
  "320f:5088"  # Telink Wireless Gaming Keyboard
  "1313:4122"  # Redox Customized
)

# 检查是否有已知外接键盘当前已连接
external_connected() {
  for id in "${EXTERNAL_KEYBOARDS[@]}"; do
    local vendor="${id%%:*}"
    local product="${id##*:}"
    for v_file in /sys/bus/usb/devices/*/idVendor; do
      local dir="$(dirname "$v_file")"
      if [[ "$(cat "$v_file" 2>/dev/null)" == "$vendor" ]] && \
         [[ "$(cat "$dir/idProduct" 2>/dev/null)" == "$product" ]]; then
        return 0
      fi
    done
  done
  return 1
}

action="${1:-}"

case "$action" in
  disable) set_inhibit 1 ;;
  enable)  set_inhibit 0 ;;
  sync)
    if external_connected; then
      set_inhibit 1
    else
      set_inhibit 0
    fi
    ;;
  *)
    echo "usage: $0 <enable|disable|sync>" >&2
    exit 2
    ;;
esac
