import "../../theme"
import "../components"
import QtQuick
import Quickshell
import Quickshell.Hyprland._Ipc
import Quickshell.Io

// 滚动布局列状态（复用 waybar 的 scroll_status.sh，传递 WAYBAR_OUTPUT_NAME）
BarModule {
    id: root

    property var barScreen: null
    property string displayText: "…"
    property string tooltipText: ""

    accentColor: Colors.yellow
    implicitWidth: Math.max(label.implicitWidth + 32, 80)
    Component.onCompleted: reader.running = true
    onScrolled: (delta) => {
        let arg = delta > 0 ? "colresize +0.05" : "colresize -0.05";
        Hyprland.dispatch("layoutmsg " + arg);
    }

    Process {
        id: reader

        command: [Quickshell.env("HOME") + "/.config/waybar/scripts/scroll_status.sh"]
        environment: ({
            "WAYBAR_OUTPUT_NAME": root.barScreen ? root.barScreen.name : ""
        })

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
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        font.weight: Font.DemiBold
    }

}
