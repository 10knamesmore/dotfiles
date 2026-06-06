import "../theme"
import "../state"
import "../components"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

// btop 级系统监控面板 — 分 Tab（CPU/内存/网络），点哪个 bar 模块开哪个 Tab。
// 自带强采集器 monReader（仅 showing 时 1.5s 一次 fork 采全部），自维护多路历史。
PanelOverlay {
    id: root

    readonly property var barColors: ["#69ff94", "#2aa9ff", "#a6e3a1", "#94e2d5", "#f9e2af", "#fab387", "#ff9977", "#dd532e"]
    readonly property int histMax: 120

    // ── CPU ──
    property real cpuTotal: 0
    property var corePcts: []
    property var coreHist: []
    property var cpuTotalHist: []
    property real cpuFreq: 0
    property int cpuTemp: 0
    property var load: [0, 0, 0]
    property int procCount: 0
    property real ctxtRate: 0
    property int procsRunning: 0
    property real _prevCtxt: 0
    property var _prevCpu: null
    property var _prevCores: []

    // ── 内存 ──
    property real memTotalKb: 0
    property real memAvailKb: 0
    property real memFreeKb: 0
    property real memCachedKb: 0
    property real memBuffersKb: 0
    property var memUsedHist: []
    property int swapPct: 0
    property string swapText: ""
    property real memDirtyKb: 0
    property real memSlabKb: 0
    property real memActiveKb: 0
    property real memInactiveKb: 0
    readonly property real memUsedKb: Math.max(0, memTotalKb - memAvailKb)
    readonly property int memUsedPct: memTotalKb > 0 ? Math.round(memUsedKb / memTotalKb * 100) : 0

    // ── 磁盘 ──
    property var disks: []
    property real diskRead: 0
    property real diskWrite: 0
    property var diskReadHist: []
    property var diskWriteHist: []
    property var _prevDiskIO: null

    // ── 网络 ──
    property var ifaces: []
    property var netUpHist: []
    property var netDownHist: []
    property var _prevNet: ({})

    // ── 进程 ──
    property var procs: []

    // ── 网络连接（ss）──
    property var conns: []

    // ── 采样时间间隔（net/disk 速率用）──
    property real _lastT: 0
    property real _dt: 1.5

    // ── tab ──
    property string currentTab: "cpu"

    showing: PanelState.systemMonitorOpen
    panelWidth: 760
    panelHeight: Math.max(400, Math.min(root.height - 40, 1040))
    panelTargetY: 46
    onCloseRequested: PanelState.systemMonitorOpen = false

    // 采集延迟到展开动画结束，动画期内容静止（无 relayout/paint/fork）
    property bool _contentReady: false

    Timer {
        interval: Tokens.animElaborate + 50
        running: root.showing
        onTriggered: root._contentReady = true
    }

    Connections {
        target: root
        function onShowingChanged() {
            if (!root.showing)
                root._contentReady = false;
        }
    }

    Connections {
        target: PanelState
        function onSystemMonitorOpenChanged() {
            if (PanelState.systemMonitorOpen)
                root.currentTab = PanelState.systemMonitorTab;
        }
    }

    // ── 工具函数 ──
    function _push(arr, val) {
        return arr.concat([val]).slice(-root.histMax);
    }

    function tempColor(t) {
        if (t >= 80)
            return Colors.red;
        if (t >= 60)
            return Colors.yellow;
        return Colors.green;
    }

    function coreColor(pct) {
        return barColors[Math.min(Math.floor(pct / 12.5), 7)];
    }

    function fmtSpeed(bps) {
        if (bps < 1024)
            return bps.toFixed(0) + " B/s";
        if (bps < 1048576)
            return (bps / 1024).toFixed(1) + " KB/s";
        return (bps / 1048576).toFixed(2) + " MB/s";
    }

    function fmtBytes(b) {
        if (b < 1048576)
            return (b / 1024).toFixed(1) + " KB";
        if (b < 1073741824)
            return (b / 1048576).toFixed(1) + " MB";
        return (b / 1073741824).toFixed(2) + " GB";
    }

    function fmtGib(kb) {
        return (kb / 1048576).toFixed(1);
    }

    function drawGauge(ctx, w, h, value, accent) {
        let cx = w / 2, cy = h / 2;
        let r = Math.min(cx, cy) - 9;
        let lw = 8;
        let sa = 135 * Math.PI / 180;
        let sweep = 270 * Math.PI / 180;
        ctx.clearRect(0, 0, w, h);
        ctx.beginPath();
        ctx.arc(cx, cy, r, sa, sa + sweep);
        ctx.strokeStyle = Colors.surface1.toString();
        ctx.lineWidth = lw;
        ctx.lineCap = "round";
        ctx.stroke();
        if (value > 0) {
            ctx.beginPath();
            ctx.arc(cx, cy, r, sa, sa + sweep * Math.min(value, 100) / 100);
            ctx.strokeStyle = accent.toString();
            ctx.lineWidth = lw;
            ctx.lineCap = "round";
            ctx.stroke();
        }
    }

    function drawLine(ctx, w, h, arr, color, maxVal, doClear) {
        if (doClear)
            ctx.clearRect(0, 0, w, h);
        if (!arr || arr.length < 2)
            return;
        let n = arr.length;
        let mx = maxVal;
        if (mx <= 0) {
            mx = Math.max.apply(null, arr);
            if (mx <= 0)
                mx = 1;
        }
        ctx.beginPath();
        for (let i = 0; i < n; i++) {
            let x = i / (n - 1) * w;
            let y = h - Math.min(arr[i], mx) / mx * (h - 2) - 1;
            if (i === 0)
                ctx.moveTo(x, y);
            else
                ctx.lineTo(x, y);
        }
        ctx.strokeStyle = color.toString();
        ctx.lineWidth = 1.5;
        ctx.lineJoin = "round";
        ctx.stroke();
        ctx.lineTo(w, h);
        ctx.lineTo(0, h);
        ctx.closePath();
        ctx.fillStyle = Qt.rgba(color.r, color.g, color.b, 0.13);
        ctx.fill();
    }

    // ── 强采集器（仅 showing 时跑）──
    Process {
        id: monReader

        property string buf: ""

        command: ["sh", "-c", "echo '@@@CPU'; grep -E '^(cpu|ctxt|procs_running)' /proc/stat; echo '@@@MEM'; grep -E '^(MemTotal|MemFree|MemAvailable|Buffers|Cached|SwapTotal|SwapFree|Dirty|Writeback|Slab|Active|Inactive)' /proc/meminfo; echo '@@@NET'; cat /proc/net/dev; echo '@@@LOAD'; cat /proc/loadavg; echo '@@@SENSORS'; sensors -j 2>/dev/null || echo '{}'; echo '@@@DISK'; df -hT -x tmpfs -x devtmpfs -x efivarfs 2>/dev/null | tail -n +2; echo '@@@DISKIO'; cat /proc/diskstats; echo '@@@FREQ'; awk '{s+=$1;n++} END{if(n>0)print s/n/1000000}' /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq 2>/dev/null; echo '@@@SS'; ss -tunp 2>/dev/null | tail -n +2 | head -n 60; echo '@@@PROCS'; ps -eo pid,user:12,comm,%cpu,%mem --sort=-%cpu | head -n 81"]
        stdout: SplitParser {
            onRead: data => monReader.buf += data + "\n"
        }
        onExited: {
            let now = Date.now() / 1000;
            root._dt = root._lastT > 0 ? Math.max(0.1, now - root._lastT) : 1.5;
            root._lastT = now;
            root._parseAll(monReader.buf);
            monReader.buf = "";
        }
    }

    Process {
        id: killProc
        onExited: {
            monReader.buf = "";
            monReader.running = true;
        }
    }

    Timer {
        interval: 1500
        repeat: true
        running: root._contentReady
        triggeredOnStart: true
        onTriggered: {
            monReader.buf = "";
            monReader.running = false;
            monReader.running = true;
        }
    }

    // ── 解析 ──
    function _parseAll(raw) {
        let segs = raw.split("@@@");
        for (let s of segs) {
            let nl = s.indexOf("\n");
            if (nl < 0)
                continue;
            let tag = s.substring(0, nl).trim();
            let body = s.substring(nl + 1);
            if (tag === "CPU")
                _parseCpu(body);
            else if (tag === "MEM")
                _parseMem(body);
            else if (tag === "NET")
                _parseNet(body);
            else if (tag === "LOAD")
                _parseLoad(body);
            else if (tag === "SENSORS")
                _parseSensors(body);
            else if (tag === "DISK")
                _parseDisk(body);
            else if (tag === "DISKIO")
                _parseDiskIO(body);
            else if (tag === "FREQ")
                root.cpuFreq = parseFloat(body.trim()) || 0;
            else if (tag === "SS")
                _parseSS(body);
            else if (tag === "PROCS")
                _parseProcs(body);
        }
    }

    function _parseCpu(body) {
        let cores = [];
        let total = 0, totalIdle = 0;
        for (let line of body.split("\n")) {
            let p = line.trim().split(/\s+/);
            if (p[0] === "ctxt") {
                let c = parseInt(p[1]) || 0;
                if (root._prevCtxt > 0)
                    root.ctxtRate = Math.max(0, (c - root._prevCtxt) / root._dt);
                root._prevCtxt = c;
                continue;
            }
            if (p[0] === "procs_running") {
                root.procsRunning = parseInt(p[1]) || 0;
                continue;
            }
            if (!p[0] || !p[0].startsWith("cpu"))
                continue;
            let nums = p.slice(1).map(Number);
            if (nums.length < 5)
                continue;
            let idle = nums[3] + nums[4];
            let tot = nums.reduce((a, b) => a + b, 0);
            if (p[0] === "cpu") {
                total = tot;
                totalIdle = idle;
            } else {
                cores.push([tot, idle]);
            }
        }
        if (root._prevCpu) {
            let dt = total - root._prevCpu[0];
            let di = totalIdle - root._prevCpu[1];
            root.cpuTotal = dt > 0 ? Math.round((dt - di) / dt * 100) : 0;
            root.cpuTotalHist = _push(root.cpuTotalHist, root.cpuTotal);
        }
        root._prevCpu = [total, totalIdle];

        if (root._prevCores.length === cores.length && cores.length > 0) {
            let pcts = [];
            for (let i = 0; i < cores.length; i++) {
                let dt = cores[i][0] - root._prevCores[i][0];
                let di = cores[i][1] - root._prevCores[i][1];
                pcts.push(dt > 0 ? Math.round((dt - di) / dt * 100) : 0);
            }
            root.corePcts = pcts;
            let ch = root.coreHist;
            if (ch.length !== pcts.length)
                ch = pcts.map(() => []);
            let nch = [];
            for (let i = 0; i < pcts.length; i++)
                nch.push(ch[i].concat([pcts[i]]).slice(-root.histMax));
            root.coreHist = nch;
        }
        root._prevCores = cores;
    }

    function _parseMem(body) {
        let v = {};
        for (let line of body.split("\n")) {
            let m = line.match(/^(\w+):\s+(\d+)/);
            if (m)
                v[m[1]] = parseInt(m[2]);
        }
        if (v.MemTotal) {
            root.memTotalKb = v.MemTotal;
            root.memAvailKb = v.MemAvailable || 0;
            root.memFreeKb = v.MemFree || 0;
            root.memCachedKb = v.Cached || 0;
            root.memBuffersKb = v.Buffers || 0;
            root.memDirtyKb = v.Dirty || 0;
            root.memSlabKb = v.Slab || 0;
            root.memActiveKb = v.Active || 0;
            root.memInactiveKb = v.Inactive || 0;
            root.memUsedHist = _push(root.memUsedHist, root.memUsedPct);
        }
        if (v.SwapTotal && v.SwapTotal > 0) {
            let su = v.SwapTotal - (v.SwapFree || 0);
            root.swapPct = Math.round(su / v.SwapTotal * 100);
            root.swapText = fmtGib(su) + "/" + fmtGib(v.SwapTotal) + "G";
        } else {
            root.swapPct = 0;
            root.swapText = "无";
        }
    }

    function _parseNet(body) {
        let skip = ["lo", "docker", "br-", "vmnet", "veth", "virbr"];
        let list = [];
        let primaryUp = 0, primaryDown = 0;
        let nm = {};
        for (let line of body.split("\n")) {
            let m = line.match(/^\s*(\S+):\s+(.*)/);
            if (!m)
                continue;
            let name = m[1];
            let sk = false;
            for (let p of skip)
                if (name.startsWith(p)) {
                    sk = true;
                    break;
                }
            if (sk)
                continue;
            let f = m[2].trim().split(/\s+/);
            if (f.length < 10)
                continue;
            let rx = parseInt(f[0]);
            let tx = parseInt(f[8]);
            let up = 0, down = 0;
            if (root._prevNet[name]) {
                down = Math.max(0, (rx - root._prevNet[name][0]) / root._dt);
                up = Math.max(0, (tx - root._prevNet[name][1]) / root._dt);
            }
            nm[name] = [rx, tx];
            list.push({
                "name": name,
                "up": up,
                "down": down,
                "upTotal": tx,
                "downTotal": rx,
                "rxPkts": parseInt(f[1]) || 0,
                "txPkts": parseInt(f[9]) || 0,
                "rxErr": parseInt(f[2]) || 0,
                "txErr": parseInt(f[10]) || 0
            });
            if (list.length === 1) {
                primaryUp = up;
                primaryDown = down;
            }
        }
        root._prevNet = nm;
        root.ifaces = list;
        root.netUpHist = _push(root.netUpHist, primaryUp);
        root.netDownHist = _push(root.netDownHist, primaryDown);
    }

    function _parseLoad(body) {
        let p = body.trim().split(/\s+/);
        root.load = [parseFloat(p[0]) || 0, parseFloat(p[1]) || 0, parseFloat(p[2]) || 0];
    }

    function _parseSensors(body) {
        try {
            let j = JSON.parse(body);
            let tctl = 0, anyTemp = 0;
            for (let chip in j) {
                let c = j[chip];
                for (let feat in c) {
                    let f = c[feat];
                    if (typeof f !== "object")
                        continue;
                    for (let k in f) {
                        if (k.indexOf("_input") < 0)
                            continue;
                        let val = f[k];
                        let fl = feat.toLowerCase();
                        if (fl.indexOf("tctl") >= 0 || fl.indexOf("package") >= 0 || fl.indexOf("tdie") >= 0)
                            tctl = Math.max(tctl, val);
                        if (k.indexOf("temp") >= 0 || fl.indexOf("temp") >= 0 || fl.indexOf("core") >= 0)
                            anyTemp = Math.max(anyTemp, val);
                    }
                }
            }
            root.cpuTemp = Math.round(tctl > 0 ? tctl : anyTemp);
        } catch (e) {
        }
    }

    function _parseDisk(body) {
        let list = [];
        for (let line of body.split("\n")) {
            let p = line.trim().split(/\s+/);
            // df -hT: FS TYPE SIZE USED AVAIL USE% MOUNT
            if (p.length < 7)
                continue;
            if (p[6].indexOf("/efi") >= 0)
                continue;
            list.push({
                "mount": p[6],
                "used": p[3],
                "size": p[2],
                "pct": parseInt(p[5]) || 0
            });
        }
        root.disks = list;
    }

    function _parseDiskIO(body) {
        let tr = 0, tw = 0;
        for (let line of body.split("\n")) {
            let p = line.trim().split(/\s+/);
            if (p.length < 10)
                continue;
            if (!/^(sd[a-z]+|nvme\d+n\d+|vd[a-z]+|mmcblk\d+)$/.test(p[2]))
                continue;
            tr += parseInt(p[5]) || 0;
            tw += parseInt(p[9]) || 0;
        }
        if (root._prevDiskIO) {
            root.diskRead = Math.max(0, (tr - root._prevDiskIO[0]) * 512 / root._dt);
            root.diskWrite = Math.max(0, (tw - root._prevDiskIO[1]) * 512 / root._dt);
            root.diskReadHist = _push(root.diskReadHist, root.diskRead);
            root.diskWriteHist = _push(root.diskWriteHist, root.diskWrite);
        }
        root._prevDiskIO = [tr, tw];
    }

    function _parseSS(body) {
        let rows = [];
        for (let line of body.split("\n")) {
            let p = line.trim().split(/\s+/);
            if (p.length < 6)
                continue;
            let procStr = p.slice(6).join(" ");
            let m = procStr.match(/"([^"]+)",pid=(\d+)/);
            rows.push({
                "proto": p[0],
                "state": p[1],
                "local": p[4],
                "peer": p[5],
                "proc": m ? (m[1] + " (" + m[2] + ")") : ""
            });
        }
        root.conns = rows;
    }

    function _parseProcs(body) {
        let rows = [];
        let lines = body.split("\n");
        for (let i = 1; i < lines.length; i++) {
            let p = lines[i].trim().split(/\s+/);
            if (p.length < 5)
                continue;
            rows.push({
                "pid": p[0],
                "user": p[1],
                "name": p[2],
                "cpu": parseFloat(p[3]) || 0,
                "mem": parseFloat(p[4]) || 0
            });
        }
        root.procs = rows;
    }

    // 按当前 tab 排序并取前 16 的进程
    function topProcs(byMem) {
        let arr = root.procs.slice();
        arr.sort((a, b) => byMem ? (b.mem - a.mem) : (b.cpu - a.cpu));
        return arr;
    }

    // ── 进程行组件 ──
    component ProcHeader: RowLayout {
        Layout.fillWidth: true
        spacing: 8
        Text { text: "PID"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.weight: Font.Bold; Layout.preferredWidth: 54 }
        Text { text: "用户"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.weight: Font.Bold; Layout.preferredWidth: 78 }
        Text { text: "进程"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.weight: Font.Bold; Layout.fillWidth: true }
        Text { text: "CPU%"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.weight: Font.Bold; Layout.preferredWidth: 50; horizontalAlignment: Text.AlignRight }
        Text { text: "内存%"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.weight: Font.Bold; Layout.preferredWidth: 50; horizontalAlignment: Text.AlignRight }
        Item { Layout.preferredWidth: 24 }
    }

    component ProcRow: RowLayout {
        property var proc
        Layout.fillWidth: true
        spacing: 8

        Text {
            text: proc.pid
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            Layout.preferredWidth: 54
        }
        Text {
            text: proc.user
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            elide: Text.ElideRight
            Layout.preferredWidth: 78
        }
        Text {
            text: proc.name
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
        Text {
            text: proc.cpu.toFixed(1)
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            Layout.preferredWidth: 50
            horizontalAlignment: Text.AlignRight
        }
        Text {
            text: proc.mem.toFixed(1)
            color: Colors.mauve
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            Layout.preferredWidth: 50
            horizontalAlignment: Text.AlignRight
        }
        Rectangle {
            Layout.preferredWidth: 24
            Layout.preferredHeight: 24
            radius: Tokens.radiusS
            color: kArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.25) : "transparent"
            Text {
                anchors.centerIn: parent
                text: "✕"
                color: kArea.containsMouse ? Colors.red : Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            MouseArea {
                id: kArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    killProc.command = ["kill", parent.parent.proc.pid];
                    killProc.running = true;
                }
            }
        }
    }

    // 区块小标题
    component SectionLabel: Text {
        color: Colors.overlay0
        font.family: Fonts.family
        font.pixelSize: Fonts.small
        font.letterSpacing: 2
        font.weight: Font.Medium
    }

    // ── 主体（固定目标尺寸 + 居中，避免 morph 动画期跟随 relayout 卡顿）──
    ColumnLayout {
        anchors.centerIn: parent
        width: root.panelWidth - 2 * Tokens.spaceM
        height: root.panelHeight - 2 * Tokens.spaceM
        spacing: Tokens.spaceM

        // Tab 栏
        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spaceS

            Repeater {
                model: [{
                    "id": "cpu",
                    "label": "CPU",
                    "accent": Colors.blue
                }, {
                    "id": "memory",
                    "label": "内存",
                    "accent": Colors.mauve
                }, {
                    "id": "network",
                    "label": "网络",
                    "accent": Colors.teal
                }]

                delegate: Rectangle {
                    required property var modelData
                    readonly property bool sel: root.currentTab === modelData.id

                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    radius: Tokens.radiusM
                    color: sel ? Qt.rgba(modelData.accent.r, modelData.accent.g, modelData.accent.b, 0.18) : (tArea.containsMouse ? Colors.surface0 : "transparent")
                    border.color: sel ? modelData.accent : "transparent"
                    border.width: sel ? 1 : 0

                    Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: sel ? modelData.accent : Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.body
                        font.weight: sel ? Font.Bold : Font.Normal
                    }

                    MouseArea {
                        id: tArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.currentTab = modelData.id
                    }

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                        }
                    }
                }
            }
        }

        // 内容区
        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: root.currentTab === "cpu" ? 0 : (root.currentTab === "memory" ? 1 : 2)

            // ══════ CPU 页 ══════
            Flickable {
                contentHeight: cpuCol.implicitHeight
                clip: true

                ColumnLayout {
                    id: cpuCol
                    width: parent.width
                    spacing: Tokens.spaceM

                    // 仪表 + 信息
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Tokens.spaceL

                        Item {
                            Layout.preferredWidth: 150
                            Layout.preferredHeight: 150

                            Canvas {
                                id: cpuGauge
                                anchors.fill: parent
                                onPaint: root.drawGauge(getContext("2d"), width, height, root.cpuTotal, Colors.blue)
                                Connections {
                                    target: root
                                    function onCpuTotalChanged() {
                                        cpuGauge.requestPaint();
                                    }
                                }
                            }
                            Column {
                                anchors.centerIn: parent
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: root.cpuTotal + "%"
                                    color: Colors.text
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.h1
                                    font.weight: Font.Bold
                                }
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: "CPU"
                                    color: Colors.subtext0
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }
                            }
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            columns: 2
                            rowSpacing: 6
                            columnSpacing: 12

                            Text {
                                text: "频率"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                            Text {
                                text: root.cpuFreq.toFixed(2) + " GHz"
                                color: Colors.sky
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                            }
                            Text {
                                text: "温度"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                            Text {
                                text: root.cpuTemp + " °C"
                                color: root.tempColor(root.cpuTemp)
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                            }
                            Text {
                                text: "负载"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                            Text {
                                text: root.load[0].toFixed(2) + "  " + root.load[1].toFixed(2) + "  " + root.load[2].toFixed(2)
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                            }
                            Text {
                                text: "核数"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                            Text {
                                text: root.corePcts.length + " 线程"
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                            }
                            Text {
                                text: "切换/s"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                            Text {
                                text: Math.round(root.ctxtRate).toLocaleString()
                                color: Colors.sky
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                            }
                            Text {
                                text: "运行队列"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                            Text {
                                text: root.procsRunning + " 个"
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.weight: Font.DemiBold
                            }
                        }
                    }

                    SectionLabel {
                        text: "CPU 总占用 (120s)"
                    }
                    Canvas {
                        id: cpuTotalCurve
                        Layout.fillWidth: true
                        Layout.preferredHeight: 70
                        onPaint: root.drawLine(getContext("2d"), width, height, root.cpuTotalHist, Colors.blue, 100, true)
                        Connections {
                            target: root
                            function onCpuTotalHistChanged() {
                                if (root.showing)
                                    cpuTotalCurve.requestPaint();
                            }
                        }
                    }

                    SectionLabel {
                        text: "每核心"
                    }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        rowSpacing: 5
                        columnSpacing: 14

                        Repeater {
                            model: root.corePcts

                            delegate: RowLayout {
                                required property var modelData
                                required property int index
                                Layout.fillWidth: true
                                spacing: 6

                                Text {
                                    text: "C" + index
                                    color: Colors.overlay1
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                    Layout.preferredWidth: 24
                                }
                                Canvas {
                                    id: coreCurve
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 18
                                    onPaint: {
                                        let h = root.coreHist;
                                        root.drawLine(getContext("2d"), width, height, h[index] || [], root.coreColor(modelData), 100, true);
                                    }
                                    Connections {
                                        target: root
                                        function onCoreHistChanged() {
                                            if (root.showing)
                                                coreCurve.requestPaint();
                                        }
                                    }
                                }
                                Text {
                                    text: modelData + "%"
                                    color: root.coreColor(modelData)
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                    Layout.preferredWidth: 32
                                    horizontalAlignment: Text.AlignRight
                                }
                            }
                        }
                    }

                    Divider {
                        Layout.fillWidth: true
                    }

                    SectionLabel {
                        text: "进程 · CPU 占用"
                    }
                    ProcHeader {}
                    Repeater {
                        model: root.topProcs(false)
                        delegate: ProcRow {
                            required property var modelData
                            proc: modelData
                        }
                    }
                }
            }

            // ══════ 内存 页 ══════
            Flickable {
                contentHeight: memCol.implicitHeight
                clip: true

                ColumnLayout {
                    id: memCol
                    width: parent.width
                    spacing: Tokens.spaceM

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Tokens.spaceL

                        Item {
                            Layout.preferredWidth: 150
                            Layout.preferredHeight: 150
                            Canvas {
                                id: memGauge
                                anchors.fill: parent
                                onPaint: root.drawGauge(getContext("2d"), width, height, root.memUsedPct, Colors.mauve)
                                Connections {
                                    target: root
                                    function onMemAvailKbChanged() {
                                        memGauge.requestPaint();
                                    }
                                }
                            }
                            Column {
                                anchors.centerIn: parent
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: root.memUsedPct + "%"
                                    color: Colors.text
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.h1
                                    font.weight: Font.Bold
                                }
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: root.fmtGib(root.memUsedKb) + "/" + root.fmtGib(root.memTotalKb) + "G"
                                    color: Colors.subtext0
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.caption
                                }
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            // 内存细分条
                            Rectangle {
                                Layout.fillWidth: true
                                height: 14
                                radius: 4
                                color: Colors.surface1
                                clip: true

                                Row {
                                    anchors.fill: parent
                                    Rectangle {
                                        width: root.memTotalKb > 0 ? (root.memTotalKb - root.memFreeKb - root.memCachedKb - root.memBuffersKb) / root.memTotalKb * parent.width : 0
                                        height: parent.height
                                        color: Colors.mauve
                                    }
                                    Rectangle {
                                        width: root.memTotalKb > 0 ? root.memCachedKb / root.memTotalKb * parent.width : 0
                                        height: parent.height
                                        color: Colors.blue
                                    }
                                    Rectangle {
                                        width: root.memTotalKb > 0 ? root.memBuffersKb / root.memTotalKb * parent.width : 0
                                        height: parent.height
                                        color: Colors.teal
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                Text {
                                    text: "■ 已用 " + root.fmtGib(root.memTotalKb - root.memFreeKb - root.memCachedKb - root.memBuffersKb) + "G"
                                    color: Colors.mauve
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                }
                                Text {
                                    text: "■ 缓存 " + root.fmtGib(root.memCachedKb) + "G"
                                    color: Colors.blue
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                }
                                Text {
                                    text: "■ 缓冲 " + root.fmtGib(root.memBuffersKb) + "G"
                                    color: Colors.teal
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                }
                            }

                            // Swap
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6
                                Text {
                                    text: "Swap"
                                    color: Colors.subtext0
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                    Layout.preferredWidth: 40
                                }
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 6
                                    radius: 3
                                    color: Colors.surface1
                                    Rectangle {
                                        width: root.swapPct / 100 * parent.width
                                        height: parent.height
                                        radius: parent.radius
                                        color: root.swapPct > 60 ? Colors.red : Colors.yellow
                                    }
                                }
                                Text {
                                    text: root.swapText
                                    color: Colors.overlay1
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                }
                            }
                        }
                    }

                    SectionLabel {
                        text: "内存占用 (120s)"
                    }
                    Canvas {
                        id: memCurve
                        Layout.fillWidth: true
                        Layout.preferredHeight: 70
                        onPaint: root.drawLine(getContext("2d"), width, height, root.memUsedHist, Colors.mauve, 100, true)
                        Connections {
                            target: root
                            function onMemUsedHistChanged() {
                                if (root.showing)
                                    memCurve.requestPaint();
                            }
                        }
                    }

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 4
                        columnSpacing: 10
                        rowSpacing: 4
                        Text { text: "Active " + root.fmtGib(root.memActiveKb) + "G"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.small }
                        Text { text: "Inactive " + root.fmtGib(root.memInactiveKb) + "G"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.small }
                        Text { text: "Dirty " + root.fmtGib(root.memDirtyKb) + "G"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.small }
                        Text { text: "Slab " + root.fmtGib(root.memSlabKb) + "G"; color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.small }
                    }

                    Divider {
                        Layout.fillWidth: true
                    }

                    SectionLabel {
                        text: "磁盘"
                    }
                    Repeater {
                        model: root.disks
                        delegate: ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 2
                            RowLayout {
                                Layout.fillWidth: true
                                Text {
                                    text: modelData.mount
                                    color: Colors.subtext0
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                }
                                Item {
                                    Layout.fillWidth: true
                                }
                                Text {
                                    text: modelData.used + "/" + modelData.size
                                    color: Colors.overlay1
                                    font.family: Fonts.family
                                    font.pixelSize: Fonts.xs
                                }
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                height: 4
                                radius: 2
                                color: Colors.surface1
                                Rectangle {
                                    width: modelData.pct / 100 * parent.width
                                    height: parent.height
                                    radius: parent.radius
                                    color: modelData.pct > 90 ? Colors.red : (modelData.pct > 75 ? Colors.yellow : Colors.teal)
                                }
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 16
                        Text {
                            text: "磁盘读 " + root.fmtSpeed(root.diskRead)
                            color: Colors.green
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }
                        Text {
                            text: "写 " + root.fmtSpeed(root.diskWrite)
                            color: Colors.peach
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }
                        Item {
                            Layout.fillWidth: true
                        }
                    }

                    Divider {
                        Layout.fillWidth: true
                    }

                    SectionLabel {
                        text: "进程 · 内存占用"
                    }
                    ProcHeader {}
                    Repeater {
                        model: root.topProcs(true)
                        delegate: ProcRow {
                            required property var modelData
                            proc: modelData
                        }
                    }
                }
            }

            // ══════ 网络 页 ══════
            Flickable {
                contentHeight: netCol.implicitHeight
                clip: true

                ColumnLayout {
                    id: netCol
                    width: parent.width
                    spacing: Tokens.spaceM

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Tokens.spaceXL

                        ColumnLayout {
                            spacing: 0
                            Text {
                                text: "↑ " + root.fmtSpeed(root.netUpHist.length ? root.netUpHist[root.netUpHist.length - 1] : 0)
                                color: Colors.green
                                font.family: Fonts.family
                                font.pixelSize: Fonts.h2
                                font.weight: Font.Bold
                            }
                            Text {
                                text: "↓ " + root.fmtSpeed(root.netDownHist.length ? root.netDownHist[root.netDownHist.length - 1] : 0)
                                color: Colors.blue
                                font.family: Fonts.family
                                font.pixelSize: Fonts.h2
                                font.weight: Font.Bold
                            }
                        }
                        Item {
                            Layout.fillWidth: true
                        }
                        Text {
                            text: root.ifaces.length ? root.ifaces[0].name : ""
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                        }
                    }

                    SectionLabel {
                        text: "上行 (120s)"
                    }
                    Canvas {
                        id: upCurve
                        Layout.fillWidth: true
                        Layout.preferredHeight: 64
                        onPaint: root.drawLine(getContext("2d"), width, height, root.netUpHist, Colors.green, 0, true)
                        Connections {
                            target: root
                            function onNetUpHistChanged() {
                                if (root.showing)
                                    upCurve.requestPaint();
                            }
                        }
                    }

                    SectionLabel {
                        text: "下行 (120s)"
                    }
                    Canvas {
                        id: downCurve
                        Layout.fillWidth: true
                        Layout.preferredHeight: 64
                        onPaint: root.drawLine(getContext("2d"), width, height, root.netDownHist, Colors.blue, 0, true)
                        Connections {
                            target: root
                            function onNetDownHistChanged() {
                                if (root.showing)
                                    downCurve.requestPaint();
                            }
                        }
                    }

                    Divider {
                        Layout.fillWidth: true
                    }

                    SectionLabel {
                        text: "接口"
                    }
                    Repeater {
                        model: root.ifaces
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 10
                            Text {
                                text: modelData.name
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                                font.weight: Font.DemiBold
                                Layout.preferredWidth: 90
                            }
                            Text {
                                text: "↑ " + root.fmtSpeed(modelData.up)
                                color: Colors.green
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }
                            Text {
                                text: "↓ " + root.fmtSpeed(modelData.down)
                                color: Colors.blue
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }
                            Item {
                                Layout.fillWidth: true
                            }
                            Text {
                                text: "Σ↑" + root.fmtBytes(modelData.upTotal) + " ↓" + root.fmtBytes(modelData.downTotal)
                                color: Colors.overlay0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }
                        }
                    }

                    Divider {
                        Layout.fillWidth: true
                    }

                    SectionLabel {
                        text: "连接 · " + root.conns.length + " 个 (ss)"
                    }
                    Repeater {
                        model: root.conns
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 8
                            Text {
                                text: modelData.proto
                                color: Colors.overlay1
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                                Layout.preferredWidth: 40
                            }
                            Text {
                                text: modelData.state
                                color: Colors.teal
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                                elide: Text.ElideRight
                                Layout.preferredWidth: 76
                            }
                            Text {
                                text: modelData.peer
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                            Text {
                                text: modelData.proc
                                color: Colors.sky
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                                elide: Text.ElideRight
                                Layout.preferredWidth: 150
                            }
                        }
                    }
                }
            }
        }
    }
}
