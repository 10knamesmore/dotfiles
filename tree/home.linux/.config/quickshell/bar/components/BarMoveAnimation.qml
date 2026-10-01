import QtQuick

// 顶栏位置移动统一使用与 Hyprland windowsMove 相同的弹簧曲线。
// hyprland.lua 的 snappy：mass 1、stiffness 200、dampening 28.3（临界阻尼，ω₀≈14.1 rad/s）。
// Qt 的 SpringAnimation 按 16ms 步长积分，参数是每步量：spring = k·0.016 ≈ 3.2，damping = c·0.016 ≈ 0.45。
// 两套积分方式不同，轨迹不逐帧相同，但观感一致；改 hyprland.lua 的 snappy 时同步这两个值。
SpringAnimation {
    spring: 3.2
    damping: 0.45
    mass: 1

    // 位移只有几十像素，亚像素误差后即停，避免不可见的尾巴持续驱动重绘。
    epsilon: 0.25
}
