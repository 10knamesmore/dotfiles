import "../theme"
import QtQuick
import QtQuick.Controls

// 音频与指针开关不依赖系统 Qt 主题，选中状态同时呈现勾选标记与颜色。
CheckBox {
    id: root
    implicitHeight: 32
    implicitWidth: label.implicitWidth + 28
    Accessible.name: text
    indicator: Rectangle {
        width: 18
        height: 18
        y: (root.height - height) / 2
        radius: 4
        color: root.checked ? Colors.blue : Colors.surface0
        border.color: root.activeFocus ? Colors.lavender : Colors.overlay0
        Text {
            anchors.centerIn: parent
            text: root.checked ? "✓" : ""
            color: Colors.crust
            font.pixelSize: 14
            Accessible.ignored: true
        }
    }
    contentItem: Text {
        id: label
        text: root.text
        leftPadding: 26
        color: Colors.text
        font.pixelSize: 13
        verticalAlignment: Text.AlignVCenter
        Accessible.ignored: true
    }
}
