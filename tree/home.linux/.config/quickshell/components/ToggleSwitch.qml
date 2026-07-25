import "../theme"
import QtQuick

Rectangle {
    id: root

    property bool checked: false
    property bool small: false

    signal toggled()

    width: small ? 30 : 40
    height: small ? 16 : 20
    radius: height / 2
    color: checked
        ? Colors.withAlpha(Colors.green, 0.3)
        : Colors.overlay(0.08)

    Rectangle {
        width: root.small ? 12 : 16
        height: width
        radius: width / 2
        anchors.verticalCenter: parent.verticalCenter
        x: root.checked ? parent.width - width - 2 : 2
        color: root.checked ? Colors.green : Colors.overlay1
        Behavior on x { NumberAnimation { duration: Tokens.animFast } }
        Behavior on color { ColorAnimation { duration: Tokens.animFast } }
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggled()
    }

    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
}
