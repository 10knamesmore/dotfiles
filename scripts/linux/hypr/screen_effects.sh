#!/usr/bin/env bash
# 同步设置笔记本内屏和 DDC/CI 外接显示器的亮度。
# Usage: screen_effects.sh brightness <0-100|+N|-N>   # 相对值以内屏当前值为基准，两屏最终同值
#        screen_effects.sh dim                        # 空闲时两屏压暗，记住原值
#        screen_effects.sh undim                      # 恢复 dim 前的原值
# 依赖：brightnessctl、ddcutil、grep、flock
# 未处理的命令失败或未定义变量会立即终止；硬件命令的容错在调用点显式处理。
set -euo pipefail

# 色温与颗粒效果由 `ScreenEffectsService.qml` 管理；本脚本只提供背光命令。

# 外接屏是 `ddcutil detect` 里的 Display 1（card0-DP-3）；内屏是笔记本屏，不支持 DDC/CI。
readonly ddc_display=1
readonly state_dir="$XDG_RUNTIME_DIR/screen-effects"
readonly ddc_saved_file="$state_dir/dimmed-ddc-brightness"
readonly dim_percent=10

# 桌面与锁屏的空闲监听器、手动调整共享亮度状态，串行完成保存、写入和恢复。
mkdir -p "$state_dir"
exec 9>"$state_dir/brightness.lock"
flock 9

# 亮度基准取内屏：外接屏可能被自带 OSD/按键单独改过，每次写入都以它为准把两屏拉回同步。
internal_percent() {
    brightnessctl -d amdgpu_bl1 -m | cut -d, -f4 | tr -d '%'
}

clamp_percent() {
    local percent="$1"
    if ((percent < 0)); then percent=0; fi
    if ((percent > 100)); then percent=100; fi
    printf '%s' "$percent"
}

set_both() {
    local percent="$1"
    brightnessctl -d amdgpu_bl1 s "${percent}%" >/dev/null 2>&1 || true
    ddcutil -d "$ddc_display" setvcp 10 "$percent" >/dev/null 2>&1 || true
}

ddc_percent() {
    ddcutil -d "$ddc_display" getvcp 10 2>/dev/null |
        grep -oP 'current value =\s+\K\d+' || true
}

restore_both() {
    if [[ -f "$ddc_saved_file" ]]; then
        brightnessctl -d amdgpu_bl1 -r >/dev/null 2>&1 || true
        local saved
        saved="$(<"$ddc_saved_file")"
        if [[ -n "$saved" ]]; then
            ddcutil -d "$ddc_display" setvcp 10 "$saved" >/dev/null 2>&1 || true
        fi
        rm "$ddc_saved_file"
    fi
}

cmd="${1:-}"
arg="${2:-}"

case "$cmd" in
brightness)
    # 手动调整结束本次调暗；稍后到达的 on-resume 不再覆盖用户选择。
    restore_both
    if [[ "$arg" == +* || "$arg" == -* ]]; then
        arg=$(clamp_percent "$(($(internal_percent) + arg))")
    else
        arg=$(clamp_percent "$arg")
    fi
    set_both "$arg"
    ;;

dim)
    # 同一段空闲可能触发多个监听器，只保存第一次调暗前的亮度。
    if [[ -f "$ddc_saved_file" ]]; then
        exit 0
    fi
    brightnessctl -d amdgpu_bl1 -s set "${dim_percent}%" >/dev/null 2>&1 || true
    saved="$(ddc_percent)"
    printf '%s\n' "$saved" >"$ddc_saved_file"
    if [[ -n "$saved" ]]; then
        ddcutil -d "$ddc_display" setvcp 10 "$dim_percent" >/dev/null 2>&1 || true
    fi
    ;;

undim)
    restore_both
    ;;

*)
    echo "usage: $0 brightness <0-100|±N> | dim | undim" >&2
    exit 2
    ;;
esac
