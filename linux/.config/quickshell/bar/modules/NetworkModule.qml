import QtQuick
import Quickshell
import Quickshell.Io
import "../components"
import "../../theme"

BarModule {
    id: root
    accentColor: Colors.sky
    implicitWidth: label.implicitWidth + 32

    property string iconText: "󰤮"
    property string valueText: "…"
    property string tooltipText: ""
    property bool disconnected: false

    Process {
        id: reader
        command: [Quickshell.env("scripts_dir") + "/network_status.sh"]
        stdout: SplitParser {
            onRead: data => {
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

    Component.onCompleted: reader.running = true

    Timer {
        interval: 5000
        running: true
        repeat: true
        onTriggered: {
            reader.running = false;
            reader.running = true;
        }
    }

    Row {
        id: label
        anchors.centerIn: parent
        spacing: 5

        Text {
            text: root.iconText
            color: root.disconnected ? Colors.red : Colors.sky
            font.family: "Hack Nerd Font"
            font.pixelSize: 14
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.valueText
            color: root.disconnected ? Colors.red : Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 13
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
            Behavior on color {
                ColorAnimation {
                    duration: 300
                }
            }
        }
    }

    onClicked: Quickshell.execDetached(["kitty", "nmtui"])
}
