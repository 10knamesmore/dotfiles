import QtQuick
pragma Singleton

// 系统监控数据 — 由 SystemStatsService 每秒更新一次；bar 的 Cpu/Memory/NetSpeed 模块纯读取
QtObject {
    // ── CPU ──
    property int cpuUsage: 0
    property var cpuCorePcts: []        // per-core 使用率数组
    // ── 内存 ──
    property int memUsagePct: 0
    property string memDetailText: ""   // "8.1/15.5G"
    property string memTooltipText: ""
    // ── 网络（取第一个物理接口）──
    property string netIface: ""
    property real netUpSpeed: 0         // bytes/s
    property real netDownSpeed: 0
    property real netUpTotal: 0
    property real netDownTotal: 0
}
