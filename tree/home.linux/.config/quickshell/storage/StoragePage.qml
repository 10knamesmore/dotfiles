pragma ComponentBehavior: Bound

import "../services"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

FocusScope {
    id: root

    required property bool showing
    property string expandedVolume: ""

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceM

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: "可移动存储"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
            }
            Text {
                text: StorageService.drives.length + " 台 · " + StorageService.mountedCount + " 已挂载"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spaceM
            Text {
                text: "自动挂载"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            StorageToggle {
                checked: StorageService.settings.autoMount
                enabled: StorageService.ready && !StorageService.configuring
                Accessible.name: "默认自动挂载新插入的设备"
                onToggled: StorageService.configure(!checked, StorageService.settings.notifications)
            }
            Item {
                Layout.fillWidth: true
            }
            Text {
                text: "插拔提示"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            StorageToggle {
                checked: StorageService.settings.notifications
                enabled: StorageService.ready && !StorageService.configuring
                Accessible.name: "插拔提示"
                onToggled: StorageService.configure(StorageService.settings.autoMount, !checked)
            }
        }

        Text {
            Layout.fillWidth: true
            visible: text !== ""
            text: StorageService.serviceError || StorageService.errors[""] || ""
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.WrapAnywhere
        }

        Text {
            Layout.fillWidth: true
            visible: StorageService.drives.length === 0 && !StorageService.serviceError
            text: StorageService.ready ? "插入 U 盘或 SD 卡" : "正在连接存储服务…"
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.body
        }

        ScrollView {
            id: scroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true

            ColumnLayout {
                width: scroll.availableWidth
                spacing: Tokens.spaceM

                Repeater {
                    model: StorageService.drives
                    delegate: StorageDriveCard {
                        required property var modelData
                        Layout.fillWidth: true
                        drive: modelData
                        expandedVolume: root.expandedVolume
                        onToggleVolume: path => root.expandedVolume = root.expandedVolume === path ? "" : path
                    }
                }
            }
        }
    }
}
