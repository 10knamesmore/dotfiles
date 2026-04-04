import QtQuick
import QtQuick.Layouts
import "../theme"

// 可复用的多选一选项行（如 DHCP/手动、自动/5GHz/2.4GHz）
RowLayout {
    id: root

    property var model: []
    property string current: ""
    property bool compact: false
    signal selected(string value)

    spacing: 6

    Repeater {
        model: root.model
        delegate: Rectangle {
            required property var modelData
            Layout.fillWidth: true
            height: root.compact ? 28 : 32
            radius: root.compact ? 6 : 8
            color: root.current === modelData.value
                ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)
                : (optHover.containsMouse ? Colors.surface1 : Colors.surface0)
            border.color: root.current === modelData.value
                ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.3) : "transparent"
            border.width: 1
            Behavior on color { ColorAnimation { duration: 150 } }

            Text {
                anchors.centerIn: parent; text: modelData.label
                color: root.current === modelData.value ? Colors.blue : Colors.text
                font.family: "Hack Nerd Font"; font.pixelSize: root.compact ? 11 : 12
                Behavior on color { ColorAnimation { duration: 150 } }
            }
            MouseArea {
                id: optHover; anchors.fill: parent
                hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                onClicked: root.selected(modelData.value)
            }
        }
    }
}
