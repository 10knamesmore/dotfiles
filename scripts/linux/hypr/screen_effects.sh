#!/usr/bin/env bash
set -euo pipefail

# ── 屏幕背光调节 ──
# 同时调节笔记本内屏（backlight class）和外接显示器（DDC/CI）。
#
# 注：色温 / 胶片颗粒的 shader 生成与热加载已整体搬进 QuickShell，见
#   ~/.config/quickshell/services/ScreenEffectsService.qml     （状态、IPC）
#   ~/.config/quickshell/screen-effects/lib/shaderGen.js       （黑体色温、源码拼装）
#   ~/.config/quickshell/screen-effects/screen-effects.frag    （GLSL 主体）
# 本脚本不再参与，也不再有 apply/toggle/warmth/grain 等子命令 —— 那些当年是给
# waybar + 快捷键写的，面板化之后就没有调用方了。
#
# 背光留在脚本里是因为它跟 shader 无关：走 brightnessctl（内屏）和 ddcutil（外接），
# 都是需要 spawn 的外部命令，QML 侧包一层没有收益。

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
