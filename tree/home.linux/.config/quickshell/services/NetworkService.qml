pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// 网络状态单例 —— 全局只 fork 一次 network_status.sh（内含 nmcli/ip 多个子进程），
// 取代原 NetworkModule 在每块显示器各自 5s 轮询同一份主机全局数据的浪费（per-screen ×屏数）。
// UI（NetworkModule）只读本单例（照 AudioService/MediaService/SystemStatsService 的收口模式）。
Singleton {
    id: root

    property string iconText: "󰤮"
    property string valueText: "…"
    property string tooltipText: ""
    property bool disconnected: false

    Process {
        id: reader

        command: [Quickshell.env("DOTS_SCRIPTS") + "/network_status.sh"]

        stdout: SplitParser {
            onRead: (data) => {
                try {
                    let obj = JSON.parse(data);
                    root.iconText = obj.icon ?? "󰤮";
                    root.valueText = obj.value ?? "";
                    root.tooltipText = obj.tooltip ?? "";
                    root.disconnected = obj.class === "disconnected";
                } catch (e) {
                    root.valueText = data;
                }
            }
        }
    }

    Timer {
        interval: 5000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            reader.running = false;
            reader.running = true;
        }
    }
}
