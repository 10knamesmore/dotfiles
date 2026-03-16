#!/bin/sh

# Wi-Fi 显示网络名称，有线网络显示接口 IP，断网时显示断开状态。

IFACE="$(route -n get default 2>/dev/null | awk '/interface:/{print $2; exit}')"

if [ -z "$IFACE" ]; then
  sketchybar --set "$NAME" icon="󰤮" label="Disconnected"
  exit 0
fi

WIFI_OUTPUT="$(networksetup -getairportnetwork "$IFACE" 2>/dev/null)"
case "$WIFI_OUTPUT" in
  "Current Wi-Fi Network: "*)
    WIFI_NAME="${WIFI_OUTPUT#Current Wi-Fi Network: }"
    ;;
  *)
    WIFI_NAME=""
    ;;
esac

IP_ADDR="$(ipconfig getifaddr "$IFACE" 2>/dev/null)"

if [ -n "$WIFI_NAME" ] && [ "$WIFI_NAME" != "You are not associated with an AirPort network." ]; then
  LABEL="$WIFI_NAME"
  ICON=""
elif [ -n "$IP_ADDR" ]; then
  LABEL="$IP_ADDR"
  ICON="󰈀"
else
  LABEL="Disconnected"
  ICON="󰤮"
fi

sketchybar --set "$NAME" icon="$ICON" label="$LABEL"
