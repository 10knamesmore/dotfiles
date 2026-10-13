#!/bin/sh

# 显示电池电量和充电状态；台式机不显示电池项。

PERCENTAGE="$(pmset -g batt | grep -Eo "\d+%" | cut -d% -f1)"
CHARGING="$(pmset -g batt | grep 'AC Power')"

if [ "$PERCENTAGE" = "" ]; then
    sketchybar --set "$NAME" drawing=off
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

sketchybar --set "$NAME" drawing=on icon="$ICON" label="${PERCENTAGE}%"
