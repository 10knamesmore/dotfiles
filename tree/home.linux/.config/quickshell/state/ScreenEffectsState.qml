pragma Singleton
import QtQuick

// 屏幕效果状态由 ScreenEffectsService 写入，控制中心及其顶栏胶囊读取。
// 本单例只保存数据和发出操作请求；文件读写、shader 生成与 Hyprland IPC 由服务执行。
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
    signal toggleRequested
    signal brightnessRequested(int value)
    // 面板打开时回读背光实际值 —— 亮度可能被其他工具改过，State 不是唯一真相源
    signal refreshRequested

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
