#!/bin/sh

# 通过 AppleScript 读取支持的播放器元数据。
for APP in Music Spotify NeteaseMusic
do
  TRACK_INFO="$(osascript \
    -e "if application \"$APP\" is running then" \
    -e "tell application \"$APP\"" \
    -e "if (player state is playing) or (player state is paused) then" \
    -e "return artist of current track & \" - \" & name of current track" \
    -e "end if" \
    -e "end tell" \
    -e "end if" 2>/dev/null)"

  if [ -n "$TRACK_INFO" ]; then
    sketchybar --set "$NAME" label="$TRACK_INFO"
    exit 0
  fi
done

sketchybar --set "$NAME" label="No media"
