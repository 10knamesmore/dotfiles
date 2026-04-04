import "../../theme"
import "../components"
import QtQuick
import Quickshell
import Quickshell.Io

BarModule {
    id: root

    property string iconText: "󰤮"
    property string valueText: "…"
    property string tooltipText: ""
    property bool disconnected: false

    accentColor: Colors.sky
    implicitWidth: label.implicitWidth + 32
    Component.onCompleted: reader.running = true
    onClicked: {
        PanelState.closeAll();
        PanelState.toggleNetwork();
    }

    Process {
        id: reader

        command: [Quickshell.env("scripts_dir") + "/network_status.sh"]

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
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.valueText
            color: root.disconnected ? Colors.red : Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: 300
                }

            }

        }

    }

}
