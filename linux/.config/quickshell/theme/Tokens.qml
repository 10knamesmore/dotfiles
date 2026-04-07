import QtQuick
pragma Singleton

// 设计令牌 — 集中管理视觉常量，一处修改全局生效
QtObject {
    // 完全圆形
    // 深色遮罩（power menu）

    // ── 圆角 ──
    readonly property int radiusXS: 4
    // slider track、key badge
    readonly property int radiusS: 8
    // 小按钮、徽章
    readonly property int radiusMS: 10
    // search box、list item、tool button
    readonly property int radiusM: 12
    // 卡片、toggle
    readonly property int radiusL: 16
    // 面板、bar 模块
    readonly property int radiusXL: 20
    // 时钟、OSD
    readonly property int radiusFull: 999
    // ── 间距 ──
    readonly property int spaceXS: 4
    readonly property int spaceS: 8
    readonly property int spaceM: 12
    readonly property int spaceL: 16
    readonly property int spaceXL: 24
    // ── 亮色/暗色适配 ──
    readonly property color borderBase: Colors.isLight ? "#000000" : "#ffffff"

    // ── 毛玻璃面板 ──
    readonly property real panelAlpha: Colors.isLight ? 0.75 : 0.55
    // 主面板（Settings/Calendar/Notif…）
    readonly property real cardAlpha: 0.45
    // 面板内卡片（比面板更透）
    readonly property real toastAlpha: 0.65
    // Toast/OSD（需要快速阅读）
    readonly property real backdropDim: 0.2
    // 遮罩层
    readonly property real backdropMedium: 0.35
    // launcher、keybindings
    readonly property real backdropDark: 0.55
    // ── 边框 ──
    readonly property real borderAlpha: 0.12
    // 面板白色边框
    readonly property real borderHoverAlpha: 0.25
    // hover 边框
    readonly property int borderWidth: 1
    // ── 发光 ──
    readonly property real glowAlpha: 0.06
    // 面板顶部内发光

    // ── 阴影 ──
    readonly property real shadowOpacity: 0.2
    readonly property real shadowHoverOpacity: 0.3
    // ── 动画 ──
    readonly property int animFast: 150
    readonly property int animNormal: 250
    readonly property int animSlow: 400
    readonly property int animElaborate: 500 // 华丽入场
    readonly property int staggerDelay: 40 // 交错入场间隔
}
