import QtQuick
pragma Singleton

// 顶栏资源统计 — SystemStatsService 每秒更新，Cpu/Memory/NetSpeed 胶囊读取；不保存历史曲线。
QtObject {
    // ── CPU ──
    property int cpuUsage: 0
    property var cpuCorePcts: []        // per-core 使用率数组
    // ── 内存 ──
    property int memUsagePct: 0
    property string memTooltipText: ""  // "RAM: 8.1 / 15.5 GiB (52%)\nSwap: ..."
    // ── 网络（取第一个物理接口）──
    property string netIface: ""
    property real netUpSpeed: 0         // bytes/s
    property real netDownSpeed: 0
    property real netUpTotal: 0
    property real netDownTotal: 0
}
