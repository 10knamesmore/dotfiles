import "../state"
import QtQuick
import Quickshell
import Quickshell.Io

// 系统监控采集 — 每秒读一次 /proc/stat + /proc/meminfo + /proc/net/dev，
// 进程内直读（FileView.blockAllReads），算好 CPU / 内存 / 网速写入 SystemStats。
// 取代旧的每秒 fork `sh -c 'grep;grep;cat'`：零进程创建，消除 1Hz 采集造成的 CPU 尖峰。
Scope {
    id: root

    property var _prevCpuStat: null   // [total, idle]
    property var _prevCpuCores: []     // [[total, idle], ...]
    property real _prevNetRx: -1
    property real _prevNetTx: -1

    // /proc 是动态伪文件：blockAllReads 让 reload() 同步重读、text() 立即拿最新内容
    // （已实测 /proc/stat 可读，reload 后内容随之更新）。watchChanges 关掉——/proc 不触发 inotify。
    FileView { id: statFile; path: "/proc/stat"; blockAllReads: true; watchChanges: false; printErrors: false }
    FileView { id: memFile; path: "/proc/meminfo"; blockAllReads: true; watchChanges: false; printErrors: false }
    FileView { id: netFile; path: "/proc/net/dev"; blockAllReads: true; watchChanges: false; printErrors: false }

    // 原始 seg 语义不变：_parseCpu 自行跳过非 cpu 行、_parseMem 只挑 Mem/Swap 字段，
    // 故直接喂整份文件内容，无需再用 grep 预过滤。
    function _tick() {
        statFile.reload();
        memFile.reload();
        netFile.reload();
        root._parseCpu(statFile.text());
        root._parseMem(memFile.text());
        root._parseNet(netFile.text());
        root._pushHist();
    }

    // 环形历史缓冲：必须新数组赋值才能触发绑定（原地 push 不触发）
    function _pushHist() {
        let m = SystemStats.histMax;
        SystemStats.cpuHist = SystemStats.cpuHist.concat([SystemStats.cpuUsage]).slice(-m);
        SystemStats.memHist = SystemStats.memHist.concat([SystemStats.memUsagePct]).slice(-m);
        SystemStats.netUpHist = SystemStats.netUpHist.concat([SystemStats.netUpSpeed]).slice(-m);
        SystemStats.netDownHist = SystemStats.netDownHist.concat([SystemStats.netDownSpeed]).slice(-m);
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
        onTriggered: root._tick()
    }
}
