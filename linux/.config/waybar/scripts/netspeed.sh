#!/usr/bin/env bash
set -euo pipefail

# Prefer the interface on default route; fallback to first active non-lo interface.
iface="$(ip route 2>/dev/null | awk '/^default/ {print $5; exit}' || true)"
if [[ -z "${iface}" ]]; then
  iface="$(ip -o link show up 2>/dev/null | awk -F': ' '$2 != "lo" {print $2; exit}' || true)"
fi

if [[ -z "${iface}" ]] || [[ ! -d "/sys/class/net/${iface}" ]]; then
  echo '{"text":"󰤮 N/A", "tooltip":"No active interface"}'
  exit 0
fi

rx_now="$(cat "/sys/class/net/${iface}/statistics/rx_bytes")"
tx_now="$(cat "/sys/class/net/${iface}/statistics/tx_bytes")"
ts_now="$(date +%s)"

state_file="/tmp/waybar-netspeed-${iface}.state"
if [[ -f "${state_file}" ]]; then
  read -r ts_prev rx_prev tx_prev < "${state_file}" || true
else
  ts_prev="${ts_now}"
  rx_prev="${rx_now}"
  tx_prev="${tx_now}"
fi

printf '%s %s %s\n' "${ts_now}" "${rx_now}" "${tx_now}" > "${state_file}"

dt=$((ts_now - ts_prev))
if (( dt <= 0 )); then
  dt=1
fi

drx=$((rx_now - rx_prev))
dtx=$((tx_now - tx_prev))
if (( drx < 0 )); then drx=0; fi
if (( dtx < 0 )); then dtx=0; fi

rx_rate=$((drx / dt))
tx_rate=$((dtx / dt))

human_rate() {
  local bytes="$1"
  if (( bytes >= 1073741824 )); then
    awk -v b="$bytes" 'BEGIN { printf "%.1f GB/s", b/1073741824 }'
  elif (( bytes >= 1048576 )); then
    awk -v b="$bytes" 'BEGIN { printf "%.1f MB/s", b/1048576 }'
  elif (( bytes >= 1024 )); then
    awk -v b="$bytes" 'BEGIN { printf "%.1f KB/s", b/1024 }'
  else
    printf "%d B/s" "$bytes"
  fi
}

rx_h="$(human_rate "$rx_rate")"
tx_h="$(human_rate "$tx_rate")"

text="󰁅 ${rx_h} 󰕒 ${tx_h}"
tooltip="Interface: ${iface}&#10;Download: ${rx_h}&#10;Upload: ${tx_h}"

printf '{"text":"%s", "tooltip":"%s"}\n' "$text" "$tooltip"
