#!/usr/bin/env bash
# 同步设置笔记本内屏和 DDC/CI 外接显示器的亮度。
# Usage: screen_effects.sh brightness <0-100|+N|-N>
# 依赖：brightnessctl、ddcutil、grep
# 环境变量：无
# 未处理的命令失败或未定义变量会立即终止；硬件命令的容错在调用点显式处理。
set -euo pipefail

# 色温与颗粒效果由 `ScreenEffectsService.qml` 管理；本脚本只提供 brightness 命令。

cmd="${1:-}"
arg="${2:-}"

case "$cmd" in
brightness)
    if [[ "$arg" == +* || "$arg" == -* ]]; then
        brightnessctl -d amdgpu_bl1 s "${arg}%" >/dev/null 2>&1 || true
        # 外接显示器：DDC/CI 只能读-改-写，没有相对调节
        ddc_cur="$(ddcutil -d 1 getvcp 10 2>/dev/null | grep -oP 'current value =\s+\K\d+' || true)"
        if [[ -n "$ddc_cur" ]]; then
            ddc_new=$((ddc_cur + ${arg}))
            ((ddc_new < 0)) && ddc_new=0
            ((ddc_new > 100)) && ddc_new=100
            ddcutil -d 1 setvcp 10 "$ddc_new" >/dev/null 2>&1 || true
        fi
    else
        brightnessctl -d amdgpu_bl1 s "${arg}%" >/dev/null 2>&1 || true
        ddcutil -d 1 setvcp 10 "$arg" >/dev/null 2>&1 || true
    fi
    ;;

*)
    echo "usage: $0 brightness <0-100|±N>" >&2
    exit 2
    ;;
esac
