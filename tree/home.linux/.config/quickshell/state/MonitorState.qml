import QtQuick
pragma Singleton

// 显示器状态单例 —— MonitorService 写入、UI（DisplayPanel）读取的中枢。
// 仿 SystemStats(单例) ← SystemStatsService(常驻服务) 的分工：数据容器在此，
// 逻辑/IPC 在 services/MonitorService.qml。UI 通过下方意图信号回调 Service。
QtObject {
    id: root

    // ── 数据（由 MonitorService 写入）──
    // 每项：{ name, description, enabled, mode, x, y, scale, transform,
    //        width, height, refreshRate, availableModes, focused, primary,
    //        color:{ cm, sdrBrightness, sdrSaturation } }
    property var monitors: []
    property string signature: ""     // 当前显示器组合签名
    property string primaryName: ""   // 当前主显示器 name
    property bool applying: false
    property string errorMsg: ""
    property int revertSecs: 0        // >0 = 回滚确认倒计时进行中

    // 已证实吃不下 HDR 的屏（name → true）。Hyprland 不暴露 supportsHDR，
    // 硬件不支持时只会把 cm 悄悄退回 srgb，只能应用一次再回读才知道。
    // UI 据此把 HDR 开关置灰，避免用户反复拨一个不会生效的开关。
    property var hdrUnsupported: ({})

    // ── 意图信号（UI 发出 → MonitorService 接收）──
    signal applyRequested(var layouts, string primary)
    signal keepRequested()
    signal revertRequested()
    signal refreshRequested()

    function requestApply(layouts, primary) {
        applyRequested(layouts, primary);
    }
    function keep() {
        keepRequested();
    }
    function revert() {
        revertRequested();
    }
    function refresh() {
        refreshRequested();
    }
}
