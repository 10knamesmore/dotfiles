#!/usr/bin/env bash
# Notification hook（matcher: idle_prompt）：Claude 完成、等待你输入时发系统通知。
# terminal-notifier 优先（无 Script Editor 授权坑、可点击跳回）；否则 macOS osascript / Linux notify-send。
# 始终 exit 0——Notification 事件不可阻断，退出码也被忽略。

input=$(cat)
dir=$(printf '%s' "$input" | jq -r '.cwd // empty' 2>/dev/null)
note=$(printf '%s' "$input" | jq -r '.message // empty' 2>/dev/null)
msg="${note:-等待你的输入} · ${dir##*/}"

if command -v terminal-notifier >/dev/null 2>&1; then
    terminal-notifier -title "Claude Code" -message "$msg" -sound default >/dev/null 2>&1
elif [ "$(uname)" = Darwin ]; then
    safe=$(printf '%s' "$msg" | tr -d '"\\') # osascript 字符串里引号/反斜杠会破坏语法
    osascript -e "display notification \"$safe\" with title \"Claude Code\"" 2>/dev/null
elif command -v notify-send >/dev/null 2>&1; then
    notify-send "Claude Code" "$msg"
fi
exit 0
