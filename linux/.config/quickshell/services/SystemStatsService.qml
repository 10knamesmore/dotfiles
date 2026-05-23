import "../theme"
import QtQuick
import Quickshell
import Quickshell.Io

// 系统监控采集 — 每秒读一次 /proc/stat + /proc/meminfo + /proc/net/dev，
// 一次 fork 拿全部数据，算好 CPU / 内存 / 网速写入 SystemStats。
// 取代旧的 Cpu/Memory/NetSpeed 三个 module 各自 1Hz 轮询（4 fork → 1 fork，并消除 NetSpeed up/down 重复读）。
Scope {
    id: root

    property var _prevCpuStat: null   // [total, idle]
    property var _prevCpuCores: []     // [[total, idle], ...]
    property real _prevNetRx: -1
    property real _prevNetTx: -1

    Process {
        id: reader
        property string _buf: ""
        command: ["sh", "-c", "grep '^cpu' /proc/stat; echo '@@@'; grep -E '^Mem(Total|Available)|^Swap(Total|Free)' /proc/meminfo; echo '@@@'; cat /proc/net/dev"]
        stdout: SplitParser {
            onRead: data => reader._buf += data + "\n"
        }
        onExited: {
            root._parse(reader._buf);
            reader._buf = "";
        }
    }

    function _parse(raw) {
        let segs = raw.split("@@@");
        if (segs.length < 3)
            return;
        root._parseCpu(segs[0]);
        root._parseMem(segs[1]);
        root._parseNet(segs[2]);
    }

    function _parseCpu(seg) {
        let cores = [];
        for (let line of seg.split("\n")) {
            let parts = line.trim().split(/\s+/);
            if (!parts[0] || !parts[0].startsWith("cpu"))
                continue;
            let nums = parts.slice(1).map(Number);
            if (nums.length < 5)
                continue;
            let idle = nums[3] + nums[4];
            let total = nums.reduce((a, b) => a + b, 0);
            if (parts[0] === "cpu") {
                if (root._prevCpuStat !== null) {
                    let dt = total - root._prevCpuStat[0];
                    let di = idle - root._prevCpuStat[1];
                    SystemStats.cpuUsage = dt > 0 ? Math.round((dt - di) / dt * 100) : 0;
                }
                root._prevCpuStat = [total, idle];
            } else {
                cores.push([total, idle]);
            }
        }
        if (root._prevCpuCores.length === cores.length && cores.length > 0) {
            let pcts = [];
            for (let i = 0; i < cores.length; i++) {
                let dt = cores[i][0] - root._prevCpuCores[i][0];
                let di = cores[i][1] - root._prevCpuCores[i][1];
                pcts.push(dt > 0 ? Math.round((dt - di) / dt * 100) : 0);
            }
            SystemStats.cpuCorePcts = pcts;
        }
        root._prevCpuCores = cores;
    }

    function _parseMem(seg) {
        let vals = {};
        for (let line of seg.split("\n")) {
            let m = line.match(/^(\w+):\s+(\d+)/);
            if (m)
                vals[m[1]] = parseInt(m[2]);
        }
        if (vals.MemTotal && vals.MemAvailable) {
            let used = vals.MemTotal - vals.MemAvailable;
            SystemStats.memUsagePct = Math.round(used / vals.MemTotal * 100);
            let usedGib = (used / 1.04858e+06).toFixed(1);
            let totalGib = (vals.MemTotal / 1.04858e+06).toFixed(1);
            SystemStats.memDetailText = usedGib + "/" + totalGib + "G";
            let swapUsed = vals.SwapTotal - (vals.SwapFree ?? 0);
            SystemStats.memTooltipText = "RAM: " + usedGib + " / " + totalGib + " GiB (" + SystemStats.memUsagePct + "%)" + (vals.SwapTotal > 0 ? "\nSwap: " + (swapUsed / 1.04858e+06).toFixed(1) + " / " + (vals.SwapTotal / 1.04858e+06).toFixed(1) + " GiB" : "");
        }
    }

    function _parseNet(seg) {
        let skipPrefixes = ["lo", "docker", "br-", "vmnet", "veth"];
        for (let line of seg.split("\n")) {
            let m = line.match(/^\s*(\S+):\s+(.*)/);
            if (!m)
                continue;
            let iface = m[1];
            let skip = false;
            for (let p of skipPrefixes) {
                if (iface.startsWith(p)) {
                    skip = true;
                    break;
                }
            }
            if (skip)
                continue;
            let fields = m[2].trim().split(/\s+/);
            if (fields.length < 10)
                continue;
            let rxBytes = parseInt(fields[0]);
            let txBytes = parseInt(fields[8]);
            SystemStats.netIface = iface;
            SystemStats.netDownTotal = rxBytes;
            SystemStats.netUpTotal = txBytes;
            if (root._prevNetRx >= 0) {
                SystemStats.netDownSpeed = rxBytes - root._prevNetRx;
                SystemStats.netUpSpeed = txBytes - root._prevNetTx;
            }
            root._prevNetRx = rxBytes;
            root._prevNetTx = txBytes;
            break; // 只取第一个匹配的物理接口
        }
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            reader._buf = "";
            reader.running = false;
            reader.running = true;
        }
    }
}
