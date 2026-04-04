import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 单个效果滑块：标签 + 滑条 + 数值
RowLayout {
    id: root

    property string label: ""
    property int value: 0

    signal moved(int val)

    Layout.fillWidth: true
    spacing: 8

    Text {
        text: root.label
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        color: Colors.subtext1
        Layout.preferredWidth: 90
    }

    Slider {
        id: slider

        Layout.fillWidth: true
        from: 0
        to: 100
        stepSize: 5
        value: root.value
        live: true
        onMoved: root.moved(Math.round(value))

        // 和外部属性同步（避免绑定环路）
        Connections {
            function onValueChanged() {
                if (!slider.pressed)
                    slider.value = root.value;

            }

            target: root
        }

        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 120
            implicitHeight: 4
            width: slider.availableWidth
            height: 4
            radius: 2
            color: Colors.surface2

            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: 2
                color: Colors.mauve
            }

        }

        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 18
            implicitHeight: 18
            width: 18
            height: 18
            radius: 9
            color: slider.pressed ? Colors.pink : Colors.text
            border.color: Colors.mauve
            border.width: 2
        }

    }

    Text {
        text: Math.round(slider.value)
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        color: Colors.subtext0
        Layout.preferredWidth: 28
        horizontalAlignment: Text.AlignRight
    }

}
