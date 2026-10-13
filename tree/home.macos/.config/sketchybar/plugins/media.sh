#!/bin/sh

# 只查询正在运行的播放器；无曲目时隐藏中间媒体项。
for APP in Music Spotify NeteaseMusic
do
  pgrep -x "$APP" >/dev/null || continue
  TRACK_INFO="$(osascript \
    -e "if application \"$APP\" is running then" \
    -e "tell application \"$APP\"" \
    -e "if (player state is playing) or (player state is paused) then" \
    -e "return artist of current track & \" - \" & name of current track" \
    -e "end if" \
    -e "end tell" \
    -e "end if" 2>/dev/null)"

  if [ -n "$TRACK_INFO" ]; then
    sketchybar --set "$NAME" drawing=on label="$TRACK_INFO"
    exit 0
  fi
done

sketchybar --set "$NAME" drawing=off
