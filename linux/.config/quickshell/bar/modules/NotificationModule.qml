import QtQuick
import QtQuick.Layouts
import "../components"
import "../../theme"

// 通知模块 — 铃铛图标 + 未读计数
BarModule {
    id: root

    accentColor: PanelState.notificationCount > 0 ? Colors.yellow : Colors.overlay0
    implicitWidth: label.implicitWidth + 32

    Row {
        id: label
        anchors.centerIn: parent
        spacing: 5

        Text {
            text: PanelState.notificationCount > 0 ? "󰂚" : "󰂜"
            color: PanelState.notificationCount > 0 ? Colors.yellow : Colors.overlay1
            font.family: "Hack Nerd Font"
            font.pixelSize: 15
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            visible: PanelState.notificationCount > 0
            text: PanelState.notificationCount
            color: Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 12
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    onClicked: {
        PanelState.closeAll()
        PanelState.toggleNotification()
    }
    onRightClicked: {
        PanelState.clearAllNotifications()
    }
}
