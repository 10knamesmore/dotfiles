import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 蓝牙面板 — 右侧弹出，扫描/连接/断开/配对/忘记
PanelOverlay {
    id: root

    // ── 状态 ──
    property bool btPowered: false
    property bool scanning: false
    property string searchQuery: ""
    property string errorMsg: ""
    property string actionDevice: "" // 正在操作的设备 MAC
    property var _deviceBuf: []
    property var _infoQueue: []
    property int _infoIdx: 0

    function refreshDevices() {
        if (!root.btPowered)
            return;
        devicesProc.running = true;
    }

    function startScan() {
        if (!root.btPowered || root.scanning)
            return;
        root.scanning = true;
        scanProc.command = ["bluetoothctl", "--timeout", "10", "scan", "on"];
        scanProc.running = true;
    }

    function stopScan() {
        scanProc.running = false;
        scanStopProc.running = true;
        root.scanning = false;
    }

    function togglePower() {
        powerProc.command = ["bluetoothctl", "power", root.btPowered ? "off" : "on"];
        powerProc.running = true;
        root.btPowered = !root.btPowered;
        if (!root.btPowered) {
            deviceModel.clear();
            filteredModel.clear();
            root.scanning = false;
        }
    }

    function connectDevice(mac) {
        root.actionDevice = mac;
        root.errorMsg = "";
        connectProc.command = ["bluetoothctl", "connect", mac];
        connectProc.running = true;
    }

    function disconnectDevice(mac) {
        root.actionDevice = mac;
        disconnectProc.command = ["bluetoothctl", "disconnect", mac];
        disconnectProc.running = true;
    }

    function pairDevice(mac) {
        root.actionDevice = mac;
        root.errorMsg = "";
        pairProc.command = ["bluetoothctl", "pair", mac];
        pairProc.running = true;
    }

    function trustDevice(mac) {
        trustProc.command = ["bluetoothctl", "trust", mac];
        trustProc.running = true;
    }

    function removeDevice(mac) {
        root.actionDevice = mac;
        removeProc.command = ["bluetoothctl", "remove", mac];
        removeProc.running = true;
    }

    function applyFilter() {
        filteredModel.clear();
        let q = searchQuery.toLowerCase();
        for (let i = 0; i < deviceModel.count; i++) {
            let item = deviceModel.get(i);
            if (q.length === 0 || item.name.toLowerCase().includes(q))
                filteredModel.append(item);
        }
    }

    function deviceIcon(name) {
        let n = name.toLowerCase();
        if (n.includes("airpods") || n.includes("headphone") || n.includes("headset") || n.includes("buds") || n.includes("ear"))
            return "󰋋";
        if (n.includes("keyboard") || n.includes("keychron"))
            return "󰌌";
        if (n.includes("mouse") || n.includes("trackpad") || n.includes("mx master"))
            return "󰍽";
        if (n.includes("phone") || n.includes("iphone") || n.includes("pixel") || n.includes("galaxy") || n.includes("xiaomi"))
            return "󰏲";
        if (n.includes("speaker") || n.includes("soundbar"))
            return "󰓃";
        if (n.includes("watch"))
            return "󰥔";
        if (n.includes("gamepad") || n.includes("controller") || n.includes("joystick"))
            return "󰊗";
        return "󰂱";
    }

    // 逐个获取设备详情
    function fetchDeviceInfo() {
        if (root._infoIdx >= root._infoQueue.length) {
            // 全部完成，排序并更新 model
            let sorted = root._deviceBuf.slice();
            sorted.sort((a, b) => {
                if (a.connected !== b.connected)
                    return a.connected ? -1 : 1;
                if (a.paired !== b.paired)
                    return a.paired ? -1 : 1;
                return a.name.localeCompare(b.name);
            });
            deviceModel.clear();
            for (let item of sorted)
                deviceModel.append(item);
            applyFilter();
            return;
        }
        let dev = root._infoQueue[root._infoIdx];
        infoProc._currentMac = dev.mac;
        infoProc._currentName = dev.name;
        infoProc._connected = false;
        infoProc._paired = false;
        infoProc._trusted = false;
        infoProc._battery = -1;
        infoProc.command = ["bluetoothctl", "info", dev.mac];
        infoProc.running = true;
    }

    showing: PanelState.bluetoothOpen
    panelWidth: 400
    panelHeight: root.height * 0.6
    panelTargetX: root.width - 410
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.bluetoothOpen = false
    onShowingChanged: {
        if (showing) {
            searchQuery = "";
            errorMsg = "";
            actionDevice = "";
            adapterProc.running = true;
        } else {
            if (scanning)
                stopScan();
        }
    }

    ListModel { id: deviceModel }
    ListModel { id: filteredModel }

    Timer {
        id: focusTimer
        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

    // 面板打开时定时刷新设备列表
    Timer {
        running: root.showing && root.btPowered
        interval: 5000
        repeat: true
        onTriggered: refreshDevices()
    }

    // ── 适配器状态 ──
    Process {
        id: adapterProc
        command: ["bluetoothctl", "show"]
        stdout: SplitParser {
            onRead: (data) => {
                if (data.includes("Powered:"))
                    root.btPowered = data.includes("yes");
            }
        }
        onExited: {
            if (root.btPowered) {
                refreshDevices();
                startScan();
                focusTimer.start();
            }
        }
    }

    // MAC 格式检测 — 名称是 MAC 地址的设备视为未解析
    function _isMacName(name) {
        return /^[0-9A-Fa-f]{2}[-:][0-9A-Fa-f]{2}[-:][0-9A-Fa-f]{2}[-:][0-9A-Fa-f]{2}[-:][0-9A-Fa-f]{2}[-:][0-9A-Fa-f]{2}$/.test(name);
    }

    // ── 设备列表（两阶段：先已配对，再全部）──
    Process {
        id: devicesProc
        command: ["bluetoothctl", "devices", "Paired"]
        onStarted: {
            root._deviceBuf = [];
            root._infoQueue = [];
            root._infoIdx = 0;
        }
        stdout: SplitParser {
            onRead: (data) => {
                let m = data.match(/^Device\s+([0-9A-Fa-f:]{17})\s+(.+)$/);
                if (m)
                    root._infoQueue.push({ "mac": m[1], "name": m[2].trim() });
            }
        }
        onExited: {
            // 已配对设备收集完毕，再收集扫描发现的有名字设备
            discoveredProc.running = true;
        }
    }

    Process {
        id: discoveredProc
        command: ["bluetoothctl", "devices"]
        property var _pairedMacs: ({})
        onStarted: {
            // 记录已在队列中的 MAC，避免重复
            _pairedMacs = {};
            for (let d of root._infoQueue)
                _pairedMacs[d.mac] = true;
        }
        stdout: SplitParser {
            onRead: (data) => {
                let m = data.match(/^Device\s+([0-9A-Fa-f:]{17})\s+(.+)$/);
                if (m) {
                    let mac = m[1], name = m[2].trim();
                    // 跳过已在列表中的、以及名称是 MAC 的未解析设备
                    if (discoveredProc._pairedMacs[mac] || root._isMacName(name))
                        return;
                    root._infoQueue.push({ "mac": mac, "name": name });
                }
            }
        }
        onExited: {
            root._infoIdx = 0;
            root.fetchDeviceInfo();
        }
    }

    // ── 设备详情（串行查询）──
    Process {
        id: infoProc
        property string _currentMac: ""
        property string _currentName: ""
        property bool _connected: false
        property bool _paired: false
        property bool _trusted: false
        property int _battery: -1

        stdout: SplitParser {
            onRead: (data) => {
                let d = data.trim();
                if (d.startsWith("Connected:"))
                    infoProc._connected = d.includes("yes");
                else if (d.startsWith("Paired:"))
                    infoProc._paired = d.includes("yes");
                else if (d.startsWith("Trusted:"))
                    infoProc._trusted = d.includes("yes");
                else if (d.startsWith("Battery Percentage:")) {
                    let bm = d.match(/\((\d+)\)/);
                    if (bm)
                        infoProc._battery = parseInt(bm[1]);
                }
            }
        }
        onExited: {
            root._deviceBuf.push({
                "mac": infoProc._currentMac,
                "name": infoProc._currentName,
                "connected": infoProc._connected,
                "paired": infoProc._paired,
                "trusted": infoProc._trusted,
                "battery": infoProc._battery
            });
            root._infoIdx++;
            root.fetchDeviceInfo();
        }
    }

    // ── 操作进程 ──
    Process { id: powerProc }

    Process {
        id: scanProc
        onExited: root.scanning = false
    }

    Process { id: scanStopProc; command: ["bluetoothctl", "scan", "off"] }

    Process {
        id: connectProc
        property string _errBuf: ""
        onStarted: _errBuf = ""
        onExited: (code, status) => {
            root.actionDevice = "";
            if (code === 0) {
                root.errorMsg = "";
                refreshDevices();
            } else {
                root.errorMsg = connectProc._errBuf || "连接失败";
            }
        }
        stderr: SplitParser {
            onRead: (data) => { connectProc._errBuf += data; }
        }
    }

    Process {
        id: disconnectProc
        onExited: {
            root.actionDevice = "";
            refreshDevices();
        }
    }

    Process {
        id: pairProc
        property string _errBuf: ""
        onStarted: _errBuf = ""
        onExited: (code, status) => {
            root.actionDevice = "";
            if (code === 0) {
                root.errorMsg = "";
                refreshDevices();
            } else {
                root.errorMsg = pairProc._errBuf || "配对失败";
            }
        }
        stderr: SplitParser {
            onRead: (data) => { pairProc._errBuf += data; }
        }
    }

    Process {
        id: trustProc
        onExited: refreshDevices()
    }

    Process {
        id: removeProc
        onExited: (code, status) => {
            root.actionDevice = "";
            if (code === 0) {
                root.errorMsg = "";
                refreshDevices();
            } else {
                root.errorMsg = "删除失败";
            }
        }
    }

    // ── UI ──
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // ── 标题栏 ──
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: root.btPowered ? "󰂯" : "󰂲"
                color: root.btPowered ? Colors.blue : Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.title
            }

            Text {
                text: "蓝牙"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item { Layout.fillWidth: true }

            // 扫描按钮
            Rectangle {
                width: scanBtnText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                visible: root.btPowered
                color: root.scanning
                    ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)
                    : (scanBtnArea.containsMouse ? Colors.surface2 : "transparent")

                Text {
                    id: scanBtnText
                    anchors.centerIn: parent
                    text: root.scanning ? "扫描中..." : "扫描"
                    color: root.scanning ? Colors.blue : (scanBtnArea.containsMouse ? Colors.blue : Colors.subtext0)
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small

                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                MouseArea {
                    id: scanBtnArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (root.scanning)
                            root.stopScan();
                        else
                            root.startScan();
                    }
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }

            // 刷新
            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                visible: root.btPowered
                color: refreshArea.containsMouse ? Colors.surface2 : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰑓"
                    color: refreshArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                MouseArea {
                    id: refreshArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.refreshDevices()
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }

            // 电源开关
            Rectangle {
                width: powerToggleText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: root.btPowered
                    ? (powerToggleArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25) : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15))
                    : (powerToggleArea.containsMouse ? Colors.surface2 : Colors.surface1)

                Text {
                    id: powerToggleText
                    anchors.centerIn: parent
                    text: root.btPowered ? "开启" : "关闭"
                    color: root.btPowered ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                MouseArea {
                    id: powerToggleArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.togglePower()
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }
        }

        // ── 搜索框 ──
        Rectangle {
            Layout.fillWidth: true
            height: 36
            radius: Tokens.radiusMS
            color: Colors.surface1
            visible: root.btPowered

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: Tokens.spaceS

                Text {
                    text: ""
                    color: Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon
                }

                TextInput {
                    id: searchInput
                    Layout.fillWidth: true
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.bodyLarge
                    clip: true
                    selectByMouse: true
                    onTextChanged: {
                        root.searchQuery = text;
                        root.applyFilter();
                    }
                    Keys.onEscapePressed: PanelState.bluetoothOpen = false

                    Text {
                        anchors.fill: parent
                        text: "搜索设备..."
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 蓝牙关闭提示 ──
        ColumnLayout {
            visible: !root.btPowered
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Tokens.spaceS

            Item { Layout.fillHeight: true }

            Text {
                text: "󰂲"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.display2
                Layout.alignment: Qt.AlignHCenter
            }

            Text {
                text: "蓝牙已关闭"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.bodyLarge
                Layout.alignment: Qt.AlignHCenter
            }

            Item { Layout.fillHeight: true }
        }

        // ── 空状态 ──
        Text {
            visible: root.btPowered && filteredModel.count === 0
            text: deviceModel.count === 0 ? (root.scanning ? "正在扫描..." : "未发现设备") : "未找到匹配设备"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 20
            Layout.bottomMargin: 20
        }

        // ── 设备列表 ──
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: filteredModel
            spacing: Tokens.spaceXS
            clip: true
            visible: root.btPowered

            delegate: Rectangle {
                id: devDelegate

                required property int index
                required property string mac
                required property string name
                required property bool connected
                required property bool paired
                required property bool trusted
                required property int battery

                width: ListView.view.width
                height: devRow.implicitHeight + 12
                radius: Tokens.radiusMS
                color: connected
                    ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, devHover.containsMouse ? 0.2 : 0.1)
                    : (devHover.containsMouse ? Colors.surface1 : "transparent")
                border.color: connected
                    ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, Tokens.borderHoverAlpha)
                    : "transparent"
                border.width: connected ? 1 : 0

                MouseArea {
                    id: devHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: (mouse) => {
                        if (mouse.button === Qt.RightButton) {
                            // 右键菜单区域
                            if (paired)
                                actionsRow.visible = !actionsRow.visible;
                        } else {
                            if (connected) {
                                root.disconnectDevice(mac);
                            } else if (paired) {
                                root.connectDevice(mac);
                            } else {
                                // 未配对设备：先配对再连接
                                root.pairDevice(mac);
                            }
                        }
                    }
                }

                ColumnLayout {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    spacing: 4

                    RowLayout {
                        id: devRow
                        spacing: Tokens.spaceS
                        Layout.fillWidth: true

                        Text {
                            text: root.deviceIcon(name)
                            color: connected ? Colors.blue : Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.heading

                            Behavior on color {
                                ColorAnimation { duration: Tokens.animFast; easing.type: Easing.BezierSpline; easing.bezierCurve: Anim.standard }
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1

                            Text {
                                text: name
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            RowLayout {
                                spacing: 6

                                Text {
                                    visible: connected
                                    text: "已连接"
                                    color: Colors.green
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }

                                Text {
                                    visible: paired && !connected
                                    text: "已配对"
                                    color: Colors.subtext0
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }

                                Text {
                                    visible: !paired && !connected
                                    text: "可用"
                                    color: Colors.overlay0
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }

                                Text {
                                    visible: battery >= 0
                                    text: "󰁹 " + battery + "%"
                                    color: battery > 20 ? Colors.green : Colors.red
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }

                                Text {
                                    visible: root.actionDevice === mac
                                    text: "..."
                                    color: Colors.blue
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }
                            }
                        }

                        Text {
                            text: mac
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                            visible: devHover.containsMouse
                            opacity: devHover.containsMouse ? 1 : 0
                            Behavior on opacity { NumberAnimation { duration: Tokens.animNormal } }
                        }
                    }

                    // ── 操作按钮行（右键展开）──
                    RowLayout {
                        id: actionsRow
                        visible: false
                        Layout.fillWidth: true
                        spacing: Tokens.spaceS

                        Rectangle {
                            visible: !connected && paired
                            width: actConnText.implicitWidth + 16
                            height: 26
                            radius: Tokens.radiusS
                            color: actConnArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.2) : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.1)

                            Text { id: actConnText; anchors.centerIn: parent; text: "连接"; color: Colors.blue; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                            MouseArea { id: actConnArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.connectDevice(mac) }
                            Behavior on color { ColorAnimation { duration: 150 } }
                        }

                        Rectangle {
                            visible: connected
                            width: actDiscText.implicitWidth + 16
                            height: 26
                            radius: Tokens.radiusS
                            color: actDiscArea.containsMouse ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.2) : Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.1)

                            Text { id: actDiscText; anchors.centerIn: parent; text: "断开"; color: Colors.yellow; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                            MouseArea { id: actDiscArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.disconnectDevice(mac) }
                            Behavior on color { ColorAnimation { duration: 150 } }
                        }

                        Rectangle {
                            visible: paired && !trusted
                            width: actTrustText.implicitWidth + 16
                            height: 26
                            radius: Tokens.radiusS
                            color: actTrustArea.containsMouse ? Qt.rgba(Colors.teal.r, Colors.teal.g, Colors.teal.b, 0.2) : Qt.rgba(Colors.teal.r, Colors.teal.g, Colors.teal.b, 0.1)

                            Text { id: actTrustText; anchors.centerIn: parent; text: "信任"; color: Colors.teal; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                            MouseArea { id: actTrustArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.trustDevice(mac) }
                            Behavior on color { ColorAnimation { duration: 150 } }
                        }

                        Rectangle {
                            visible: paired
                            width: actRemoveText.implicitWidth + 16
                            height: 26
                            radius: Tokens.radiusS
                            color: actRemoveArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.2) : Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.1)

                            Text { id: actRemoveText; anchors.centerIn: parent; text: "忘记"; color: Colors.red; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                            MouseArea { id: actRemoveArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.removeDevice(mac) }
                            Behavior on color { ColorAnimation { duration: 150 } }
                        }

                        Item { Layout.fillWidth: true }
                    }
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }
        }

        // ── 错误提示 ──
        Text {
            visible: root.errorMsg !== ""
            text: "⚠ " + root.errorMsg
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }
}
