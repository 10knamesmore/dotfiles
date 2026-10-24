import "../components"
import "../services"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property var device
    required property bool active
    property bool confirmingForget: false
    readonly property var audioOutput: AudioService.bluetoothOutput(device.address)
    spacing: Tokens.spaceS
    onActiveChanged: {
        if (active)
            confirmingForget = false;
    }

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: 1
        color: Colors.surface2
    }

    RowLayout {
        Layout.fillWidth: true
        Text {
            Layout.fillWidth: true
            text: root.device.address
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            HoverHandler { id: addressHover }
            ToolTip.visible: addressHover.hovered
            ToolTip.text: root.device.bonded ? "已保存配对" : "未保存绑定"
            ToolTip.delay: 600
        }
        Text {
            visible: root.device.rssi !== null
            text: root.device.rssi + " dBm"
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }
    }

    Text {
        visible: !!root.audioOutput
        text: root.audioOutput ? AudioService.deviceFormat(root.audioOutput) : ""
        color: Colors.peach
        font.family: Fonts.family
        font.pixelSize: Fonts.small
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: Tokens.spaceS
        Layout.minimumHeight: 30
        Text {
            Layout.fillWidth: true
            text: "连接免确认"
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            MouseArea {
                anchors.fill: parent
                enabled: authorizationSwitch.enabled
                cursorShape: Qt.PointingHandCursor
                onClicked: authorizationSwitch.toggled()
            }
        }
        ToggleSwitch {
            id: authorizationSwitch
            checked: root.device.trusted
            enabled: BluetoothService.ready && !BluetoothService.busy
            activeFocusOnTab: true
            opacity: enabled ? 1 : 0.4
            border.width: activeFocus ? 1 : 0
            border.color: Colors.text
            Accessible.role: Accessible.CheckBox
            Accessible.name: "连接免确认"
            Accessible.checkable: true
            Accessible.checked: checked
            Accessible.onPressAction: toggled()
            Keys.onSpacePressed: toggled()
            Keys.onReturnPressed: toggled()
            onToggled: BluetoothService.request("trust", root.device.path, !root.device.trusted)
        }
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: Tokens.spaceS
        Item {
            visible: !root.confirmingForget
            Layout.fillWidth: true
        }
        Text {
            visible: root.confirmingForget
            Layout.fillWidth: true
            text: "删除配对记录并断开？"
            wrapMode: Text.Wrap
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }
        ActionButton {
            visible: root.confirmingForget
            text: "取消"
            onClicked: root.confirmingForget = false
        }
        ActionButton {
            text: root.confirmingForget ? "忘记" : "忘记设备"
            emphasized: root.confirmingForget
            accent: Colors.red
            enabled: BluetoothService.ready && !BluetoothService.busy
            onClicked: {
                if (root.confirmingForget)
                    BluetoothService.request("forget", root.device.path);
                else
                    root.confirmingForget = true;
            }
        }
    }
}
