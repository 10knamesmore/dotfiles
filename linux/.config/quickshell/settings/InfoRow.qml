import QtQuick
import QtQuick.Layouts
import "../theme"

// 通用信息行 — icon + label + optional value
RowLayout {
    property string icon: ""
    property string label: ""
    property string value: ""
    property color valueColor: Colors.text

    Layout.fillWidth: true
    spacing: 8

    Text {
        text: icon
        color: Colors.overlay1
        font.family: "Hack Nerd Font"
        font.pixelSize: 13
        Layout.preferredWidth: 20
        horizontalAlignment: Text.AlignHCenter
    }
    Text {
        text: label
        color: Colors.subtext0
        font.family: "Hack Nerd Font"
        font.pixelSize: 11
        elide: Text.ElideMiddle
        Layout.fillWidth: true
    }
    Text {
        visible: value.length > 0
        text: value
        color: valueColor
        font.family: "Hack Nerd Font"
        font.pixelSize: 11
        font.weight: Font.DemiBold
    }
}
