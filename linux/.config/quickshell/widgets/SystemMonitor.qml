import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 桌面浮动系统监控 — 双圆弧仪表 + 温度 + 负载 + Swap + 磁盘 + 网络 + 进程数 + Uptime
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 布局配置（改这里调整位置和大小）──
    property int widgetMarginRight: 20  // 右边距
    property int widgetY: 280           // 上边距

    // ── CPU 数据 ──
    property int cpuUsage: 0
    property var prevStat: null
    property var prevCores: []
    property var corePcts: []
    readonly property var barColors: ["#69ff94", "#2aa9ff", "#f8f8f2", "#f8f8f2", "#ffffa5", "#ffffa5", "#ff9977", "#dd532e"]

    // ── 内存数据 ──
    property int memUsagePct: 0
    property string memLabel: ""
    property int swapUsagePct: 0
    property string swapLabel: ""

    // ── 温度 / 磁盘 / 网络 / 系统 ──
    property int cpuTemp: 0
    property int gpuTemp: 0
    property int gpuUsage: 0
    property var disks: []
    property var _diskBuf: []
    property real netUp: 0
    property real netDown: 0
    property var _prevNet: null
    property real loadAvg1: 0
    property real loadAvg5: 0
    property real loadAvg15: 0
    property int processCount: 0
    property string uptime: ""
    property string cpuFreq: ""

    function tempColor(t) {
        if (t >= 80) return Colors.red;
        if (t >= 60) return Colors.yellow;
        return Colors.green;
    }

    function formatSpeed(kbps) {
        if (kbps >= 1024) return (kbps / 1024).toFixed(1) + " MB/s";
        return Math.round(kbps) + " KB/s";
    }

    aboveWindows: false
    anchors.top: true
    anchors.right: true
    implicitWidth: 280
    implicitHeight: 520
    margins.top: widgetY
    margins.right: widgetMarginRight
    visible: PanelState.systemMonitorVisible
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    Component.onCompleted: {
        cpuReader.running = true;
        memReader.running = true;
        dynReader.running = true;
    }

    // ── CPU 轮询 ──
    Process {
        id: cpuReader
        command: ["bash", "-c", "grep '^cpu' /proc/stat"]
        onExited: {
            let lines = cpuReader.stdout.lines;
            cpuReader.stdout.lines = [];
            let cores = [];
            for (let i = 0; i < lines.length; i++) {
                let parts = lines[i].trim().split(/\s+/);
                let nums = parts.slice(1).map(Number);
                let idle = nums[3] + nums[4];
                let total = nums.reduce((a, b) => a + b, 0);
                if (parts[0] === "cpu") {
                    if (root.prevStat !== null) {
                        let dt = total - root.prevStat[0];
                        let di = idle - root.prevStat[1];
                        root.cpuUsage = dt > 0 ? Math.round((dt - di) / dt * 100) : 0;
                    }
                    root.prevStat = [total, idle];
                } else {
                    cores.push([total, idle]);
                }
            }
            if (root.prevCores.length === cores.length && cores.length > 0) {
                let pcts = [];
                for (let i = 0; i < cores.length; i++) {
                    let dt = cores[i][0] - root.prevCores[i][0];
                    let di = cores[i][1] - root.prevCores[i][1];
                    pcts.push(dt > 0 ? Math.round((dt - di) / dt * 100) : 0);
                }
                root.corePcts = pcts;
            }
            root.prevCores = cores;
            cpuGauge.requestPaint();
        }
        stdout: SplitParser {
            property var lines: []
            onRead: data => { lines.push(data); }
        }
    }

    // ── 内存 + Swap 轮询 ──
    Process {
        id: memReader
        command: ["bash", "-c", "grep -E '^(MemTotal|MemAvailable|SwapTotal|SwapFree)' /proc/meminfo"]
        onExited: {
            let v = memReader.stdout.vals;
            memReader.stdout.vals = {};
            if (v.MemTotal && v.MemAvailable) {
                let used = v.MemTotal - v.MemAvailable;
                root.memUsagePct = Math.round(used / v.MemTotal * 100);
                root.memLabel = (used / 1.04858e+06).toFixed(1) + "/" + (v.MemTotal / 1.04858e+06).toFixed(1) + "G";
            }
            if (v.SwapTotal && v.SwapTotal > 0) {
                let swapUsed = v.SwapTotal - (v.SwapFree || 0);
                root.swapUsagePct = Math.round(swapUsed / v.SwapTotal * 100);
                root.swapLabel = (swapUsed / 1.04858e+06).toFixed(1) + "/" + (v.SwapTotal / 1.04858e+06).toFixed(1) + "G";
            }
            memGauge.requestPaint();
        }
        stdout: SplitParser {
            property var vals: ({})
            onRead: data => {
                let m = data.match(/^(\w+):\s+(\d+)/);
                if (m) vals[m[1]] = parseInt(m[2]);
            }
        }
    }

    // ── 温度 + 磁盘 + 网络 + 系统信息 ──
    Process {
        id: dynReader
        command: ["sh", "-c", [
            "echo \"CPUT:$(sensors k10temp-pci-00c3 -j 2>/dev/null | python3 -c 'import sys,json;d=json.load(sys.stdin);print(int(d[\"k10temp-pci-00c3\"][\"Tctl\"][\"temp1_input\"]))' 2>/dev/null || echo 0)\"",
            "echo \"GPUT:$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits 2>/dev/null || echo 0)\"",
            "echo \"GPUU:$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits 2>/dev/null || echo 0)\"",
            "echo \"LOAD:$(cut -d' ' -f1-3 /proc/loadavg)\"",
            "echo \"PROCS:$(ls -d /proc/[0-9]* 2>/dev/null | wc -l)\"",
            "echo \"UPTIME:$(uptime -p 2>/dev/null | sed 's/up //')\"",
            "echo \"FREQ:$(awk '{printf \"%.1f\", $1/1000000}' /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq 2>/dev/null || echo 0)\"",
            "df -hT -x tmpfs -x devtmpfs -x efivarfs 2>/dev/null | tail -n +2 | grep -v /efi | awk '{printf \"DISK:%s:%s:%s:%s\\n\", $7,$3,$4,$6}'",
            "cat /proc/net/dev | awk 'NR>2{gsub(\":\",\"\",$1); if($1!=\"lo\") printf \"NET:%s:%s:%s\\n\",$1,$2,$10}'"
        ].join(" && ")]
        onStarted: { root._diskBuf = []; }
        onExited: {
            root.disks = root._diskBuf;
            let netBytes = dynReader.stdout._netBytes;
            dynReader.stdout._netBytes = {};
            let totalRx = 0, totalTx = 0;
            for (let iface in netBytes) {
                totalRx += netBytes[iface][0];
                totalTx += netBytes[iface][1];
            }
            if (root._prevNet !== null) {
                root.netDown = (totalRx - root._prevNet[0]) / 1024 / 5;
                root.netUp = (totalTx - root._prevNet[1]) / 1024 / 5;
            }
            root._prevNet = [totalRx, totalTx];
        }
        stdout: SplitParser {
            property var _netBytes: ({})
            onRead: data => {
                if (data.startsWith("CPUT:"))
                    root.cpuTemp = parseInt(data.substring(5)) || 0;
                else if (data.startsWith("GPUT:"))
                    root.gpuTemp = parseInt(data.substring(5)) || 0;
                else if (data.startsWith("GPUU:"))
                    root.gpuUsage = parseInt(data.substring(5)) || 0;
                else if (data.startsWith("LOAD:")) {
                    let p = data.substring(5).split(" ");
                    root.loadAvg1 = parseFloat(p[0]) || 0;
                    root.loadAvg5 = parseFloat(p[1]) || 0;
                    root.loadAvg15 = parseFloat(p[2]) || 0;
                } else if (data.startsWith("PROCS:"))
                    root.processCount = parseInt(data.substring(6)) || 0;
                else if (data.startsWith("UPTIME:"))
                    root.uptime = data.substring(7).trim();
                else if (data.startsWith("FREQ:"))
                    root.cpuFreq = data.substring(5).trim();
                else if (data.startsWith("DISK:")) {
                    let p = data.substring(5).split(":");
                    root._diskBuf.push({ "mount": p[0], "used": p[1], "size": p[2], "pct": p[3] });
                } else if (data.startsWith("NET:")) {
                    let p = data.substring(4).split(":");
                    _netBytes[p[0]] = [parseInt(p[1]) || 0, parseInt(p[2]) || 0];
                }
            }
        }
    }

    Timer {
        interval: 1000; running: true; repeat: true
        onTriggered: {
            cpuReader.running = false; cpuReader.running = true;
            memReader.running = false; memReader.running = true;
        }
    }

    Timer {
        interval: 5000; running: true; repeat: true
        onTriggered: { dynReader.running = false; dynReader.running = true; }
    }

    // ── 主体 ──
    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 260
        height: 500
        radius: Tokens.radiusXL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(Tokens.borderBase.r, Tokens.borderBase.g, Tokens.borderBase.b, Tokens.borderAlpha)
        border.width: 1

        SoftShadow { anchors.fill: parent; radius: parent.radius }
        InnerGlow {}

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 6

            // ── 双圆弧仪表 ──
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 16

                Item {
                    Layout.preferredWidth: 100; Layout.preferredHeight: 100
                    Canvas {
                        id: cpuGauge; anchors.fill: parent
                        onPaint: drawGauge(getContext("2d"), width, height, root.cpuUsage, Colors.blue)
                    }
                    Column {
                        anchors.centerIn: parent; spacing: 0
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: root.cpuUsage + "%"; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.bodyLarge; font.weight: Font.Bold }
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "CPU"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                    }
                }

                Item {
                    Layout.preferredWidth: 100; Layout.preferredHeight: 100
                    Canvas {
                        id: memGauge; anchors.fill: parent
                        onPaint: drawGauge(getContext("2d"), width, height, root.memUsagePct, Colors.mauve)
                    }
                    Column {
                        anchors.centerIn: parent; spacing: 0
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: root.memUsagePct + "%"; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.bodyLarge; font.weight: Font.Bold }
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: root.memLabel || "MEM"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                    }
                }
            }

            // ── CPU Core 微型柱状图 ──
            Row {
                Layout.alignment: Qt.AlignHCenter
                spacing: 1
                visible: root.corePcts.length > 0
                Repeater {
                    model: root.corePcts
                    delegate: Rectangle {
                        required property var modelData
                        width: 3; height: Math.max(2, modelData / 100 * 24); radius: 1
                        color: barColors[Math.min(Math.floor(modelData / 12.5), 7)]
                        anchors.bottom: parent.bottom
                    }
                }
            }

            Divider { Layout.fillWidth: true }

            // ── 温度 + GPU 使用率 ──
            GridLayout {
                Layout.fillWidth: true
                columns: 2
                rowSpacing: 3
                columnSpacing: 8

                // CPU 温度
                RowLayout {
                    spacing: 3
                    Text { text: "CPU"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                    Text { text: root.cpuTemp + "°C"; color: tempColor(root.cpuTemp); font.family: Fonts.family; font.pixelSize: Fonts.small; font.weight: Font.DemiBold }
                }
                // CPU 频率
                RowLayout {
                    spacing: 3
                    Text { text: "Freq"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                    Text { text: root.cpuFreq + " GHz"; color: Colors.sky; font.family: Fonts.family; font.pixelSize: Fonts.small; font.weight: Font.DemiBold }
                }
                // GPU 温度
                RowLayout {
                    spacing: 3
                    Text { text: "GPU"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                    Text { text: root.gpuTemp + "°C"; color: tempColor(root.gpuTemp); font.family: Fonts.family; font.pixelSize: Fonts.small; font.weight: Font.DemiBold }
                }
                // GPU 使用率
                RowLayout {
                    spacing: 3
                    Text { text: "GPU%"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                    Text { text: root.gpuUsage + "%"; color: root.gpuUsage > 80 ? Colors.red : root.gpuUsage > 50 ? Colors.yellow : Colors.green; font.family: Fonts.family; font.pixelSize: Fonts.small; font.weight: Font.DemiBold }
                }
            }

            // ── Swap ──
            RowLayout {
                Layout.fillWidth: true
                visible: root.swapLabel !== ""
                spacing: 4
                Text { text: "Swap"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                Rectangle {
                    Layout.fillWidth: true; height: 3; radius: 1.5; color: Colors.surface1
                    Rectangle {
                        width: root.swapUsagePct / 100 * parent.width; height: parent.height; radius: parent.radius
                        color: root.swapUsagePct > 80 ? Colors.red : root.swapUsagePct > 50 ? Colors.yellow : Colors.teal
                    }
                }
                Text { text: root.swapLabel; color: Colors.overlay1; font.family: Fonts.family; font.pixelSize: Fonts.xs }
            }

            // ── 负载均值 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 4
                Text { text: "Load"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                Text { text: root.loadAvg1.toFixed(2); color: root.loadAvg1 > 8 ? Colors.red : root.loadAvg1 > 4 ? Colors.yellow : Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.small; font.weight: Font.DemiBold }
                Text { text: root.loadAvg5.toFixed(2); color: Colors.overlay1; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                Text { text: root.loadAvg15.toFixed(2); color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                Item { Layout.fillWidth: true }
                Text { text: root.processCount + " procs"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
            }

            Divider { Layout.fillWidth: true }

            // ── 磁盘 ──
            Repeater {
                model: root.disks
                delegate: ColumnLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 2
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: modelData.mount; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                        Item { Layout.fillWidth: true }
                        Text { text: modelData.used + "/" + modelData.size; color: Colors.overlay1; font.family: Fonts.family; font.pixelSize: Fonts.xs }
                    }
                    Rectangle {
                        Layout.fillWidth: true; height: 3; radius: 1.5; color: Colors.surface1
                        Rectangle {
                            width: { let pct = parseInt(modelData.pct) || 0; return pct / 100 * parent.width; }
                            height: parent.height; radius: parent.radius
                            color: { let pct = parseInt(modelData.pct) || 0; return pct > 90 ? Colors.red : pct > 75 ? Colors.yellow : Colors.teal; }
                        }
                    }
                }
            }

            // ── 网络吞吐 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                RowLayout {
                    spacing: 3
                    Text { text: "\u2191"; color: Colors.green; font.family: Fonts.family; font.pixelSize: Fonts.caption; font.weight: Font.Bold }
                    Text { text: formatSpeed(root.netUp); color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                }
                RowLayout {
                    spacing: 3
                    Text { text: "\u2193"; color: Colors.blue; font.family: Fonts.family; font.pixelSize: Fonts.caption; font.weight: Font.Bold }
                    Text { text: formatSpeed(root.netDown); color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                }
                Item { Layout.fillWidth: true }
            }

            Divider { Layout.fillWidth: true }

            // ── Uptime ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 4
                Text { text: "\u23f1"; color: Colors.overlay1; font.family: Fonts.family; font.pixelSize: Fonts.caption }
                Text { text: root.uptime || "..."; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.xs }
            }
        }
    }

    function drawGauge(ctx, w, h, value, accentColor) {
        let cx = w / 2, cy = h / 2;
        let r = Math.min(cx, cy) - 8;
        let lineW = 5;
        let startAngle = 135 * Math.PI / 180;
        let totalSweep = 270 * Math.PI / 180;
        ctx.clearRect(0, 0, w, h);
        ctx.beginPath();
        ctx.arc(cx, cy, r, startAngle, startAngle + totalSweep);
        ctx.strokeStyle = Colors.surface1.toString();
        ctx.lineWidth = lineW;
        ctx.lineCap = "round";
        ctx.stroke();
        if (value > 0) {
            let endAngle = startAngle + totalSweep * Math.min(value, 100) / 100;
            ctx.beginPath();
            ctx.arc(cx, cy, r, startAngle, endAngle);
            ctx.strokeStyle = accentColor.toString();
            ctx.lineWidth = lineW;
            ctx.lineCap = "round";
            ctx.stroke();
        }
    }
}
