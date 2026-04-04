import QtQuick
import QtQuick.Layouts
import "../theme"

// WiFi 连接编辑视图 — 由 NetworkPanel 实例化
Flickable {
    id: editView

    required property var panel  // NetworkPanel 引用

    contentHeight: editCol.implicitHeight
    clip: true

    ColumnLayout {
        id: editCol
        width: editView.width
        spacing: 10

        // ── 返回 + 标题 ──
        RowLayout {
            Layout.fillWidth: true; spacing: 8

            Rectangle {
                width: 28; height: 28; radius: 14
                color: backArea.containsMouse ? Colors.surface2 : "transparent"
                Behavior on color { ColorAnimation { duration: 150 } }
                Text {
                    anchors.centerIn: parent; text: "󰁍"
                    color: backArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: "Hack Nerd Font"; font.pixelSize: 14
                    Behavior on color { ColorAnimation { duration: 150 } }
                }
                MouseArea {
                    id: backArea; anchors.fill: parent
                    hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                    onClicked: editView.panel.editingSsid = ""
                }
            }

            Text {
                text: editView.panel.editingSsid; color: Colors.text
                font.family: "Hack Nerd Font"; font.pixelSize: 15; font.bold: true
                elide: Text.ElideRight; Layout.fillWidth: true
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1 }

        // ── 加载中 ──
        Text {
            visible: editView.panel.editLoading
            text: "加载中..."; color: Colors.overlay0
            font.family: "Hack Nerd Font"; font.pixelSize: 12
            Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 10
        }

        // ── 编辑表单 ──
        ColumnLayout {
            Layout.fillWidth: true; spacing: 8
            visible: !editView.panel.editLoading

            // ═══════════ 基本 ═══════════

            // 自动连接
            Rectangle {
                Layout.fillWidth: true; height: 40; radius: 10
                color: autoConnHover.containsMouse ? Colors.surface1 : "transparent"
                Behavior on color { ColorAnimation { duration: 150 } }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                    Text { text: "󰑐"; color: Colors.overlay1; font.family: "Hack Nerd Font"; font.pixelSize: 14 }
                    Text { text: "自动连接"; color: Colors.text; Layout.fillWidth: true; font.family: "Hack Nerd Font"; font.pixelSize: 12 }
                    Rectangle {
                        width: 40; height: 22; radius: 11
                        color: editView.panel.editAutoConnect ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.3) : Colors.surface2
                        Behavior on color { ColorAnimation { duration: 200 } }
                        Rectangle {
                            width: 16; height: 16; radius: 8; y: 3
                            x: editView.panel.editAutoConnect ? parent.width - 19 : 3
                            color: editView.panel.editAutoConnect ? Colors.green : Colors.overlay1
                            Behavior on x { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }
                            Behavior on color { ColorAnimation { duration: 200 } }
                        }
                    }
                }
                MouseArea {
                    id: autoConnHover; anchors.fill: parent
                    hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                    onClicked: editView.panel.editAutoConnect = !editView.panel.editAutoConnect
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1; opacity: 0.5 }

            // ═══════════ IPv4 ═══════════
            Text {
                text: "IPv4"; color: Colors.subtext0
                font.family: "Hack Nerd Font"; font.pixelSize: 10; font.weight: Font.DemiBold; Layout.leftMargin: 4
            }

            // IP method
            RowLayout {
                Layout.fillWidth: true; spacing: 6
                Repeater {
                    model: [ { value: "auto", label: "DHCP" }, { value: "manual", label: "手动" } ]
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true; height: 32; radius: 8
                        color: editView.panel.editIpMethod === modelData.value
                            ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)
                            : (ipMethodHover.containsMouse ? Colors.surface1 : Colors.surface0)
                        border.color: editView.panel.editIpMethod === modelData.value
                            ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.3) : "transparent"
                        border.width: 1
                        Behavior on color { ColorAnimation { duration: 150 } }
                        Text {
                            anchors.centerIn: parent; text: modelData.label
                            color: editView.panel.editIpMethod === modelData.value ? Colors.blue : Colors.text
                            font.family: "Hack Nerd Font"; font.pixelSize: 12
                            Behavior on color { ColorAnimation { duration: 150 } }
                        }
                        MouseArea {
                            id: ipMethodHover; anchors.fill: parent
                            hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                            onClicked: editView.panel.editIpMethod = modelData.value
                        }
                    }
                }
            }

            // 手动 IP 字段
            ColumnLayout {
                Layout.fillWidth: true; spacing: 6
                visible: editView.panel.editIpMethod === "manual"
                EditField { label: "IP 地址 (CIDR)"; placeholder: "192.168.1.100/24"; text: editView.panel.editIpAddr; onEdited: t => editView.panel.editIpAddr = t }
                EditField { label: "网关"; placeholder: "192.168.1.1"; text: editView.panel.editGateway; onEdited: t => editView.panel.editGateway = t }
            }

            EditField { label: "DNS"; placeholder: "8.8.8.8, 8.8.4.4"; text: editView.panel.editDns; onEdited: t => editView.panel.editDns = t }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1; opacity: 0.5 }

            // ═══════════ IPv6 ═══════════
            Text {
                text: "IPv6"; color: Colors.subtext0
                font.family: "Hack Nerd Font"; font.pixelSize: 10; font.weight: Font.DemiBold; Layout.leftMargin: 4
            }
            OptionRow {
                Layout.fillWidth: true
                model: [ { value: "auto", label: "自动" }, { value: "dhcp", label: "DHCP" }, { value: "disabled", label: "禁用" } ]
                current: editView.panel.editIp6Method
                onSelected: v => editView.panel.editIp6Method = v
            }
            EditField {
                visible: editView.panel.editIp6Method !== "disabled"
                label: "IPv6 DNS"; placeholder: "2001:4860:4860::8888"
                text: editView.panel.editIp6Dns; onEdited: t => editView.panel.editIp6Dns = t
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1; opacity: 0.5 }

            // ═══════════ 高级 ═══════════
            Text {
                text: "高级"; color: Colors.subtext0
                font.family: "Hack Nerd Font"; font.pixelSize: 10; font.weight: Font.DemiBold; Layout.leftMargin: 4
            }

            // 频段
            RowLayout {
                Layout.fillWidth: true; spacing: 6
                Text { text: "频段"; color: Colors.overlay1; font.family: "Hack Nerd Font"; font.pixelSize: 11; Layout.preferredWidth: 50 }
                OptionRow {
                    Layout.fillWidth: true; compact: true
                    model: [ { value: "", label: "自动" }, { value: "a", label: "5 GHz" }, { value: "bg", label: "2.4 GHz" } ]
                    current: editView.panel.editBand
                    onSelected: v => editView.panel.editBand = v
                }
            }

            EditField { label: "MAC 地址克隆"; placeholder: "留空使用真实 MAC"; text: editView.panel.editMac; onEdited: t => editView.panel.editMac = t }
            EditField { label: "MTU"; placeholder: "auto"; text: editView.panel.editMtu; onEdited: t => editView.panel.editMtu = t }

            // 隐藏网络
            ToggleRow {
                Layout.fillWidth: true; icon: "󰈈"; label: "隐藏网络"
                toggled: editView.panel.editHidden
                onClicked: editView.panel.editHidden = !editView.panel.editHidden
            }

            // 计量
            RowLayout {
                Layout.fillWidth: true; spacing: 6
                Text { text: "计量"; color: Colors.overlay1; font.family: "Hack Nerd Font"; font.pixelSize: 11; Layout.preferredWidth: 50 }
                OptionRow {
                    Layout.fillWidth: true; compact: true
                    model: [ { value: "unknown", label: "自动" }, { value: "yes", label: "是" }, { value: "no", label: "否" } ]
                    current: editView.panel.editMetered
                    onSelected: v => editView.panel.editMetered = v
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1; opacity: 0.5 }

            // ═══════════ 代理 ═══════════
            Text {
                text: "代理"; color: Colors.subtext0
                font.family: "Hack Nerd Font"; font.pixelSize: 10; font.weight: Font.DemiBold; Layout.leftMargin: 4
            }
            OptionRow {
                Layout.fillWidth: true
                model: [ { value: "none", label: "无" }, { value: "auto", label: "PAC" } ]
                current: editView.panel.editProxyMethod
                onSelected: v => editView.panel.editProxyMethod = v
            }
            EditField {
                visible: editView.panel.editProxyMethod === "auto"
                label: "PAC URL"; placeholder: "file:///etc/proxy.pac"
                text: editView.panel.editProxyPacUrl; onEdited: t => editView.panel.editProxyPacUrl = t
            }
            ToggleRow {
                Layout.fillWidth: true; icon: "󰖟"; label: "仅浏览器"
                visible: editView.panel.editProxyMethod === "auto"
                toggled: editView.panel.editProxyBrowserOnly
                onClicked: editView.panel.editProxyBrowserOnly = !editView.panel.editProxyBrowserOnly
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1; opacity: 0.5 }

            // ── 保存反馈 ──
            Text {
                visible: editView.panel.editSaveMsg !== ""
                text: editView.panel.editSaveMsg
                color: editView.panel.editSaveMsg === "已保存" ? Colors.green : Colors.red
                font.family: "Hack Nerd Font"; font.pixelSize: 11
                Layout.alignment: Qt.AlignHCenter
            }

            // ── 操作按钮 ──
            RowLayout {
                Layout.fillWidth: true; spacing: 8

                Rectangle {
                    Layout.fillWidth: true; height: 36; radius: 10
                    color: saveBtnArea.containsMouse
                        ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25)
                        : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)
                    Behavior on color { ColorAnimation { duration: 150 } }
                    Text {
                        anchors.centerIn: parent; text: "保存"; color: Colors.blue
                        font.family: "Hack Nerd Font"; font.pixelSize: 13; font.weight: Font.DemiBold
                    }
                    MouseArea {
                        id: saveBtnArea; anchors.fill: parent
                        hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                        onClicked: editView.panel.saveEdits()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true; height: 36; radius: 10
                    color: forgetBtnArea.containsMouse
                        ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.2)
                        : Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.1)
                    Behavior on color { ColorAnimation { duration: 150 } }
                    Text {
                        anchors.centerIn: parent; text: "忘记网络"; color: Colors.red
                        font.family: "Hack Nerd Font"; font.pixelSize: 13; font.weight: Font.DemiBold
                    }
                    MouseArea {
                        id: forgetBtnArea; anchors.fill: parent
                        hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                        onClicked: editView.panel.forgetNetwork()
                    }
                }
            }
        }
    }
}
