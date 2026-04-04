import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "../theme"

// 工具按钮 — 截图/录屏/显示器切换等
Rectangle {
    id: root

    property string icon: ""
    property string label: ""
    property string command: ""
    property bool closeOnClick: true

    Layout.fillWidth: true
    implicitHeight: 48
    radius: 10
    color: toolHover.containsMouse ? Colors.surface1 : Colors.surface0
    Behavior on color { ColorAnimation { duration: 150 } }

    Process { id: proc }

    Column {
        anchors.centerIn: parent
        spacing: 2
        Text {
            text: root.icon
            color: Colors.overlay1
            font.family: "Hack Nerd Font"
            font.pixelSize: 16
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
        id: toolHover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            if (root.closeOnClick)
                PanelState.settingsOpen = false;
            proc.command = ["sh", "-c", root.command];
            proc.running = true;
        }
    }
}
