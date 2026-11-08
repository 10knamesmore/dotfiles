pragma ComponentBehavior: Bound

import "../components"
import "../services"
import "../theme"
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property var volume
    required property bool busy
    required property bool expanded
    readonly property bool mounted: volume.mountPoints.length > 0
    readonly property var preference: StorageService.preference(volume.key)
    signal toggleDetails

    spacing: Tokens.spaceS

    Divider {
        Layout.fillWidth: true
    }

    RowLayout {
        Layout.fillWidth: true
        Text {
            Layout.fillWidth: true
            text: root.volume.label || root.volume.device
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            elide: Text.ElideRight
        }
        Text {
            text: root.volume.readOnly ? "写保护" : root.mounted ? "已挂载" : "未挂载"
            color: root.volume.readOnly ? Colors.peach : root.mounted ? Colors.green : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }
    }

    Text {
        Layout.fillWidth: true
        text: [root.volume.device, root.volume.filesystem || "未知格式", StorageService.formatSize(root.volume.size)].join(" · ")
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.xs
        wrapMode: Text.Wrap
    }

    Text {
        Layout.fillWidth: true
        visible: root.mounted
        text: root.volume.mountPoints.join("\n")
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.xs
        wrapMode: Text.WrapAnywhere
    }

    RowLayout {
        Layout.fillWidth: true
        ActionButton {
            visible: root.volume.mountable
            text: root.mounted ? "卸载" : (root.volume.readOnly || root.preference.readOnly ? "只读挂载" : "挂载")
            emphasized: !root.mounted
            enabled: StorageService.ready && !root.busy
            onClicked: StorageService.request(root.mounted ? "unmount" : "mount", root.volume.path)
        }
        ActionButton {
            visible: root.mounted
            text: "打开"
            enabled: StorageService.ready && !root.busy
            onClicked: StorageService.openVolume(root.volume)
        }
        Text {
            visible: !root.volume.mountable
            text: "不可挂载"
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }
        Item {
            Layout.fillWidth: true
        }
        ActionButton {
            text: root.expanded ? "收起" : "配置"
            emphasized: root.expanded
            onClicked: root.toggleDetails()
        }
    }

    Text {
        Layout.fillWidth: true
        visible: text !== ""
        text: StorageService.errors[root.volume.path] || StorageService.errors[root.volume.key] || ""
        color: Colors.red
        font.family: Fonts.family
        font.pixelSize: Fonts.small
        wrapMode: Text.WrapAnywhere
    }

    ColumnLayout {
        Layout.fillWidth: true
        visible: root.expanded
        spacing: Tokens.spaceS

        Text {
            Layout.fillWidth: true
            text: root.volume.uuid ? "UUID  " + root.volume.uuid : "此分区没有 UUID，无法保存偏好"
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
            wrapMode: Text.WrapAnywhere
        }

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: "下次插入"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            Dropdown {
                Layout.preferredWidth: 160
                model: ["跟随默认", "自动挂载", "手动挂载"]
                currentText: root.preference.autoMount === "on" ? "自动挂载" : root.preference.autoMount === "off" ? "手动挂载" : "跟随默认"
                enabled: StorageService.ready && !!root.volume.key && root.volume.mountable && !StorageService.configuring
                onActivated: value => StorageService.savePreference(root.volume.key, value === "自动挂载" ? "on" : value === "手动挂载" ? "off" : "inherit", root.preference.readOnly)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: "下次挂载只读"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            StorageToggle {
                checked: root.preference.readOnly
                enabled: StorageService.ready && !!root.volume.key && root.volume.mountable && !StorageService.configuring
                Accessible.name: "下次挂载只读"
                onToggled: StorageService.savePreference(root.volume.key, root.preference.autoMount, !checked)
            }
        }
    }
}
