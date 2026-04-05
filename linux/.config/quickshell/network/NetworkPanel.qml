import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// WiFi 网络面板 — 右上角弹出，扫描/连接/断开/编辑
PanelOverlay {
    id: root

    // ── 状态 ──
    property string searchQuery: ""
    property bool wifiEnabled: true
    property string selectedSsid: ""
    property bool connecting: false
    property string errorMsg: ""
    property string _wifiIface: "wlan0"
    // 编辑模式
    property string editingSsid: ""
    property bool editAutoConnect: true
    property string editIpMethod: "auto"
    property string editIpAddr: ""
    property string editGateway: ""
    property string editDns: ""
    property string editIp6Method: "auto"
    property string editIp6Dns: ""
    property string editMac: ""
    property string editMtu: "auto"
    property string editBand: ""
    property bool editHidden: false
    property string editMetered: "unknown"
    property string editProxyMethod: "none"
    property string editProxyPacUrl: ""
    property bool editProxyBrowserOnly: false
    property bool editLoading: false
    property string editSaveMsg: ""
    property var savedNetworks: ({
    })
    property var _scanBuf: []
    property var _savedBuf: ({
    })

    // ── 数据获取 ──
    function checkWifiRadio() {
        wifiRadioProc.running = true;
    }

    function scanNetworks() {
        if (!root.wifiEnabled)
            return ;

        scanProc.running = true;
    }

    function loadSaved() {
        savedProc.running = true;
    }

    function applyFilter() {
        filteredModel.clear();
        let q = searchQuery.toLowerCase();
        for (let i = 0; i < networkModel.count; i++) {
            let item = networkModel.get(i);
            if (q.length === 0 || item.ssid.toLowerCase().includes(q))
                filteredModel.append(item);

        }
    }

    function toggleWifi() {
        toggleRadioProc.command = ["nmcli", "radio", "wifi", root.wifiEnabled ? "off" : "on"];
        toggleRadioProc.running = true;
        root.wifiEnabled = !root.wifiEnabled;
        if (!root.wifiEnabled) {
            networkModel.clear();
            filteredModel.clear();
            selectedSsid = "";
            editingSsid = "";
        }
    }

    function connectToNetwork(ssid, password) {
        if (password)
            connectProc.command = ["nmcli", "dev", "wifi", "connect", ssid, "password", password];
        else
            connectProc.command = ["nmcli", "dev", "wifi", "connect", ssid];
        connectProc.running = true;
    }

    function disconnectNetwork() {
        disconnectProc.command = ["nmcli", "device", "disconnect", root._wifiIface];
        disconnectProc.running = true;
    }

    function openEditView(ssid) {
        root.editingSsid = ssid;
        root.editLoading = true;
        root.editSaveMsg = "";
        root.editAutoConnect = true;
        root.editIpMethod = "auto";
        root.editIpAddr = "";
        root.editGateway = "";
        root.editDns = "";
        root.editIp6Method = "auto";
        root.editIp6Dns = "";
        root.editMac = "";
        root.editMtu = "auto";
        root.editBand = "";
        root.editHidden = false;
        root.editMetered = "unknown";
        root.editProxyMethod = "none";
        root.editProxyPacUrl = "";
        root.editProxyBrowserOnly = false;
        editReadProc.command = ["nmcli", "-t", "-f", "connection.autoconnect,connection.metered,ipv4.method,ipv4.addresses,ipv4.gateway,ipv4.dns,ipv6.method,ipv6.dns,802-11-wireless.cloned-mac-address,802-11-wireless.mtu,802-11-wireless.band,802-11-wireless.hidden,proxy.method,proxy.pac-url,proxy.browser-only", "connection", "show", ssid];
        editReadProc.running = true;
    }

    function saveEdits() {
        let args = ["nmcli", "connection", "modify", root.editingSsid, "connection.autoconnect", root.editAutoConnect ? "yes" : "no", "connection.metered", root.editMetered, "ipv4.method", root.editIpMethod];
        if (root.editIpMethod === "manual") {
            if (root.editIpAddr)
                args = args.concat(["ipv4.addresses", root.editIpAddr]);

            if (root.editGateway)
                args = args.concat(["ipv4.gateway", root.editGateway]);

        }
        if (root.editDns)
            args = args.concat(["ipv4.dns", root.editDns]);
        else
            args = args.concat(["ipv4.dns", ""]);
        args = args.concat(["ipv6.method", root.editIp6Method]);
        if (root.editIp6Dns)
            args = args.concat(["ipv6.dns", root.editIp6Dns]);
        else
            args = args.concat(["ipv6.dns", ""]);
        if (root.editMac)
            args = args.concat(["802-11-wireless.cloned-mac-address", root.editMac]);
        else
            args = args.concat(["802-11-wireless.cloned-mac-address", ""]);
        args = args.concat(["802-11-wireless.mtu", root.editMtu || "auto"]);
        args = args.concat(["802-11-wireless.band", root.editBand || ""]);
        args = args.concat(["802-11-wireless.hidden", root.editHidden ? "yes" : "no"]);
        args = args.concat(["proxy.method", root.editProxyMethod]);
        if (root.editProxyMethod === "auto" && root.editProxyPacUrl)
            args = args.concat(["proxy.pac-url", root.editProxyPacUrl]);
        else
            args = args.concat(["proxy.pac-url", ""]);
        args = args.concat(["proxy.browser-only", root.editProxyBrowserOnly ? "yes" : "no"]);
        editSaveProc.command = args;
        editSaveProc.running = true;
    }

    function forgetNetwork() {
        forgetProc.command = ["nmcli", "connection", "delete", root.editingSsid];
        forgetProc.running = true;
    }

    function signalIcon(sig) {
        if (sig >= 75)
            return "󰤨";

        if (sig >= 50)
            return "󰤥";

        if (sig >= 25)
            return "󰤢";

        return "󰤟";
    }

    function signalColor(sig) {
        if (sig >= 60)
            return Colors.green;

        if (sig >= 30)
            return Colors.yellow;

        return Colors.red;
    }

    showing: PanelState.networkOpen
    panelWidth: 400
    panelHeight: root.height * 0.7
    panelTargetX: root.width - 410
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.networkOpen = false
    onShowingChanged: {
        if (showing) {
            searchQuery = "";
            selectedSsid = "";
            editingSsid = "";
            errorMsg = "";
            connecting = false;
            checkWifiRadio();
            loadSaved();
            scanNetworks();
            focusTimer.start();
        }
    }

    ListModel {
        id: networkModel
    }

    ListModel {
        id: filteredModel
    }

    Timer {
        id: focusTimer

        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

    Timer {
        running: root.showing && root.wifiEnabled && root.editingSsid === ""
        interval: 10000
        repeat: true
        onTriggered: scanNetworks()
    }

    // WiFi 开关状态
    Process {
        id: wifiRadioProc

        command: ["nmcli", "radio", "wifi"]
        onExited: {
            if (root.wifiEnabled)
                ifaceProc.running = true;

        }

        stdout: SplitParser {
            onRead: (data) => {
                return root.wifiEnabled = data.trim() === "enabled";
            }
        }

    }

    // 获取无线接口名
    Process {
        id: ifaceProc

        command: ["sh", "-c", "ip route 2>/dev/null | awk '/^default/ {print $5; exit}'"]

        stdout: SplitParser {
            onRead: (data) => {
                let iface = data.trim();
                if (iface)
                    root._wifiIface = iface;

            }
        }

    }

    // 扫描 WiFi 列表
    Process {
        id: scanProc

        command: ["nmcli", "-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list", "--rescan", "auto"]
        onStarted: root._scanBuf = []
        onExited: {
            let map = {
            };
            for (let item of root._scanBuf) {
                if (!map[item.ssid] || item.signal > map[item.ssid].signal || item.inUse) {
                    if (map[item.ssid] && map[item.ssid].inUse)
                        item.inUse = true;

                    item.saved = root.savedNetworks[item.ssid] === true;
                    map[item.ssid] = item;
                }
            }
            let sorted = Object.values(map);
            sorted.sort((a, b) => {
                if (a.inUse !== b.inUse)
                    return a.inUse ? -1 : 1;

                if (a.saved !== b.saved)
                    return a.saved ? -1 : 1;

                return b.signal - a.signal;
            });
            networkModel.clear();
            for (let item of sorted) networkModel.append(item)
            applyFilter();
        }

        stdout: SplitParser {
            onRead: (data) => {
                let raw = data.trim();
                if (!raw)
                    return ;

                let inUse = raw.endsWith(":*");
                if (inUse)
                    raw = raw.slice(0, -2);
                else if (raw.endsWith(":"))
                    raw = raw.slice(0, -1);
                let lastColon = raw.lastIndexOf(":");
                let security = raw.substring(lastColon + 1);
                raw = raw.substring(0, lastColon);
                lastColon = raw.lastIndexOf(":");
                let signal = parseInt(raw.substring(lastColon + 1)) || 0;
                let ssid = raw.substring(0, lastColon).replace(/\\:/g, ":");
                if (!ssid)
                    return ;

                root._scanBuf.push({
                    "ssid": ssid,
                    "signal": signal,
                    "security": security,
                    "inUse": inUse,
                    "saved": root.savedNetworks[ssid] === true
                });
            }
        }

    }

    // 已保存网络列表
    Process {
        id: savedProc

        command: ["nmcli", "-t", "-f", "NAME,TYPE", "connection", "show"]
        onStarted: root._savedBuf = {
        }
        onExited: {
            root.savedNetworks = root._savedBuf;
            scanNetworks();
        }

        stdout: SplitParser {
            onRead: (data) => {
                let parts = data.split(":");
                if (parts.length >= 2 && parts[parts.length - 1] === "802-11-wireless")
                    root._savedBuf[parts.slice(0, -1).join(":")] = true;

            }
        }

    }

    // WiFi 开关
    Process {
        id: toggleRadioProc
    }

    // 连接网络
    Process {
        id: connectProc

        property string _errBuf: ""

        onStarted: {
            _errBuf = "";
            root.connecting = true;
            root.errorMsg = "";
        }
        onExited: (code, status) => {
            root.connecting = false;
            if (code === 0) {
                root.selectedSsid = "";
                root.errorMsg = "";
                passwordInput.text = "";
                scanNetworks();
            } else {
                root.errorMsg = connectProc._errBuf || "连接失败";
            }
        }

        stderr: SplitParser {
            onRead: (data) => {
                return connectProc._errBuf += data;
            }
        }

    }

    // 断开连接
    Process {
        id: disconnectProc

        onExited: scanNetworks()
    }

    // ── 编辑相关进程 ──
    Process {
        id: editReadProc

        onExited: root.editLoading = false

        stdout: SplitParser {
            function val(s) {
                let v = s.substring(s.indexOf(":") + 1).trim();
                return (v === "--" || v === "") ? "" : v;
            }

            onRead: (data) => {
                let d = data.trim();
                if (d.startsWith("connection.autoconnect:"))
                    root.editAutoConnect = d.split(":")[1].trim() === "yes";
                else if (d.startsWith("connection.metered:"))
                    root.editMetered = d.split(":")[1].trim();
                else if (d.startsWith("ipv4.method:"))
                    root.editIpMethod = d.split(":")[1].trim();
                else if (d.startsWith("ipv4.addresses:"))
                    root.editIpAddr = val(d);
                else if (d.startsWith("ipv4.gateway:"))
                    root.editGateway = val(d);
                else if (d.startsWith("ipv4.dns:"))
                    root.editDns = val(d);
                else if (d.startsWith("ipv6.method:"))
                    root.editIp6Method = d.split(":")[1].trim();
                else if (d.startsWith("ipv6.dns:"))
                    root.editIp6Dns = val(d);
                else if (d.startsWith("802-11-wireless.cloned-mac-address:"))
                    root.editMac = val(d);
                else if (d.startsWith("802-11-wireless.mtu:"))
                    root.editMtu = val(d) || "auto";
                else if (d.startsWith("802-11-wireless.band:"))
                    root.editBand = val(d);
                else if (d.startsWith("802-11-wireless.hidden:"))
                    root.editHidden = d.split(":")[1].trim() === "yes";
                else if (d.startsWith("proxy.method:"))
                    root.editProxyMethod = d.split(":")[1].trim();
                else if (d.startsWith("proxy.pac-url:"))
                    root.editProxyPacUrl = val(d);
                else if (d.startsWith("proxy.browser-only:"))
                    root.editProxyBrowserOnly = d.split(":")[1].trim() === "yes";
            }
        }

    }

    Process {
        id: editSaveProc

        property string _errBuf: ""

        onStarted: {
            _errBuf = "";
            root.editSaveMsg = "";
        }
        onExited: (code, status) => {
            if (code === 0) {
                root.editSaveMsg = "已保存";
                // 如果是已连接网络，重新应用
                reapplyProc.command = ["nmcli", "connection", "up", root.editingSsid];
                reapplyProc.running = true;
            } else {
                root.editSaveMsg = editSaveProc._errBuf || "保存失败";
            }
        }

        stderr: SplitParser {
            onRead: (data) => {
                return editSaveProc._errBuf += data;
            }
        }

    }

    Process {
        id: reapplyProc

        onExited: scanNetworks()
    }

    Process {
        id: forgetProc

        onExited: (code, status) => {
            if (code === 0) {
                root.editingSsid = "";
                loadSaved();
            } else {
                root.editSaveMsg = "删除失败";
            }
        }
    }

    // ════════════════════════════════════════
    // 列表视图（editingSsid === "" 时显示）
    // ════════════════════════════════════════
    ColumnLayout {
        id: listView

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS
        visible: root.editingSsid === ""

        // ── 标题栏 ──
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: root.wifiEnabled ? "󰤨" : "󰤭"
                color: root.wifiEnabled ? Colors.blue : Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.title
            }

            Text {
                text: "WiFi"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item {
                Layout.fillWidth: true
            }

            // 刷新
            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: refreshArea.containsMouse ? Colors.surface2 : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰑓"
                    color: refreshArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon

                    Behavior on color {
                        ColorAnimation {
                            duration: 150
                        }

                    }

                }

                MouseArea {
                    id: refreshArea

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.scanNetworks()
                }

                Behavior on color {
                    ColorAnimation {
                        duration: 150
                    }

                }

            }

            // WiFi 开关
            Rectangle {
                width: wifiToggleText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: root.wifiEnabled ? (wifiToggleArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25) : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)) : (wifiToggleArea.containsMouse ? Colors.surface2 : Colors.surface1)

                Text {
                    id: wifiToggleText

                    anchors.centerIn: parent
                    text: root.wifiEnabled ? "开启" : "关闭"
                    color: root.wifiEnabled ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small

                    Behavior on color {
                        ColorAnimation {
                            duration: 150
                        }

                    }

                }

                MouseArea {
                    id: wifiToggleArea

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.toggleWifi()
                }

                Behavior on color {
                    ColorAnimation {
                        duration: 150
                    }

                }

            }

        }

        // ── 搜索框 ──
        Rectangle {
            Layout.fillWidth: true
            height: 36
            radius: Tokens.radiusMS
            color: Colors.surface1
            visible: root.wifiEnabled

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
                    Keys.onEscapePressed: PanelState.networkOpen = false

                    Text {
                        anchors.fill: parent
                        text: "搜索网络..."
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

        // ── WiFi 关闭提示 ──
        ColumnLayout {
            visible: !root.wifiEnabled
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Tokens.spaceS

            Item {
                Layout.fillHeight: true
            }

            Text {
                text: "󰤭"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.display2
                Layout.alignment: Qt.AlignHCenter
            }

            Text {
                text: "WiFi 已关闭"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.bodyLarge
                Layout.alignment: Qt.AlignHCenter
            }

            Item {
                Layout.fillHeight: true
            }

        }

        // ── 空状态 ──
        Text {
            visible: root.wifiEnabled && filteredModel.count === 0
            text: networkModel.count === 0 ? "正在扫描..." : "未找到匹配网络"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 20
            Layout.bottomMargin: 20
        }

        // ── 网络列表 ──
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: filteredModel
            spacing: Tokens.spaceXS
            clip: true
            visible: root.wifiEnabled

            delegate: Rectangle {
                id: netDelegate

                required property int index
                required property string ssid
                required property int signal
                required property string security
                required property bool inUse
                required property bool saved

                width: ListView.view.width
                height: netRow.implicitHeight + 12
                radius: Tokens.radiusMS
                color: inUse ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, netHover.containsMouse ? 0.2 : 0.1) : (netHover.containsMouse ? Colors.surface1 : "transparent")
                border.color: inUse ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, Tokens.borderHoverAlpha) : "transparent"
                border.width: inUse ? 1 : 0

                MouseArea {
                    id: netHover

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: (mouse) => {
                        if (mouse.button === Qt.RightButton) {
                            // 右键：仅已保存网络可编辑
                            if (saved || inUse)
                                root.openEditView(ssid);

                        } else {
                            if (inUse) {
                                root.disconnectNetwork();
                            } else if (saved || !security) {
                                root.connectToNetwork(ssid, "");
                            } else {
                                root.selectedSsid = ssid;
                                root.errorMsg = "";
                                passwordInput.text = "";
                                passwordInput.forceActiveFocus();
                            }
                        }
                    }
                }

                RowLayout {
                    id: netRow

                    spacing: Tokens.spaceS

                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        leftMargin: 10
                        rightMargin: 10
                    }

                    Text {
                        text: root.signalIcon(signal)
                        color: root.signalColor(signal)
                        font.family: Fonts.family
                        font.pixelSize: Fonts.heading
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            text: ssid
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
                                visible: inUse
                                text: "已连接"
                                color: Colors.green
                                font.family: Fonts.family
                                font.pixelSize: Fonts.caption
                            }

                            Text {
                                visible: saved && !inUse
                                text: "已保存"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.caption
                            }

                            Text {
                                visible: !!security
                                text: "󰌾 " + security
                                color: Colors.overlay1
                                font.family: Fonts.family
                                font.pixelSize: Fonts.caption
                            }

                        }

                    }

                    Text {
                        text: signal + "%"
                        color: root.signalColor(signal)
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        font.weight: Font.DemiBold
                    }

                }

                Behavior on color {
                    ColorAnimation {
                        duration: 150
                    }

                }

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

        // ── 密码输入区 ──
        Rectangle {
            Layout.fillWidth: true
            visible: root.selectedSsid !== ""
            height: passwordCol.implicitHeight + 16
            radius: Tokens.radiusMS
            color: Colors.surface1

            ColumnLayout {
                id: passwordCol

                anchors.fill: parent
                anchors.margins: Tokens.spaceS
                spacing: 6

                Text {
                    text: "连接到 " + root.selectedSsid
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    font.weight: Font.DemiBold
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Tokens.spaceS

                    Rectangle {
                        Layout.fillWidth: true
                        height: 32
                        radius: Tokens.radiusS
                        color: Colors.surface0

                        TextInput {
                            id: passwordInput

                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            verticalAlignment: TextInput.AlignVCenter
                            color: Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            clip: true
                            echoMode: TextInput.Password
                            selectByMouse: true
                            onAccepted: {
                                if (text.length > 0)
                                    root.connectToNetwork(root.selectedSsid, text);

                            }
                            Keys.onEscapePressed: root.selectedSsid = ""

                            Text {
                                anchors.fill: parent
                                verticalAlignment: Text.AlignVCenter
                                text: "输入密码..."
                                color: Colors.overlay0
                                font: parent.font
                                visible: !parent.text && !parent.activeFocus
                            }

                        }

                    }

                    Rectangle {
                        width: connectBtnText.implicitWidth + 20
                        height: 32
                        radius: Tokens.radiusS
                        color: connectBtnArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25) : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)

                        Text {
                            id: connectBtnText

                            anchors.centerIn: parent
                            text: root.connecting ? "连接中..." : "连接"
                            color: Colors.blue
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            font.weight: Font.DemiBold
                        }

                        MouseArea {
                            id: connectBtnArea

                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            enabled: !root.connecting
                            onClicked: {
                                if (passwordInput.text.length > 0)
                                    root.connectToNetwork(root.selectedSsid, passwordInput.text);

                            }
                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: 150
                            }

                        }

                    }

                    Rectangle {
                        width: 32
                        height: 32
                        radius: Tokens.radiusS
                        color: cancelArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "󰅖"
                            color: cancelArea.containsMouse ? Colors.red : Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon

                            Behavior on color {
                                ColorAnimation {
                                    duration: 150
                                }

                            }

                        }

                        MouseArea {
                            id: cancelArea

                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.selectedSsid = ""
                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: 150
                            }

                        }

                    }

                }

            }

        }

    }

    // ── 编辑视图 ──
    NetworkEditView {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        visible: root.editingSsid !== ""
        panel: root
    }

}
