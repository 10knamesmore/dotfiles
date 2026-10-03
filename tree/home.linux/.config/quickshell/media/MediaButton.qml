import "../theme"
import QtQuick
import QtQuick.Controls

// 媒体面板内的图标按钮；label 同时用于提示和辅助功能名称。
Button {
    id: root

    required property string label
    property bool primary: false
    property bool selected: false

    implicitWidth: primary ? 40 : 32
    implicitHeight: implicitWidth
    padding: 0
    hoverEnabled: true
    Accessible.name: label
    ToolTip.visible: hovered
    ToolTip.text: label
    ToolTip.delay: 600
    opacity: enabled ? 1 : 0.3
    scale: down ? 0.92 : primary && hovered ? 1.04 : 1

    background: Rectangle {
        radius: root.primary ? height / 2 : Tokens.radiusS
        color: root.primary ? Colors.mauve : root.hovered || root.visualFocus ? Colors.overlay(0.07) : "transparent"
        border.width: root.visualFocus ? 1 : 0
        border.color: Colors.mauve

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animFast
            }

        }

    }

    contentItem: Text {
        text: root.text
        textFormat: Text.PlainText
        font.family: Fonts.family
        font.pixelSize: root.primary ? Fonts.h3 : Fonts.heading
        color: root.primary ? Colors.base : root.selected || root.checked ? Colors.mauve : Colors.subtext0
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        Accessible.ignored: true
    }

    Behavior on scale {
        NumberAnimation {
            duration: Tokens.animFast
            easing.type: Easing.OutCubic
        }

    }

}
