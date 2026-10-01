pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property string connectionType: "disconnected"
    property string interfaceName: ""
    property string address: ""
    property string ssid: ""
    property int signalStrength: 0
    readonly property bool disconnected: connectionType === "disconnected"

    Process {
        id: reader

        command: ["bash", "-c", `
            set -euo pipefail
            export LC_ALL=C
            iface=$(ip route show default | awk 'NR == 1 {print $5}')
            if [[ -z "$iface" ]]; then
                printf 'disconnected\\n'
                exit
            fi
            if [[ -d "/sys/class/net/$iface/wireless" ]]; then
                printf 'wifi\\n'
            else
                printf 'ethernet\\n'
            fi
            printf '%s\\n' "$iface"
            ip -4 -o addr show dev "$iface" | awk 'BEGIN {ORS=""} /inet / {print $4 " "} END {print "\\n"}'
            if [[ -d "/sys/class/net/$iface/wireless" ]]; then
                nmcli -t --escape no -f ACTIVE,SIGNAL,SSID dev wifi list ifname "$iface" --rescan no
            fi
        `]

        stdout: StdioCollector {
            onStreamFinished: {
                const lines = text.split("\n");
                const activeNetwork = lines.slice(3).find(line => line.startsWith("yes:"));
                root.connectionType = lines[0];
                root.interfaceName = lines[1] ?? "";
                root.address = (lines[2] ?? "").trim();
                root.ssid = activeNetwork ? activeNetwork.substring(activeNetwork.indexOf(":", 4) + 1) : "";
                root.signalStrength = activeNetwork ? Number(activeNetwork.split(":")[1]) : 0;
            }
        }

        stderr: StdioCollector {
            onStreamFinished: {
                if (text.trim() !== "")
                    console.warn("Network status:", text.trim());
            }
        }

        onExited: (exitCode, exitStatus) => {
            if (exitCode !== 0 || exitStatus !== 0)
                console.warn("Network status collection failed:", exitCode, exitStatus);
        }
    }

    Timer {
        interval: 5000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: reader.running = true
    }
}
