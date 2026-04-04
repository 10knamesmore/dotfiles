#!/usr/bin/env bash
set -euo pipefail

# Network status for QuickShell NetworkModule
# Output: JSON {"icon","value","tooltip","class"}

iface=$(ip route 2>/dev/null | awk '/^default/ {print $5; exit}')
if [[ -z "$iface" ]]; then
    echo '{"icon":"󰤮","value":"Disconnected","class":"disconnected"}'
    exit
fi

if [[ -d "/sys/class/net/$iface/wireless" ]]; then
    # Waybar: format-wifi "  {essid} ({signalStrength}%)"
    line=$(LANG=C nmcli -t -f active,ssid,signal dev wifi 2>/dev/null | grep "^yes" | head -1)
    if [[ -n "$line" ]]; then
        ssid=$(echo "$line" | cut -d: -f2)
        sig=$(echo "$line" | cut -d: -f3)
    else
        ssid=$(LANG=C nmcli -t -f NAME connection show --active 2>/dev/null | head -1)
        sig=$(awk 'NR>2{print int($3)}' /proc/net/wireless 2>/dev/null | head -1)
        [[ -z "$sig" ]] && sig=0
    fi
    ip=$(ip -4 addr show dev "$iface" 2>/dev/null | awk '/inet / {print $2}')
    echo '{"icon":"","value":"'"$ssid"' ('"$sig"'%)","tooltip":"SSID: '"$ssid"'\n信号: '"$sig"'%\nIP: '"$ip"'\n接口: '"$iface"'","class":"wifi"}'
else
    ip=$(ip -4 addr show dev "$iface" 2>/dev/null | awk '/inet / {print $2}')
    echo '{"icon":"󰈁","value":"'"$iface"'","tooltip":"以太网: '"$iface"'\nIP: '"$ip"'","class":"ethernet"}'
fi
