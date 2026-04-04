import QtQuick
import Quickshell
import Quickshell.Io
import "../components"
import "../../theme"

// 网络速度（复用 waybar 的 netspeed.sh）
BarModule {
    id: root
    accentColor: Colors.teal
    implicitWidth: Math.max(label.implicitWidth + 32, 120)

    property string direction: "up"   // "up" 或 "down"
    property string displayText: (direction === "up" ? "󰕒" : "󰁅") + " --"
    property string tooltipText: ""

    Process {
        id: reader
        command: [Quickshell.env("HOME") + "/.config/waybar/scripts/netspeed.sh", root.direction]
        stdout: SplitParser {
            onRead: data => {
                try {
                    let obj = JSON.parse(data);
                    root.displayText = obj.text ?? data;
                    root.tooltipText = obj.tooltip ?? "";
                } catch (e) {
                    root.displayText = data;
                }
            }
        }
    }

    Component.onCompleted: reader.running = true

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            reader.running = false;
            reader.running = true;
        }
    }

    Text {
        id: label
        anchors.centerIn: parent
        text: root.displayText
        color: Colors.teal
        font.family: "Hack Nerd Font"
        font.pixelSize: 13
        font.weight: Font.DemiBold
    }
}
