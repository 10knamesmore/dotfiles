import QtQuick
pragma Singleton

// 屏幕效果状态单例 —— ScreenEffectsService 写入、UI（ScreenEffectsPanel、bar 的
// ScreenEffectsModule）读取的中枢。仿 MonitorState ← MonitorService 的分工：
// 数据容器在此，读写文件/生成 shader/hyprctl IPC 全在 services/ScreenEffectsService.qml。
//
// bar 模块过去是 inotify 监听状态文件来更新图标 —— 可它和面板本来就在同一个
// quickshell 进程里，等于「自己写文件 → 内核通知自己 → 自己读回来」。收口到本单例后
// 直接属性绑定即可。
QtObject {
    id: root

    // ── 数据（0-100，由 ScreenEffectsService 写入）──
    property int warmth: 0        // 护眼色温，映射到 6500K..2500K
    property int grain: 0         // 胶片颗粒强度
    property int grainSize: 50    // 颗粒粗细
    property int shadowBoost: 40  // 暗部颗粒增强
    property int brightness: 100  // 屏幕背光（不进 shader，走 brightnessctl/ddcutil）

    // 只有色温和颗粒决定 shader 是否加载；颗粒大小/暗部增强只在颗粒开启时有意义。
    readonly property bool effectsActive: warmth > 0 || grain > 0

    // ── 意图信号（UI 发出 → ScreenEffectsService 接收）──
    signal applyRequested(int warmth, int grain, int grainSize, int shadowBoost)
    signal toggleRequested()
    signal brightnessRequested(int value)
    // 面板打开时回读背光实际值 —— 亮度可能被亮度键/其他工具改过，State 不是唯一真相源
    signal refreshRequested()

    function requestApply(w, g, gs, sb) {
        applyRequested(w, g, gs, sb);
    }
    // 单参数便捷入口：滑块只改一项，其余沿用当前值
    function setWarmth(v) {
        applyRequested(v, root.grain, root.grainSize, root.shadowBoost);
    }
    function setGrain(v) {
        applyRequested(root.warmth, v, root.grainSize, root.shadowBoost);
    }
    function setGrainSize(v) {
        applyRequested(root.warmth, root.grain, v, root.shadowBoost);
    }
    function setShadowBoost(v) {
        applyRequested(root.warmth, root.grain, root.grainSize, v);
    }
    function toggle() {
        toggleRequested();
    }
    function setBrightness(v) {
        brightnessRequested(v);
    }
    function refresh() {
        refreshRequested();
    }
}
