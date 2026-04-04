import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

// 单个效果滑块：标签 + 滑条 + 数值
RowLayout {
    id: root
    Layout.fillWidth: true
    spacing: 8

    property string label: ""
    property int value: 0
    signal moved(int val)

    Text {
        text: root.label
        font.pixelSize: 13
        color: "#b8c0e0"
        Layout.preferredWidth: 90
    }

    Slider {
        id: slider
        Layout.fillWidth: true
        from: 0; to: 100; stepSize: 5
        value: root.value
        live: true

        onMoved: root.moved(Math.round(value))

        // 和外部属性同步（避免绑定环路）
        Connections {
            target: root
            function onValueChanged() {
                if (!slider.pressed) {
                    slider.value = root.value
                }
            }
        }

        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 120
            implicitHeight: 4
            width: slider.availableWidth
            height: 4
            radius: 2
            color: "#5b6078"

            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: 2
                color: "#c6a0f6"
            }
        }

        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 18
            implicitHeight: 18
            width: 18; height: 18
            radius: 9
            color: slider.pressed ? "#f5bde6" : "#cad3f5"
            border.color: "#c6a0f6"
            border.width: 2
        }
    }

    Text {
        text: Math.round(slider.value)
        font.pixelSize: 12
        color: "#a5adcb"
        Layout.preferredWidth: 28
        horizontalAlignment: Text.AlignRight
    }
}
