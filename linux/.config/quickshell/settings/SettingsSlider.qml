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

            // 渐变填充（accent 浅→深）
            Rectangle {
                width: parent.width * slider.visualPosition
                height: parent.height
                radius: parent.radius

                gradient: Gradient {
                    orientation: Gradient.Horizontal

                    GradientStop {
                        position: 0
                        color: Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, 0.6)
                    }

                    GradientStop {
                        position: 1
                        color: root.accentColor
                    }

                }

                Behavior on width {
                    NumberAnimation {
                        duration: 80
                    }

                }

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
            scale: slider.pressed ? 1.15 : slider.hovered ? 1.08 : 1

            // Hover 外圈发光
            Rectangle {
                anchors.centerIn: parent
                width: parent.width + 8
                height: parent.height + 8
                radius: width / 2
                color: "transparent"
                border.color: Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, slider.hovered ? 0.2 : 0)
                border.width: 2

                Behavior on border.color {
                    ColorAnimation {
                        duration: 200
                    }

                }

            }

            Behavior on scale {
                NumberAnimation {
                    duration: 150
                    easing.type: Easing.OutBack
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
