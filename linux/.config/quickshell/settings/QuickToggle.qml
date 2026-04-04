import QtQuick
import QtQuick.Layouts
import "../theme"

// 可复用的快捷开关组件
Rectangle {
    id: root

    property string icon: ""
    property string label: ""
    property string status: ""
    property bool toggled: false
    signal clicked()
    signal rightClicked()

    Layout.fillWidth: true
    implicitHeight: toggleCol.implicitHeight + 16
    radius: 12
    color: toggled
        ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, toggleHover.containsMouse ? 0.25 : 0.15)
        : toggleHover.containsMouse ? Colors.surface1 : Colors.surface0
    border.color: toggled
        ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, toggleHover.containsMouse ? 0.5 : 0.3)
        : toggleHover.containsMouse ? Qt.rgba(1, 1, 1, 0.06) : "transparent"
    border.width: 1

    Behavior on color { ColorAnimation { duration: 200 } }

    ColumnLayout {
        id: toggleCol
        anchors.fill: parent
        anchors.margins: 8
        spacing: 2

        Text {
            text: root.icon
            color: root.toggled ? Colors.blue : Colors.overlay1
            font.family: "Hack Nerd Font"
            font.pixelSize: 18
        }
        Text {
            text: root.label
            color: Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            font.weight: Font.DemiBold
        }
        Text {
            text: root.status
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 9
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }

    MouseArea {
        id: toggleHover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked()
            else
                root.clicked()
        }
    }
}
