pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// 网络状态单例。每五秒全局运行一次 network_status.sh，NetworkModule 只读取本单例，
// 因而多显示器不会重复采集同一份主机状态。
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
