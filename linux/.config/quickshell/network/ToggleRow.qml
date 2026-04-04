import QtQuick
import QtQuick.Layouts
import "../theme"

// 可复用的开关行（图标 + 标签 + toggle 开关）
Rectangle {
    id: root

    property string icon: ""
    property string label: ""
    property bool toggled: false
    signal clicked()

    Layout.fillWidth: true
    height: 40; radius: 10
    color: hoverArea.containsMouse ? Colors.surface1 : "transparent"
    Behavior on color { ColorAnimation { duration: 150 } }

    RowLayout {
        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
        Text { text: root.icon; color: Colors.overlay1; font.family: "Hack Nerd Font"; font.pixelSize: 14 }
        Text { text: root.label; color: Colors.text; Layout.fillWidth: true; font.family: "Hack Nerd Font"; font.pixelSize: 12 }
        Rectangle {
            width: 40; height: 22; radius: 11
            color: root.toggled ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.3) : Colors.surface2
            Behavior on color { ColorAnimation { duration: 200 } }
            Rectangle {
                width: 16; height: 16; radius: 8; y: 3
                x: root.toggled ? parent.width - 19 : 3
                color: root.toggled ? Colors.green : Colors.overlay1
                Behavior on x { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }
                Behavior on color { ColorAnimation { duration: 200 } }
            }
        }
    }
    MouseArea {
        id: hoverArea; anchors.fill: parent
        hoverEnabled: true; cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
