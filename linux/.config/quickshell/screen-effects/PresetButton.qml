import "../theme"
import QtQuick
import QtQuick.Layouts

// 预设按钮
Rectangle {
    id: root

    property alias text: label.text

    signal clicked()

    Layout.fillWidth: true
    height: 28
    radius: Tokens.radiusS
    color: mouseArea.containsMouse ? Colors.surface1 : Colors.surface0
    border.color: Colors.surface2
    border.width: 1

    Behavior on color {
        ColorAnimation {
            duration: Tokens.animFast
        }
    }

    Text {
        id: label

        anchors.centerIn: parent
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        font.bold: true
        color: mouseArea.containsMouse ? Colors.text : Colors.subtext0

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animFast
            }
        }
    }

    MouseArea {
        id: mouseArea

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }

}
