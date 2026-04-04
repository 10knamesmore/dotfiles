import QtQuick
import QtQuick.Layouts
import "../components"
import "../../theme"

// 剪贴板模块 — bar 图标，点击打开剪贴板历史面板
BarModule {
    id: root

    accentColor: Colors.flamingo
    implicitWidth: label.implicitWidth + 32

    Row {
        id: label
        anchors.centerIn: parent
        spacing: 5

        Text {
            text: "󰅍"
            color: Colors.flamingo
            font.family: "Hack Nerd Font"
            font.pixelSize: 15
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    onClicked: {
        PanelState.closeAll()
        PanelState.toggleClipboard()
    }
}
