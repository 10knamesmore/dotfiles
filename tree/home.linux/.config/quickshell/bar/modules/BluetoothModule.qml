import "../../theme"
import "../../state"
import "../components"
import QtQuick
import Quickshell.Bluetooth

// 蓝牙入口复用现有设备面板；适配器状态由 BlueZ 的属性变更驱动。
BarModule {
    id: root

    readonly property bool powered: Bluetooth.defaultAdapter?.enabled ?? false

    accentColor: powered ? Colors.blue : Colors.overlay0
    implicitWidth: label.implicitWidth + 32
    onClicked: {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleBluetooth());
    }

    Row {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Text {
            id: label
            text: root.powered ? "󰂯" : "󰂲"
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
        }

        Text {
            text: "蓝牙"
            opacity: root.expansion
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
