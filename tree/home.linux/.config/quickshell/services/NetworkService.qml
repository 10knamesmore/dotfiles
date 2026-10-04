pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// 默认 IPv4 网卡的状态；libnm 和内核路由通知驱动更新，不轮询外部命令。
Singleton {
    id: root

    property var snapshot: ({
        connectionType: "disconnected",
        interfaceName: "",
        address: "",
        ssid: "",
        signalStrength: 0
    })
    readonly property string connectionType: snapshot.connectionType
    readonly property string interfaceName: snapshot.interfaceName
    readonly property string address: snapshot.address
    readonly property string ssid: snapshot.ssid
    readonly property int signalStrength: snapshot.signalStrength
    readonly property bool disconnected: connectionType === "disconnected"
    readonly property string statusIcon: disconnected ? "󰤮" : connectionType === "wifi" ? ["󰤟", "󰤢", "󰤥", "󰤨"][Math.min(3, Math.floor(signalStrength / 25))] : "󰈀"

    Process {
        command: ["/usr/bin/python3", "-u", Quickshell.shellPath("services/network-status/watch.py")]
        running: true

        stdout: SplitParser {
            onRead: line => root.snapshot = JSON.parse(line)
        }
        stderr: SplitParser {
            onRead: line => console.info(line)
        }
        onExited: (exitCode, exitStatus) => console.error("[network-status] collector exited", exitCode, exitStatus)
    }
}
