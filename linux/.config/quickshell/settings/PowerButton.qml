import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import "../theme"

// 电源操作按钮
Rectangle {
    id: root

    property string icon: ""
    property string label: ""
    property string command: ""

    Layout.fillWidth: true
    implicitHeight: 48
    radius: 10
    color: pwrHover.containsMouse ? Colors.surface1 : Colors.surface0
    Behavior on color { ColorAnimation { duration: 150 } }

    Process { id: proc }

    Column {
        anchors.centerIn: parent
        spacing: 2
        Text {
            text: root.icon
            color: Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 18
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Text {
            text: root.label
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 9
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }

    MouseArea {
        id: pwrHover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            PanelState.settingsOpen = false
            proc.command = ["sh", "-c", root.command]
            proc.running = true
        }
    }
}
