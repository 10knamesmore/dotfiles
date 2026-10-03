pragma ComponentBehavior: Bound

import "../../theme"
import "../../state"
import "../../services"
import "../components"
import QtQuick
import Quickshell.Bluetooth

// 四个功能入口与最右侧控制首页 handle 共用胶囊；只高亮自身，不移动、缩放或弹出提示。
BarModule {
    id: root

    clickable: false
    hovered: false
    implicitWidth: entries.implicitWidth + 32

    Row {
        id: entries
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        EntryButton {
            page: "network"
            label: "网络"
            icon: NetworkService.statusIcon
            iconColor: NetworkService.disconnected ? Colors.overlay1 : Colors.blue
        }

        EntryButton {
            page: "bluetooth"
            label: "蓝牙"
            icon: Bluetooth.defaultAdapter?.enabled ? "󰂯" : "󰂲"
            iconColor: Bluetooth.defaultAdapter?.enabled ? Colors.blue : Colors.overlay1
        }

        EntryButton {
            page: "notifications"
            label: "通知"
            icon: SystemState.notificationCount > 0 ? "󰂚" : "󰂜"
            count: SystemState.notificationCount
            iconColor: count > 0 ? Colors.yellow : Colors.subtext0
        }

        EntryButton {
            page: "clipboard"
            label: "剪贴板"
            icon: "󰅍"
        }

        Rectangle {
            width: 1
            height: 14
            anchors.verticalCenter: parent.verticalCenter
            color: Colors.overlay(0.12)
        }

        EntryButton {
            page: "home"
            label: "控制中心"
            icon: "󰒓"
        }
    }

    Text {
        anchors.left: entries.right
        anchors.leftMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        text: "控制中心"
        opacity: root.expansion
        color: Colors.text
        font.family: Fonts.family
        font.pixelSize: Fonts.body
    }

    component EntryButton: Rectangle {
        id: entry

        required property string page
        required property string label
        required property string icon
        property color iconColor: Colors.subtext0
        property int count: 0
        readonly property bool selected: PanelState.controlCenterOpen && PanelState.controlCenterPage === page

        width: labelRow.implicitWidth + 14
        height: 28
        radius: Tokens.radiusM
        color: hover.containsMouse ? Colors.withAlpha(Colors.blue, 0.22) : selected ? Colors.withAlpha(Colors.blue, 0.12) : "transparent"
        activeFocusOnTab: true
        Accessible.role: Accessible.Button
        Accessible.name: label
        Accessible.description: count > 0 ? count + " 条通知" : ""
        Accessible.onPressAction: PanelState.openControlCenter(page, root)
        Keys.onReturnPressed: PanelState.openControlCenter(page, root)
        Keys.onSpacePressed: PanelState.openControlCenter(page, root)

        Row {
            id: labelRow
            anchors.centerIn: parent
            spacing: 3

            Text {
                text: entry.icon
                color: hover.containsMouse || entry.selected ? Colors.text : entry.iconColor
                font.family: Fonts.family
                font.pixelSize: Fonts.icon
                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }
            }

            Text {
                visible: entry.count > 0
                text: entry.count
                color: entry.iconColor
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        MouseArea {
            id: hover
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: PanelState.openControlCenter(entry.page, root)
        }

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animFast
            }
        }
    }
}
