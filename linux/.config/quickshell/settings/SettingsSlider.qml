import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 可复用的设置滑块组件
ColumnLayout {
    id: root

    property string icon: ""
    property string label: ""
    property real value: 0
    property color accentColor: Colors.blue

    signal moved(real val)

    spacing: 4

    RowLayout {
        spacing: 8

        Text {
            text: root.icon
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.heading
        }

        Text {
            text: root.label
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

    }

    Slider {
        id: slider

        Layout.fillWidth: true
        from: 0
        to: 1
        value: root.value
        onMoved: root.moved(value)

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.NoButton
        }

        background: Rectangle {
            implicitWidth: 200
            implicitHeight: 8
            radius: 4
            color: Colors.surface1

            Rectangle {
                width: parent.width * slider.visualPosition
                height: parent.height
                radius: parent.radius
                color: root.accentColor
            }

        }

        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 22
            implicitHeight: 22
            radius: 11
            color: slider.pressed ? Colors.text : slider.hovered ? Colors.subtext1 : Colors.subtext0
            border.color: root.accentColor
            border.width: 2
            scale: slider.pressed ? 1.1 : slider.hovered ? 1.05 : 1

            Behavior on scale {
                NumberAnimation {
                    duration: 100
                }

            }

            Behavior on color {
                ColorAnimation {
                    duration: 150
                }

            }

        }

    }

}
