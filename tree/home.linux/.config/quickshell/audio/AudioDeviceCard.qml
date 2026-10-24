import "../services"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    required property var node
    readonly property bool selected: node === (node.isSink ? AudioService.sink : AudioService.source)
    readonly property string format: AudioService.deviceFormat(node)

    implicitHeight: contents.implicitHeight + Tokens.spaceM * 2
    radius: Tokens.radiusM
    color: selected ? Colors.withAlpha(Colors.peach, 0.07) : Colors.withAlpha(Colors.surface1, Tokens.cardAlpha)
    border.width: 1
    border.color: selected ? Colors.withAlpha(Colors.peach, 0.25) : Colors.overlay(0.05)

    ColumnLayout {
        id: contents
        anchors.fill: parent
        anchors.margins: Tokens.spaceM
        spacing: Tokens.spaceS

        Button {
            id: selectButton
            Layout.fillWidth: true
            padding: 0
            implicitHeight: Math.max(30, contentItem.implicitHeight)
            enabled: root.node.ready
            Accessible.name: root.node.description
            Accessible.role: Accessible.RadioButton
            Accessible.checkable: true
            Accessible.checked: root.selected
            onClicked: AudioService.selectDevice(root.node)
            HoverHandler {
                cursorShape: selectButton.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
            }
            background: Rectangle {
                radius: Tokens.radiusS
                color: selectButton.hovered || selectButton.activeFocus ? Colors.overlay(0.04) : "transparent"
            }
            contentItem: RowLayout {
                spacing: Tokens.spaceS
                Text {
                    text: root.node.properties["api.bluez5.address"] ? "󰂯" : root.node.isSink ? "󰕾" : "󰍬"
                    color: Colors.peach
                    font.family: Fonts.family
                    font.pixelSize: Fonts.heading
                }
                Text {
                    Layout.fillWidth: true
                    text: root.node.description
                    wrapMode: Text.Wrap
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                }
                Text {
                    text: root.selected ? "●" : "○"
                    color: root.selected ? Colors.peach : Colors.overlay1
                    font.pixelSize: Fonts.body
                }
            }
        }

        Text {
            visible: root.format !== ""
            text: root.format
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        DeviceVolume {
            Layout.fillWidth: true
            node: root.node
        }
    }
}
