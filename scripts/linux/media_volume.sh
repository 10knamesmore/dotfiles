#!/usr/bin/zsh

# 获取当前播放信息
title=$(playerctl metadata title 2>/dev/null)
artist=$(playerctl metadata artist 2>/dev/null)
playing=$(playerctl status 2>/dev/null)
# 获取音量（百分比）
volume=$(pactl get-sink-volume @DEFAULT_SINK@ | grep -oP '\d+%' | head -1 | grep -oP '\d+')
# 判断是否静音
mute=$(pactl get-sink-mute @DEFAULT_SINK@ | grep -oP 'yes|no')
# is_muted=false
if [[ "$mute" == "yes" ]]; then
    # is_muted=true
    class="muted"
else
    class="unmuted"
fi

# 播放状态图标和CSS类
if [[ "$playing" == "Playing" ]]; then
    play_class="playing"
    status_icon="󰏤" # pause
elif [[ "$playing" == "Paused" ]]; then
    play_class="paused"
    status_icon="󰐊" # play
else
    play_class="no-player"
    status_icon="󰓛" # stop
fi

# 组合显示文本
if [[ -z "$title" ]]; then
    media_content="无播放内容"
else
    if [[ -n "$artist" ]]; then
        media_content="$artist - $title"
    else
        media_content="$title"
    fi
fi

# 创建工具提示内容（显示更详细信息）
tooltip="播放状态: ${playing:-无播放器}
音量:    ${volume}%
当前播放: ${media_content}"

# 为简洁的显示准备文本
# 截断过长的标题
if [[ ${#media_content} -gt 20 ]]; then
    display_text="$status_icon ${media_content:0:20}..."
else
    display_text="$status_icon $media_content"
fi

# 合并CSS类
combined_class="$play_class $class"

# 使用 jq 输出Waybar需要的标准JSON格式
jq --unbuffered --compact-output -n \
    --arg text "$display_text" \
    --arg tooltip "$tooltip" \
    --arg class "$combined_class" \
    --arg alt "$playing" \
    --argjson percentage "$volume" \
    '{"text": $text, "alt": $alt, "tooltip": $tooltip, "class": $class, "percentage": $percentage}'
