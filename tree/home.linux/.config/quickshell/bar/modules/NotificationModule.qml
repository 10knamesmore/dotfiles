import "../../theme"
import "../../state"
import "../components"
import QtQuick
import QtQuick.Layouts

// 通知模块 — 铃铛图标 + 未读计数
BarModule {
    id: root

    accentColor: SystemState.notificationCount > 0 ? Colors.yellow : Colors.overlay0
    implicitWidth: icon.implicitWidth + (count.visible ? count.implicitWidth + 5 : 0) + 32
    onClicked: mouse => {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleNotification());
    }
    onRightClicked: {
        SystemState.clearAllNotifications();
    }

    Row {
        id: label

        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5

        Text {
            id: icon
            text: SystemState.notificationCount > 0 ? "󰂚" : "󰂜"
            color: SystemState.notificationCount > 0 ? Colors.yellow : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: count
            visible: SystemState.notificationCount > 0
            text: SystemState.notificationCount
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: "通知"
            opacity: root.expansion
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            anchors.verticalCenter: parent.verticalCenter
        }
    }

}
