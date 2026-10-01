import "../../theme"
import "../../state"
import "../components"
import QtQuick
import QtQuick.Layouts

// 剪贴板模块 — bar 图标，点击打开剪贴板历史面板
BarModule {
    id: root

    accentColor: Colors.flamingo
    implicitWidth: icon.implicitWidth + 32
    onClicked: mouse => {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleClipboard());
    }

    Row {
        id: label

        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Text {
            id: icon
            text: "󰅍"
            color: Colors.flamingo
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: "剪贴板"
            opacity: root.expansion
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            anchors.verticalCenter: parent.verticalCenter
        }

    }

}
