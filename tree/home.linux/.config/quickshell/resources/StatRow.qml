import "../theme"
import QtQuick

// 指标名与数值分列，供资源面板组合容量、速率和地址。
Item {
    id: root

    required property string label
    required property string value
    property color valueColor: Colors.text

    Accessible.name: label + "，" + value
    Accessible.role: Accessible.StaticText
    implicitHeight: Math.max(labelText.implicitHeight, valueText.implicitHeight)

    Text {
        id: labelText

        Accessible.ignored: true
        anchors.left: parent.left
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        text: root.label
    }
    Text {
        id: valueText

        Accessible.ignored: true
        anchors.right: parent.right
        color: root.valueColor
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        text: root.value
    }
}
