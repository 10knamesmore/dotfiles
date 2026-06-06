#!/bin/sh

# Wi-Fi 显示网络名称，有线网络显示接口 IP，hover 时显示网络详情。

get_default_iface() {
  route -n get default 2>/dev/null | awk '/interface:/{print $2; exit}'
}

get_service_name() {
  networksetup -listnetworkserviceorder 2>/dev/null | awk -v iface="$1" '
    /^\([0-9]+\)/ {
      service=$0
      sub(/^\([0-9]+\) /, "", service)
      sub(/^\*/, "", service)
      next
    }
    /Device: / {
      device=$0
      sub(/^.*Device: /, "", device)
      sub(/\).*/, "", device)
      if (device == iface) {
        print service
        exit
      }
    }
  '
}

get_wifi_name() {
  wifi_output="$(networksetup -getairportnetwork "$1" 2>/dev/null)"
  case "$wifi_output" in
    "Current Wi-Fi Network: "*)
      printf '%s\n' "${wifi_output#Current Wi-Fi Network: }"
      ;;
  esac
}

get_dns_servers() {
  dns_servers="$(networksetup -getdnsservers "$1" 2>/dev/null)"
  case "$dns_servers" in
    ""|"There aren't any DNS Servers set on "*)
      printf '%s\n' "Automatic"
      ;;
    *)
      printf '%s\n' "$dns_servers" | awk 'BEGIN { first=1 } { if (!first) printf ", "; printf "%s", $0; first=0 } END { printf "\n" }'
      ;;
  esac
}

set_main_item() {
  iface="$(get_default_iface)"

  if [ -z "$iface" ]; then
    sketchybar --set "$NAME" icon="󰤮" label="Disconnected"
    return
  fi

  wifi_name="$(get_wifi_name "$iface")"
  ip_addr="$(ipconfig getifaddr "$iface" 2>/dev/null)"

  if [ -n "$wifi_name" ]; then
    label="$wifi_name"
    icon=""
  elif [ -n "$ip_addr" ]; then
    label="$ip_addr"
    icon="󰈀"
  else
    label="Disconnected"
    icon="󰤮"
  fi

  sketchybar --set "$NAME" icon="$icon" label="$label"
}

show_details() {
  iface="$(get_default_iface)"

  if [ -z "$iface" ]; then
    sketchybar --set network.details.title label="No active network" \
               --set network.details.interface label="" \
               --set network.details.ip label="" \
               --set network.details.gateway label="" \
               --set network.details.dns label="" \
               --set "$NAME" popup.drawing=on
    return
  fi

  service_name="$(get_service_name "$iface")"
  wifi_name="$(get_wifi_name "$iface")"
  ip_addr="$(ipconfig getifaddr "$iface" 2>/dev/null)"
  router_addr="$(route -n get default 2>/dev/null | awk '/gateway:/{print $2; exit}')"
  dns_servers="$(get_dns_servers "${service_name:-$iface}")"

  if [ -n "$wifi_name" ]; then
    network_label="Wi-Fi: $wifi_name"
  else
    network_label="Service: ${service_name:-Ethernet}"
  fi

  sketchybar --set network.details.title label="$network_label" \
             --set network.details.interface label="Interface: $iface" \
             --set network.details.ip label="IP: ${ip_addr:-Unavailable}" \
             --set network.details.gateway label="Gateway: ${router_addr:-Unavailable}" \
             --set network.details.dns label="DNS: $dns_servers" \
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
    set_main_item
    ;;
esac
