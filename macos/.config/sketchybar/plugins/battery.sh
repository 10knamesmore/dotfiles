#!/bin/sh

# 显示当前电池电量百分比，并在充电时切换为充电图标。

PERCENTAGE="$(pmset -g batt | grep -Eo "\d+%" | cut -d% -f1)"
CHARGING="$(pmset -g batt | grep 'AC Power')"

if [ "$PERCENTAGE" = "" ]; then
    sketchybar --set "$NAME" icon="" label="NoBattery"
    exit 0
fi

case "${PERCENTAGE}" in
9[0-9] | 100)
    ICON=""
    ;;
[6-8][0-9])
    ICON=""
    ;;
[3-5][0-9])
    ICON=""
    ;;
[1-2][0-9])
    ICON=""
    ;;
*) ICON="" ;;
esac

if [[ "$CHARGING" != "" ]]; then
    ICON=""
fi

sketchybar --set "$NAME" icon="$ICON" label="${PERCENTAGE}%"
