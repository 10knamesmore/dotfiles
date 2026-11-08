pragma ComponentBehavior: Bound

import "../components"
import "../services"
import "../theme"
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root

    required property var drive
    required property string expandedVolume
    readonly property bool busy: StorageService.busyForDrive(drive)
    signal toggleVolume(string path)

    implicitHeight: content.implicitHeight + Tokens.spaceM * 2
    radius: Tokens.radiusM
    color: Colors.withAlpha(Colors.surface1, Tokens.cardAlpha)
    border.width: 1
    border.color: Colors.overlay(0.05)

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Tokens.spaceM
        spacing: Tokens.spaceM

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: root.drive.name
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            Text {
                text: StorageService.formatSize(root.drive.size)
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            ActionButton {
                text: root.busy ? "处理中…" : "安全移除"
                enabled: StorageService.ready && !root.busy
                onClicked: StorageService.request("remove", root.drive.path)
            }
        }

        Text {
            Layout.fillWidth: true
            visible: text !== ""
            text: StorageService.errors[root.drive.path] || ""
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.WrapAnywhere
        }

        Text {
            Layout.fillWidth: true
            visible: root.drive.volumes.length === 0
            text: "没有可用分区"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        Repeater {
            model: root.drive.volumes
            delegate: StorageVolume {
                required property var modelData
                Layout.fillWidth: true
                volume: modelData
                busy: root.busy
                expanded: root.expandedVolume === volume.path
                onToggleDetails: root.toggleVolume(volume.path)
            }
        }
    }
}
