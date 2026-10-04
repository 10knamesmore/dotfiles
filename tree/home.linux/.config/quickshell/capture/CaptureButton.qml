import "../theme"
import QtQuick
import QtQuick.Controls

// 捕获浮层的文字按钮；保留 Button 的键盘与无障碍状态，不用图形热区替代控件。
Button {
    id: root
    property bool accent: false
    property string hint: ""
    implicitHeight: 34
    implicitWidth: Math.max(54, label.implicitWidth + 22)
    padding: 8
    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    Accessible.name: text
    ToolTip.visible: hovered && hint !== ""
    ToolTip.text: hint
    background: Rectangle {
        radius: 7
        color: root.down ? Colors.surface2 : root.checked || root.accent ? Colors.blue : root.hovered ? Colors.surface1 : Colors.surface0
        opacity: root.enabled ? 1 : 0.4
        border.width: root.activeFocus ? 2 : 0
        border.color: Colors.lavender
    }
    contentItem: Text {
        id: label
        text: root.text
        color: root.checked || root.accent ? Colors.crust : Colors.text
        opacity: root.enabled ? 1 : 0.5
        font.pixelSize: 13
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        Accessible.ignored: true
    }
}
