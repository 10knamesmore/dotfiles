import "../../theme"
import "../../state"
import "../components"
import QtQuick
import QtQuick.Layouts

// 通知模块 — 铃铛图标 + 未读计数
BarModule {
    id: root

    accentColor: SystemState.notificationCount > 0 ? Colors.yellow : Colors.overlay0
    implicitWidth: label.implicitWidth + 32
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        MorphState.morphSourceX = pos.x + 2;
        MorphState.morphSourceY = pos.y + 6;
        PanelState.toggleNotification();
    }
    onRightClicked: {
        SystemState.clearAllNotifications();
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: SystemState.notificationCount > 0 ? "󰂚" : "󰂜"
            color: SystemState.notificationCount > 0 ? Colors.yellow : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            visible: SystemState.notificationCount > 0
            text: SystemState.notificationCount
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

    }

}
