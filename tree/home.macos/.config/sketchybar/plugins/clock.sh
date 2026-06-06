#!/bin/sh

# 以 24 小时制显示中间时钟，hover 时显示日期和星期。

show_details() {
  sketchybar --set clock.details.date label="$(date '+%Y-%m-%d')" \
             --set clock.details.weekday label="$(date '+%A')" \
             --set "$NAME" popup.drawing=on
}

hide_details() {
  sketchybar --set "$NAME" popup.drawing=off
}

case "$SENDER" in
  "mouse.entered")
    show_details
    ;;
  "mouse.exited"|"mouse.exited.global")
    hide_details
    ;;
  *)
    sketchybar --set "$NAME" label="$(date '+%H:%M:%S')"
    ;;
esac
