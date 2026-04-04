import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import "../theme"

// 可折叠系统信息面板
Item {
    id: root

    property bool expanded: false

    implicitWidth: parent ? parent.width : 300
    implicitHeight: expanded ? infoCol.implicitHeight : 0
    clip: true

    Behavior on implicitHeight {
        NumberAnimation {
            duration: 250
            easing.type: Easing.OutCubic
        }
    }

    // ── 数据 ──
    property string cpuModel: ""
    property int cpuTemp: 0
    property string gpuModel: ""
    property int gpuTemp: 0
    property int memUsed: 0    // GB * 10
    property int memTotal: 0   // GB * 10
    property int memPct: 0
    property var disks: []     // [{mount, used, size, pct, fstype}]
    property string kernel: ""
    property string ipAddr: ""
    property int packages: 0
    property string hostName: ""
    property string wmVersion: ""
    property var monitors: []  // [{name, res}]
    property real loadAvg1: 0
    property real loadAvg5: 0
    property real loadAvg15: 0
    property int cpuCores: 16

    onExpandedChanged: {
        if (expanded)
            fetchAll();
    }

    Timer {
        running: root.expanded
        interval: 5000
        repeat: true
        onTriggered: fetchDynamic()
    }

    function fetchAll() {
        staticProc.running = true;
        fetchDynamic();
    }

    function fetchDynamic() {
        dynamicProc.running = true;
    }

    // 静态信息（只读一次）
    Process {
        id: staticProc
        command: ["sh", "-c", [
                // CPU model
                "echo \"CPU:$(grep 'model name' /proc/cpuinfo | head -1 | sed 's/.*: //')\"",
                // GPU model
                "echo \"GPU:$(nvidia-smi --query-gpu=name --format=csv,noheader,nounits 2>/dev/null || echo N/A)\"",
                // Kernel
                "echo \"KERNEL:$(uname -r)\"",
                // Hostname
                "echo \"HOST:$(hostname)\"",
                // Packages
                "echo \"PKG:$(pacman -Q 2>/dev/null | wc -l)\"",
                // WM version
                "echo \"WM:Hyprland $(hyprctl version -j 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin).get(\"tag\",\"?\"))' 2>/dev/null)\"",
                // Monitors
                "hyprctl monitors -j 2>/dev/null | python3 -c \"import sys,json;[print(f'MON:{m[\\\"name\\\"]} {m[\\\"width\\\"]}x{m[\\\"height\\\"]}@{int(m[\\\"refreshRate\\\"])}Hz') for m in json.load(sys.stdin)]\"",
                // CPU cores
                "echo \"CORES:$(nproc)\""].join(" && ")]
        stdout: SplitParser {
            onRead: data => {
                if (data.startsWith("CPU:"))
                    root.cpuModel = data.substring(4).trim();
                else if (data.startsWith("GPU:"))
                    root.gpuModel = data.substring(4).trim();
                else if (data.startsWith("KERNEL:"))
                    root.kernel = data.substring(7).trim();
                else if (data.startsWith("HOST:"))
                    root.hostName = data.substring(5).trim();
                else if (data.startsWith("PKG:"))
                    root.packages = parseInt(data.substring(4)) || 0;
                else if (data.startsWith("WM:"))
                    root.wmVersion = data.substring(3).trim();
                else if (data.startsWith("MON:")) {
                    let m = root.monitors.slice();
                    m.push(data.substring(4).trim());
                    root.monitors = m;
                } else if (data.startsWith("CORES:"))
                    root.cpuCores = parseInt(data.substring(6)) || 16;
            }
        }
    }

    // 动态信息（每 5 秒刷新）
    Process {
        id: dynamicProc
        command: ["sh", "-c", [
                // CPU temp
                "echo \"CPUT:$(sensors k10temp-pci-00c3 -j 2>/dev/null | python3 -c 'import sys,json;d=json.load(sys.stdin);print(int(d[\"k10temp-pci-00c3\"][\"Tctl\"][\"temp1_input\"]))' 2>/dev/null || echo 0)\"",
                // GPU temp
                "echo \"GPUT:$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits 2>/dev/null || echo 0)\"",
                // Memory
                "awk '/MemTotal/{t=$2} /MemAvailable/{a=$2} END{u=t-a; printf \"MEM:%d:%d:%d\\n\", u/1048576*10, t/1048576*10, (t-a)*100/t}' /proc/meminfo",
                // Load
                "echo \"LOAD:$(cut -d' ' -f1-3 /proc/loadavg)\"",
                // IP
                "echo \"IP:$(ip -br addr 2>/dev/null | grep UP | awk '{print $3}' | cut -d/ -f1 | head -1)\"",
                // Disks
                "df -hT -x tmpfs -x devtmpfs -x efivarfs 2>/dev/null | tail -n +2 | grep -v /efi | awk '{printf \"DISK:%s:%s:%s:%s:%s\\n\", $7,$3,$4,$6,$2}'"].join(" && ")]
        onStarted: {
            root._diskBuf = [];
        }
        stdout: SplitParser {
            onRead: data => {
                if (data.startsWith("CPUT:"))
                    root.cpuTemp = parseInt(data.substring(5)) || 0;
                else if (data.startsWith("GPUT:"))
                    root.gpuTemp = parseInt(data.substring(5)) || 0;
                else if (data.startsWith("MEM:")) {
                    let p = data.substring(4).split(":");
                    root.memUsed = parseInt(p[0]) || 0;
                    root.memTotal = parseInt(p[1]) || 0;
                    root.memPct = parseInt(p[2]) || 0;
                } else if (data.startsWith("LOAD:")) {
                    let p = data.substring(5).split(" ");
                    root.loadAvg1 = parseFloat(p[0]) || 0;
                    root.loadAvg5 = parseFloat(p[1]) || 0;
                    root.loadAvg15 = parseFloat(p[2]) || 0;
                } else if (data.startsWith("IP:"))
                    root.ipAddr = data.substring(3).trim();
                else if (data.startsWith("DISK:")) {
                    let p = data.substring(5).split(":");
                    root._diskBuf.push({
                        mount: p[0],
                        used: p[1],
                        size: p[2],
                        pct: p[3],
                        fstype: p[4]
                    });
                }
            }
        }
        onExited: root.disks = root._diskBuf
    }
    property var _diskBuf: []

    function tempColor(t) {
        if (t >= 80)
            return Colors.red;
        if (t >= 60)
            return Colors.yellow;
        return Colors.green;
    }

    function loadLabel() {
        let ratio = loadAvg1 / cpuCores;
        if (ratio < 0.5)
            return {
                text: "低",
                color: Colors.green
            };
        if (ratio < 1.0)
            return {
                text: "中",
                color: Colors.yellow
            };
        return {
            text: "高",
            color: Colors.red
        };
    }

    // ── UI ──
    ColumnLayout {
        id: infoCol
        width: parent.width
        spacing: 8

        // ── 硬件 Card ──
        InfoCard {
            Layout.fillWidth: true
            contentItem: ColumnLayout {
                spacing: 4
                InfoRow {
                    icon: "󰻠"
                    label: root.cpuModel
                    value: root.cpuTemp + "°C"
                    valueColor: tempColor(root.cpuTemp)
                }
                InfoRow {
                    icon: "󰾲"
                    label: root.gpuModel
                    value: root.gpuTemp + "°C"
                    valueColor: tempColor(root.gpuTemp)
                }
                InfoRow {
                    icon: "󰍛"
                    label: (root.memUsed / 10).toFixed(1) + " / " + (root.memTotal / 10).toFixed(1) + " GB"
                    value: root.memPct + "%"
                    valueColor: root.memPct > 85 ? Colors.red : root.memPct > 70 ? Colors.yellow : Colors.text
                }
                InfoRow {
                    icon: "󰊚"
                    label: "负载: " + loadLabel().text + " (" + root.loadAvg1.toFixed(1) + "  " + root.loadAvg5.toFixed(1) + "  " + root.loadAvg15.toFixed(1) + ")"
                    valueColor: loadLabel().color
                }
            }
        }

        // ── 存储 Card ──
        InfoCard {
            Layout.fillWidth: true
            contentItem: ColumnLayout {
                spacing: 4
                Repeater {
                    model: root.disks
                    delegate: InfoRow {
                        required property var modelData
                        icon: "󰋊"
                        label: modelData.mount + " : " + modelData.used + " / " + modelData.size + " - " + modelData.fstype
                        value: modelData.pct
                        valueColor: {
                            let n = parseInt(modelData.pct) || 0;
                            return n > 90 ? Colors.red : n > 80 ? Colors.yellow : Colors.text;
                        }
                    }
                }
            }
        }

        // ── 系统 Card ──
        InfoCard {
            Layout.fillWidth: true
            contentItem: ColumnLayout {
                spacing: 4
                InfoRow {
                    icon: "󰍹"
                    label: root.hostName
                }
                InfoRow {
                    icon: ""
                    label: root.wmVersion
                }
                InfoRow {
                    icon: "󰌘"
                    label: root.kernel
                }
                InfoRow {
                    icon: "󰏗"
                    label: root.packages + " (pacman)"
                }
            }
        }

        // ── 网络 & 显示器 Card ──
        InfoCard {
            Layout.fillWidth: true
            contentItem: ColumnLayout {
                spacing: 4
                InfoRow {
                    icon: "󰩟"
                    label: root.ipAddr || "N/A"
                }
                Repeater {
                    model: root.monitors
                    delegate: InfoRow {
                        required property var modelData
                        icon: "󰍹"
                        label: modelData
                    }
                }
            }
        }
    }

    // ── 组件 ──

    component InfoCard: Rectangle {
        property Item contentItem: null

        onContentItemChanged: {
            if (contentItem) {
                contentItem.parent = cardInner;
                contentItem.anchors.left = cardInner.left;
                contentItem.anchors.right = cardInner.right;
            }
        }

        implicitHeight: contentItem ? contentItem.implicitHeight + 20 : 20
        radius: 10
        color: cardHover.containsMouse ? Colors.surface1 : Colors.surface0
        border.color: cardHover.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.2) : Qt.rgba(1, 1, 1, 0.04)
        border.width: 1

        Behavior on color {
            ColorAnimation {
                duration: 200
                easing.type: Easing.OutCubic
            }
        }
        Behavior on border.color {
            ColorAnimation {
                duration: 200
                easing.type: Easing.OutCubic
            }
        }

        Item {
            id: cardInner
            anchors.fill: parent
            anchors.margins: 10
        }

        MouseArea {
            id: cardHover
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.NoButton
        }
    }

    component InfoRow: RowLayout {
        property string icon: ""
        property string label: ""
        property string value: ""
        property color valueColor: Colors.text

        Layout.fillWidth: true
        spacing: 8

        Text {
            text: icon
            color: Colors.overlay1
            font.family: "Hack Nerd Font"
            font.pixelSize: 13
            Layout.preferredWidth: 20
            horizontalAlignment: Text.AlignHCenter
        }
        Text {
            text: label
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }
        Text {
            visible: value.length > 0
            text: value
            color: valueColor
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            font.weight: Font.DemiBold
        }
    }
}
