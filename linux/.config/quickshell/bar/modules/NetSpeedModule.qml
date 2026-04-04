import "../../theme"
import "../components"
import QtQuick
import Quickshell
import Quickshell.Io

// 网络速度（复用 waybar 的 netspeed.sh）
BarModule {
    id: root

    property string direction: "up" // "up" 或 "down"
    property string displayText: (direction === "up" ? "󰕒" : "󰁅") + " --"
    property string tooltipText: ""

    accentColor: Colors.teal
    implicitWidth: Math.max(label.implicitWidth + 32, 120)
    Component.onCompleted: reader.running = true

    Process {
        id: reader

        command: [Quickshell.env("HOME") + "/.config/waybar/scripts/netspeed.sh", root.direction]

        stdout: SplitParser {
            onRead: (data) => {
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
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        font.weight: Font.DemiBold
    }

}
