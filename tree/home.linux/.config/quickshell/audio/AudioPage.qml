pragma ComponentBehavior: Bound

import "../components"
import "../services"
import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property bool showing
    property bool inputsSelected: false
    readonly property var devices: inputsSelected ? AudioService.inputs : AudioService.outputs
    spacing: Tokens.spaceM

    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: Tokens.spaceL
        Layout.leftMargin: Tokens.spaceL
        Layout.rightMargin: Tokens.spaceL
        spacing: Tokens.spaceS

        ActionButton {
            text: "󰕾  输出"
            emphasized: !root.inputsSelected
            accent: Colors.peach
            onClicked: root.inputsSelected = false
        }
        ActionButton {
            text: "󰍬  输入"
            emphasized: root.inputsSelected
            accent: Colors.peach
            onClicked: root.inputsSelected = true
        }
        Item { Layout.fillWidth: true }
        ActionButton {
            text: "󰂯"
            Accessible.name: "蓝牙设备"
            onClicked: PanelState.openControlCenter("bluetooth")
        }
    }

    Flickable {
        id: viewport
        Layout.fillWidth: true
        Layout.fillHeight: true
        contentHeight: contents.implicitHeight + Tokens.spaceL
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {
            HoverHandler {
                cursorShape: Qt.PointingHandCursor
            }
        }

        ColumnLayout {
            id: contents
            x: Tokens.spaceL
            width: viewport.width - Tokens.spaceL * 2
            spacing: Tokens.spaceS

            Repeater {
                model: root.devices
                AudioDeviceCard {
                    required property var modelData
                    Layout.fillWidth: true
                    node: modelData
                }
            }
            Text {
                visible: !AudioService.ready || root.devices.length === 0
                Layout.fillWidth: true
                Layout.topMargin: Tokens.spaceL
                text: !AudioService.ready ? "PipeWire 未连接"
                    : root.inputsSelected ? "没有可用的音频输入" : "没有可用的音频输出"
                color: AudioService.ready ? Colors.overlay1 : Colors.red
                font.family: Fonts.family
                font.pixelSize: Fonts.body
            }
        }
    }
}
