pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// 默认 IPv4 网卡的状态；由 Rust 后端的 NetworkManager D-Bus 和 rtnetlink 事件驱动更新。
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
        command: ["desktop-backend", "network-status"]
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
