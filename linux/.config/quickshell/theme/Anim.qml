import QtQuick
pragma Singleton

// 动画曲线常量 — 用于 easing.bezierCurve 属性
// 用法：easing.type: Easing.BezierSpline; easing.bezierCurve: Anim.decelerate
QtObject {
    // 减速（入场）: 快进慢出，元素从屏幕外滑入时使用
    readonly property var decelerate: [0.2, 0.9, 0.3, 1, 1, 1]
    // 加速（退场）: 慢进快出，元素消失时使用（比入场快，感觉更灵敏）
    readonly property var accelerate: [0.4, 0, 0.7, 0.2, 1, 1]
    // 标准：Material 3 标准曲线，通用状态变化
    readonly property var standard: [0.4, 0, 0.2, 1, 1, 1]
    // 弹性：带 overshoot 的弹跳感，hover 微缩放等微交互使用
    readonly property var elastic: [0.34, 1.56, 0.64, 1, 1, 1]
}
