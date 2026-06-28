import QtQuick
pragma Singleton

// 显示器状态单例 —— MonitorService 写入、UI（DisplayPanel）读取的中枢。
// 仿 SystemStats(单例) ← SystemStatsService(常驻服务) 的分工：数据容器在此，
// 逻辑/IPC 在 services/MonitorService.qml。UI 通过下方意图信号回调 Service。
QtObject {
    id: root

    // ── 数据（由 MonitorService 写入）──
    // 每项：{ name, description, enabled, mode, x, y, scale, transform,
    //        width, height, refreshRate, availableModes, focused, primary }
    property var monitors: []
    property string signature: ""     // 当前显示器组合签名
    property string primaryName: ""   // 当前主显示器 name
    property bool applying: false
    property string errorMsg: ""
    property int revertSecs: 0        // >0 = 回滚确认倒计时进行中

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
