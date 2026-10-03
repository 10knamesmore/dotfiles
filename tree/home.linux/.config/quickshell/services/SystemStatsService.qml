import "../state"
import QtQuick
import Quickshell
import Quickshell.Io

// 顶栏每秒进程内直读 /proc；面板关闭时也保留轻量趋势，不启动进程采集。
Scope {
    id: root

    property var previousCpu: ({})
    property var previousNetwork: ({})
    property real previousNetworkTime: 0

    function parseCpu(text, now) {
        const current = {};
        for (const line of text.split("\n")) {
            const fields = line.trim().split(/\s+/);
            if (!/^cpu\d*$/.test(fields[0]))
                continue;
            // guest 已计入 user，guest_nice 已计入 nice，不重复求和。
            const ticks = fields.slice(1, 9).map(Number);
            current[fields[0]] = {
                total: ticks.reduce((sum, value) => sum + value, 0),
                idle: ticks[3] + ticks[4]
            };
        }
        function usage(name) {
            const before = root.previousCpu[name];
            if (!before)
                return 0;
            const elapsed = current[name].total - before.total;
            return elapsed > 0 ? 100 * (elapsed - current[name].idle + before.idle) / elapsed : 0;
        }
        if (root.previousCpu.cpu && current.cpu) {
            SystemStats.cpu = {
                usage: Math.round(usage("cpu")),
                cores: Object.keys(current).filter(name => name !== "cpu").map(name => Math.round(usage(name)))
            };
            if (!PanelState.cpuOpen)
                ResourceStats.recordCpu(now, SystemStats.cpu.usage);
        }
        root.previousCpu = current;
    }
    function parseMemory(text, now) {
        const values = {};
        for (const line of text.split("\n")) {
            const match = line.match(/^(\w+):\s+(\d+)/);
            if (match)
                values[match[1]] = Number(match[2]) * 1024;
        }
        if (!values.MemTotal || values.MemAvailable === undefined)
            return;
        const used = values.MemTotal - values.MemAvailable;
        SystemStats.memory = {
            usage: Math.round(used / values.MemTotal * 100),
            usedBytes: used,
            totalBytes: values.MemTotal
        };
        if (!PanelState.memoryOpen)
            ResourceStats.recordMemory(now, used);
    }
    function parseNetwork(text, routes, now) {
        let defaultInterface = "";
        let lowestMetric = Infinity;
        for (const line of routes.trim().split("\n").slice(1)) {
            const fields = line.trim().split(/\s+/);
            if (fields[1] === "00000000" && (parseInt(fields[3], 16) & 1) && Number(fields[6]) < lowestMetric) {
                defaultInterface = fields[0];
                lowestMetric = Number(fields[6]);
            }
        }
        const elapsed = now - root.previousNetworkTime;
        const current = {};
        const interfaces = [];
        for (const line of text.split("\n")) {
            const match = line.match(/^\s*(\S+):\s+(.*)/);
            if (!match || match[1] === "lo")
                continue;
            const name = match[1];
            const fields = match[2].trim().split(/\s+/);
            const received = Number(fields[0]);
            const sent = Number(fields[8]);
            const before = root.previousNetwork[name];
            current[name] = {
                received: received,
                sent: sent
            };
            interfaces.push({
                name: name,
                downSpeed: before && elapsed > 0 ? Math.max(0, received - before.received) / elapsed : 0,
                upSpeed: before && elapsed > 0 ? Math.max(0, sent - before.sent) / elapsed : 0,
                downTotal: received,
                upTotal: sent
            });
        }
        let selected = interfaces.find(iface => iface.name === ResourceStats.networkInterface);
        if (!selected)
            selected = interfaces.find(iface => iface.name === defaultInterface) || interfaces[0];
        SystemStats.network = selected || {
            name: "",
            downSpeed: 0,
            upSpeed: 0,
            downTotal: 0,
            upTotal: 0
        };
        ResourceStats.networkInterface = selected ? selected.name : "";
        if (root.previousNetworkTime > 0 && !PanelState.networkStatsOpen)
            ResourceStats.recordNetwork(now, interfaces.filter(iface => root.previousNetwork[iface.name]));
        root.previousNetwork = current;
        root.previousNetworkTime = now;
    }
    function tick() {
        const now = Date.now() / 1000;
        statFile.reload();
        memFile.reload();
        netFile.reload();
        routeFile.reload();
        parseCpu(statFile.text(), now);
        parseMemory(memFile.text(), now);
        parseNetwork(netFile.text(), routeFile.text(), now);
    }

    FileView {
        id: statFile

        blockAllReads: true
        path: "/proc/stat"
        printErrors: false
        watchChanges: false
    }
    FileView {
        id: memFile

        blockAllReads: true
        path: "/proc/meminfo"
        printErrors: false
        watchChanges: false
    }
    FileView {
        id: netFile

        blockAllReads: true
        path: "/proc/net/dev"
        printErrors: false
        watchChanges: false
    }
    FileView {
        id: routeFile

        blockAllReads: true
        path: "/proc/net/route"
        printErrors: false
        watchChanges: false
    }
    Timer {
        interval: 1000
        repeat: true
        running: true
        triggeredOnStart: true

        onTriggered: root.tick()
    }
}
