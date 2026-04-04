import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland._Ipc
import "../components"
import "../../theme"

// 滚动布局列状态（复用 waybar 的 scroll_status.sh，传递 WAYBAR_OUTPUT_NAME）
BarModule {
    id: root
    accentColor: Colors.yellow
    implicitWidth: Math.max(label.implicitWidth + 32, 80)

    property var barScreen: null
    property string displayText: "…"
    property string tooltipText: ""

    Process {
        id: reader
        command: [Quickshell.env("HOME") + "/.config/waybar/scripts/scroll_status.sh"]
        environment: ({
                "WAYBAR_OUTPUT_NAME": root.barScreen ? root.barScreen.name : ""
            })
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
        interval: 250
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
        color: Colors.yellow
        font.family: "Hack Nerd Font"
        font.pixelSize: 13
        font.weight: Font.DemiBold
    }

    onScrolled: delta => {
        let arg = delta > 0 ? "colresize +0.05" : "colresize -0.05";
        Hyprland.dispatch("layoutmsg " + arg);
    }
}
