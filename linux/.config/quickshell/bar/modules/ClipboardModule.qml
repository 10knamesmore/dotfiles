import "../../theme"
import "../components"
import QtQuick
import QtQuick.Layouts

// 剪贴板模块 — bar 图标，点击打开剪贴板历史面板
BarModule {
    id: root

    accentColor: Colors.flamingo
    implicitWidth: label.implicitWidth + 32
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        PanelState.morphSourceX = pos.x + 2;
        PanelState.morphSourceY = pos.y + 6;
        PanelState.toggleClipboard();
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: "󰅍"
            color: Colors.flamingo
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            anchors.verticalCenter: parent.verticalCenter
        }

    }

}
