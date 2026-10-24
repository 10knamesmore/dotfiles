import "../components"
import "../services"
import "../theme"
import "DeviceInfo.js" as DeviceInfo
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    required property var device
    required property bool expanded
    signal toggleDetails()
    property real detailsProgress: expanded ? 1 : 0
    readonly property var operation: BluetoothService.progressForDevice(device.path)
    readonly property string error: BluetoothService.errors[device.path] ?? ""

    implicitHeight: contents.implicitHeight + Tokens.spaceM * 2
    radius: Tokens.radiusM
    color: device.connected ? Colors.withAlpha(Colors.blue, headerHover.hovered ? 0.12 : 0.07)
        : Colors.withAlpha(Colors.surface1, headerHover.hovered ? 0.7 : Tokens.cardAlpha)
    border.width: 1
    border.color: headerInteraction.activeFocus ? Colors.blue : device.connected ? Colors.withAlpha(Colors.blue, 0.25) : Colors.overlay(0.05)

    Item {
        id: headerInteraction
        width: root.width
        height: deviceHeader.implicitHeight + Tokens.spaceM * 2
        activeFocusOnTab: true
        Accessible.role: Accessible.Button
        Accessible.name: root.device.name + (root.expanded ? "，收起详情" : "，展开详情")
        Accessible.checkable: true
        Accessible.checked: root.expanded
        Accessible.onPressAction: root.toggleDetails()
        Keys.onReturnPressed: root.toggleDetails()
        Keys.onSpacePressed: root.toggleDetails()

        HoverHandler {
            id: headerHover
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                headerInteraction.forceActiveFocus();
                root.toggleDetails();
            }
        }
    }

    Behavior on detailsProgress {
        NumberAnimation {
            duration: 220
            easing.type: Easing.OutCubic
        }
    }
    Behavior on color {
        ColorAnimation { duration: Tokens.animFast }
    }
    Behavior on border.color {
        ColorAnimation { duration: Tokens.animFast }
    }

    ColumnLayout {
        id: contents
        x: Tokens.spaceM
        y: Tokens.spaceM
        width: root.width - Tokens.spaceM * 2
        spacing: 0

        RowLayout {
            id: deviceHeader
            Layout.fillWidth: true
            spacing: Tokens.spaceS

            Text {
                text: DeviceInfo.icon(root.device)
                color: root.device.connected ? Colors.blue : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.heading
                HoverHandler { id: deviceIconHover }
                ToolTip.visible: deviceIconHover.hovered && DeviceInfo.serviceNames(root.device).length > 0
                ToolTip.text: DeviceInfo.serviceNames(root.device).join("\n")
                ToolTip.delay: 600
                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.device.name
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                HoverHandler { id: nameHover }
                ToolTip.visible: nameHover.hovered && truncated
                ToolTip.text: text
                ToolTip.delay: 600
            }

            Text {
                visible: root.device.battery !== null
                text: root.device.battery + "%"
                color: Colors.green
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            ActionButton {
                visible: !root.operation
                text: root.device.connected ? "断开" : "连接"
                emphasized: !root.device.connected
                enabled: BluetoothService.ready && !BluetoothService.busy
                onClicked: BluetoothService.request(root.device.connected ? "disconnect" : "connect", root.device.path)
            }
            ActionButton {
                visible: !!root.operation
                text: "取消"
                enabled: root.operation?.action === "connect"
                onClicked: BluetoothService.request("cancel", root.device.path)
            }
            Text {
                text: "⌄"
                rotation: root.detailsProgress * 180
                color: Colors.overlay1
                font.pixelSize: Fonts.body
            }
        }

        Text {
            visible: !!root.operation
            Layout.fillWidth: true
            Layout.topMargin: Tokens.spaceS
            text: DeviceInfo.operationText(root.operation)
            color: Colors.yellow
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        Text {
            visible: root.error !== ""
            Layout.fillWidth: true
            Layout.topMargin: Tokens.spaceS
            text: root.error
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.WrapAnywhere
        }

        Item {
            Layout.fillWidth: true
            implicitHeight: root.detailsProgress * (management.implicitHeight + Tokens.spaceM)
            enabled: root.expanded
            clip: true

            BluetoothDeviceDetails {
                id: management
                width: parent.width
                y: Tokens.spaceM - (1 - root.detailsProgress) * 6
                opacity: root.detailsProgress
                device: root.device
                active: root.expanded
            }
        }
    }
}
