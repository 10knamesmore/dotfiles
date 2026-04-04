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
    radius: 8
    color: mouseArea.containsMouse ? "#494d64" : "#363a4f"
    border.color: "#5b6078"
    border.width: 1

    Text {
        id: label

        anchors.centerIn: parent
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        font.bold: true
        color: mouseArea.containsMouse ? "#cad3f5" : "#a5adcb"
    }

    MouseArea {
        id: mouseArea

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }

}
