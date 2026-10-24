pragma ComponentBehavior: Bound

import "../components"
import "../services"
import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

FocusScope {
    id: root

    required property bool showing
    property string expandedDevice: ""
    readonly property var adapter: BluetoothService.adapter
    readonly property var filteredDevices: BluetoothService.devices.filter(device =>
        device.adapter === root.adapter?.path &&
        (device.name + " " + device.address).toLowerCase().includes(search.text.toLowerCase()))

    function updateRows() {
        const sorted = filteredDevices.slice().sort((a, b) => groupOrder(a) - groupOrder(b) || a.name.localeCompare(b.name));
        for (let index = 0; index < sorted.length; index++) {
            const device = sorted[index];
            let previous = index;
            while (previous < rows.count && rows.get(previous).path !== device.path)
                previous++;
            if (previous === rows.count)
                rows.insert(index, { path: device.path, device, group: groupName(device) });
            else {
                if (previous !== index)
                    rows.move(previous, index, 1);
                rows.set(index, { path: device.path, device, group: groupName(device) });
            }
        }
        if (rows.count > sorted.length)
            rows.remove(sorted.length, rows.count - sorted.length);
    }

    function groupOrder(device) {
        return device.connected ? 0 : device.bonded ? 1 : 2;
    }

    function groupName(device) {
        return ["已连接", "已保存", "附近设备"][groupOrder(device)];
    }

    onFilteredDevicesChanged: updateRows()
    onShowingChanged: {
        if (!showing && BluetoothService.scanning && BluetoothService.ready)
            BluetoothService.request("scan", "", false);
    }
    Component.onCompleted: updateRows()

    Connections {
        target: BluetoothService
        function onPromptChanged() {
            if (BluetoothService.prompt) {
                root.expandedDevice = BluetoothService.prompt.device;
                PanelState.openControlCenter("bluetooth");
            }
        }
    }

    ListModel {
        id: rows
        dynamicRoles: true
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceM

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: "蓝牙"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
            }
            ActionButton {
                visible: BluetoothService.powered
                text: "󰑓"
                Accessible.name: BluetoothService.scanning ? "停止扫描" : "扫描"
                emphasized: BluetoothService.scanning
                enabled: BluetoothService.ready && !BluetoothService.busy
                onClicked: BluetoothService.request("scan", "", !BluetoothService.scanning)
            }
            ToggleSwitch {
                checked: BluetoothService.powered
                enabled: BluetoothService.ready && !!root.adapter && !BluetoothService.busy
                activeFocusOnTab: true
                Accessible.role: Accessible.CheckBox
                Accessible.name: "蓝牙"
                Accessible.checked: checked
                Accessible.onPressAction: toggled()
                Keys.onSpacePressed: toggled()
                opacity: enabled ? 1 : 0.4
                onToggled: BluetoothService.request("power", "", !BluetoothService.powered)
            }
        }

        ComboBox {
            id: adapterSelector
            visible: BluetoothService.adapters.length > 1
            Layout.fillWidth: true
            model: BluetoothService.adapters
            textRole: "name"
            enabled: !BluetoothService.busy && !BluetoothService.scanning
            Accessible.name: "蓝牙适配器"
            onActivated: index => BluetoothService.selectedAdapter = BluetoothService.adapters[index].path
            HoverHandler {
                cursorShape: adapterSelector.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
            }
            delegate: ItemDelegate {
                required property var modelData
                width: adapterSelector.width
                text: modelData.name
                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }
            }
        }

        Text {
            visible: BluetoothService.serviceError !== "" || !!BluetoothService.errors[""]
            Layout.fillWidth: true
            text: BluetoothService.serviceError || BluetoothService.errors[""] || ""
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.WrapAnywhere
        }

        PairingPrompt {
            Layout.fillWidth: true
            visible: !!BluetoothService.prompt
            request: BluetoothService.prompt
        }

        TextField {
            id: search
            visible: BluetoothService.powered
            Layout.fillWidth: true
            implicitHeight: 36
            placeholderText: "搜索名称或地址"
            color: Colors.text
            placeholderTextColor: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            selectByMouse: true
            Accessible.name: "搜索蓝牙设备"
            background: Rectangle {
                radius: Tokens.radiusS
                color: Colors.surface1
                border.width: search.activeFocus ? 1 : 0
                border.color: Colors.blue
            }
        }

        Text {
            visible: !BluetoothService.powered || rows.count === 0
            Layout.fillWidth: true
            text: !BluetoothService.ready ? "蓝牙服务未就绪"
                : !root.adapter ? "未发现蓝牙适配器"
                : !BluetoothService.powered ? "蓝牙已关闭"
                : search.text ? "没有匹配的设备"
                : root.adapter.discovering ? "正在扫描，请让设备进入配对模式" : "点击扫描以查找附近设备"
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            wrapMode: Text.Wrap
        }

        ListView {
            id: deviceList
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: BluetoothService.powered
            model: rows
            spacing: Tokens.spaceS
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            add: Transition {
                NumberAnimation {
                    property: "opacity"
                    from: 0
                    to: 1
                    duration: Tokens.animFast
                }
            }
            move: Transition {
                NumberAnimation {
                    property: "y"
                    duration: 220
                    easing.type: Easing.OutCubic
                }
            }
            displaced: Transition {
                NumberAnimation {
                    property: "y"
                    duration: 220
                    easing.type: Easing.OutCubic
                }
            }
            ScrollBar.vertical: ScrollBar {
                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }
            }
            section.property: "group"
            section.criteria: ViewSection.FullString
            section.delegate: Item {
                required property string section
                width: deviceList.width
                height: 12
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width
                    height: 1
                    color: Colors.overlay(0.08)
                }
            }
            delegate: BluetoothDeviceCard {
                required property var model
                width: deviceList.width
                device: model.device
                expanded: root.expandedDevice === device.path
                onToggleDetails: root.expandedDevice = expanded ? "" : device.path
            }
        }

        Item {
            visible: !BluetoothService.powered
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }
}
