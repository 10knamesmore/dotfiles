import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell.Io

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

    Process {
        id: proc
    }

    Column {
        anchors.centerIn: parent
        spacing: 2

        Text {
            text: root.icon
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.iconLarge
            anchors.horizontalCenter: parent.horizontalCenter
        }

        Text {
            text: root.label
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
            anchors.horizontalCenter: parent.horizontalCenter
        }

    }

    MouseArea {
        id: pwrHover

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            PanelState.settingsOpen = false;
            proc.command = ["sh", "-c", root.command];
            proc.running = true;
        }
    }

    Behavior on color {
        ColorAnimation {
            duration: 150
        }

    }

}
