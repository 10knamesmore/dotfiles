#!/usr/bin/env bash
set -euo pipefail

# ── 屏幕效果控制脚本 ──
# 管理护眼色温、胶片颗粒效果（强度/大小/暗部增强）
# 通过动态生成 GLSL shader 并热加载到 Hyprland 实现

CACHE_DIR="$HOME/.cache/hypr"
STATE_FILE="$CACHE_DIR/screen-effects.json"
STATE_BAK="$CACHE_DIR/screen-effects.json.bak"
SHADER_OUT="$CACHE_DIR/screen-effects.glsl"
TEMPLATE="$HOME/.config/hypr/screen-effects.glsl.template"

mkdir -p "$CACHE_DIR"

# ── 默认值 ──
DEFAULT_STATE='{"warmth":0,"grain":0,"grain_size":50,"shadow_boost":40}'

# ── 状态读写 ──

read_state() {
    if [[ -f "$STATE_FILE" ]]; then
        local raw
        raw="$(cat "$STATE_FILE")"
        # 兼容旧 JSON：补全缺失字段
        echo "$raw" | jq "{
      warmth: (.warmth // 0),
      grain: (.grain // 0),
      grain_size: (.grain_size // 50),
      shadow_boost: (.shadow_boost // 40)
    }"
    else
        echo "$DEFAULT_STATE"
    fi
}

write_state() {
    echo "$1" >"$STATE_FILE"
}

get_val() {
    echo "$1" | jq -r ".$2 // 0"
}

clamp() {
    local val="$1"
    ((val < 0)) && val=0
    ((val > 100)) && val=100
    echo "$val"
}

# ── Shader 生成与应用 ──

apply_shader() {
    local state="$1"
    local warmth grain grain_size shadow_boost
    warmth="$(get_val "$state" warmth)"
    grain="$(get_val "$state" grain)"
    grain_size="$(get_val "$state" grain_size)"
    shadow_boost="$(get_val "$state" shadow_boost)"

    if ((warmth == 0 && grain == 0)); then
        hyprctl keyword decoration:screen_shader "" >/dev/null 2>&1
        return
    fi

    if [[ ! -f "$TEMPLATE" ]]; then
        echo "error: template not found: $TEMPLATE" >&2
        exit 1
    fi

    # 将 0-100 映射到 0.0-1.0
    local w_f g_f gs_f sb_f
    w_f="$(awk "BEGIN { printf \"%.2f\", $warmth / 100.0 }")"
    g_f="$(awk "BEGIN { printf \"%.2f\", $grain / 100.0 }")"
    gs_f="$(awk "BEGIN { printf \"%.2f\", $grain_size / 100.0 }")"
    sb_f="$(awk "BEGIN { printf \"%.2f\", $shadow_boost / 100.0 }")"

    sed -e "s/__WARMTH__/$w_f/g" \
        -e "s/__GRAIN__/$g_f/g" \
        -e "s/__GRAIN_SIZE__/$gs_f/g" \
        -e "s/__SHADOW_BOOST__/$sb_f/g" \
        "$TEMPLATE" >"$SHADER_OUT"

    hyprctl keyword decoration:screen_shader "$SHADER_OUT" >/dev/null 2>&1
}

# ── 修改并应用单个参数 ──

set_param() {
    local state="$1" key="$2" val="$3"
    val="$(clamp "$val")"
    state="$(echo "$state" | jq --argjson v "$val" ".$key = \$v")"
    write_state "$state"
    apply_shader "$state"
}

# ── 循环值 ──

cycle_values() {
    local current="$1"
    shift
    local steps=("$@")
    local next="${steps[0]}"

    for i in "${!steps[@]}"; do
        if ((steps[i] == current)) && ((i + 1 < ${#steps[@]})); then
            next="${steps[i+1]}"
            break
        elif ((steps[i] == current)); then
            next="${steps[0]}"
            break
        elif ((steps[i] > current)); then
            next="${steps[i]}"
            break
        fi
    done
    echo "$next"
}

# ── 主逻辑 ──

cmd="${1:-}"
arg="${2:-}"

state="$(read_state)"
warmth="$(get_val "$state" warmth)"
grain="$(get_val "$state" grain)"
grain_size="$(get_val "$state" grain_size)"
shadow_boost="$(get_val "$state" shadow_boost)"

case "$cmd" in
warmth)
    set_param "$state" warmth "$((warmth + arg))"
    ;;
grain)
    set_param "$state" grain "$((grain + arg))"
    ;;
grain_size)
    set_param "$state" grain_size "$((grain_size + arg))"
    ;;
shadow_boost)
    set_param "$state" shadow_boost "$((shadow_boost + arg))"
    ;;

warmth-cycle)
    set_param "$state" warmth "$(cycle_values "$warmth" 0 30 60 90)"
    ;;
grain-cycle)
    set_param "$state" grain "$(cycle_values "$grain" 0 20 50 80)"
    ;;

apply)
    # 不修改 JSON，只读取当前状态重新生成+加载 shader（供 QuickShell 调用）
    apply_shader "$state"
    ;;

toggle)
    if ((warmth > 0 || grain > 0)); then
        cp "$STATE_FILE" "$STATE_BAK"
        state="$(echo "$state" | jq '.warmth = 0 | .grain = 0')"
        write_state "$state"
    elif [[ -f "$STATE_BAK" ]]; then
        state="$(cat "$STATE_BAK")"
        write_state "$state"
    else
        state='{"warmth":60,"grain":0,"grain_size":50,"shadow_boost":40}'
        write_state "$state"
    fi
    apply_shader "$state"
    ;;

brightness)
    # 同时调节笔记本屏幕 (amdgpu_bl1) 和外接显示器 (DDC/CI)
    if [[ "$arg" == +* || "$arg" == -* ]]; then
        brightnessctl -d amdgpu_bl1 s "${arg}%" >/dev/null 2>&1 || true
        # 外接显示器：读当前值，计算新值
        ddc_cur="$(ddcutil -d 1 getvcp 10 2>/dev/null | grep -oP 'current value =\s+\K\d+' || true)"
        if [[ -n "$ddc_cur" ]]; then
            ddc_new=$(( ddc_cur + ${arg} ))
            (( ddc_new < 0 )) && ddc_new=0
            (( ddc_new > 100 )) && ddc_new=100
            ddcutil -d 1 setvcp 10 "$ddc_new" >/dev/null 2>&1 || true
        fi
    else
        # 绝对值设置
        brightnessctl -d amdgpu_bl1 s "${arg}%" >/dev/null 2>&1 || true
        ddcutil -d 1 setvcp 10 "$arg" >/dev/null 2>&1 || true
    fi
    ;;

status)
    brightness="$(brightnessctl -d amdgpu_bl1 -m 2>/dev/null | cut -d, -f4 | tr -d '%' || echo "?")"

    text="☀${brightness}"

    tooltip="屏幕亮度: ${brightness}%\\n护眼色温: ${warmth}%\\n颗粒强度: ${grain}%\\n颗粒大小: ${grain_size}%\\n暗部增强: ${shadow_boost}%"

    echo "{\"text\":\"$text\",\"tooltip\":\"$tooltip\"}"
    ;;

*)
    echo "usage: $0 <warmth|grain|grain_size|shadow_boost|apply|toggle|brightness|status> [±value]" >&2
    exit 2
    ;;
esac
