import "../../theme"
import "../../state"
import "../../services"
import "../components"
import QtQuick
import Quickshell.Bluetooth

// 网络、蓝牙与通知状态合用一个胶囊；展开时将这个实时头部交给控制中心。
BarModule {
    id: root

    readonly property bool bluetoothEnabled: Bluetooth.defaultAdapter?.enabled ?? false
    accentColor: Colors.blue
    implicitWidth: statusRow.implicitWidth + 32
    onClicked: PanelState.toggleControlCenter(root)

    Accessible.role: Accessible.Button
    Accessible.name: "控制中心"
    Accessible.description: (NetworkService.disconnected ? "网络未连接" : "网络已连接") + "，蓝牙" + (bluetoothEnabled ? "开启" : "关闭") + "，" + SystemState.notificationCount + " 条通知"
    Accessible.onPressAction: PanelState.toggleControlCenter(root)

    Row {
        id: statusRow
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10

        Text {
            text: NetworkService.disconnected ? "󰤮" : NetworkService.connectionType === "wifi" ? "󰤨" : "󰈀"
            color: NetworkService.disconnected ? Colors.overlay1 : Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }

        Text {
            text: root.bluetoothEnabled ? "󰂯" : "󰂲"
            color: root.bluetoothEnabled ? Colors.blue : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }

        Text {
            visible: SystemState.notificationCount > 0
            text: "󰂚 " + SystemState.notificationCount
            color: Colors.yellow
            font.family: Fonts.family
            font.pixelSize: Fonts.body
        }

        Text {
            text: "󰒓"
            color: ScreenEffectsState.effectsActive ? Colors.mauve : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }
    }

    Text {
        anchors.left: statusRow.right
        anchors.leftMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        text: "控制中心"
        opacity: root.expansion
        color: Colors.text
        font.family: Fonts.family
        font.pixelSize: Fonts.body
    }
}
