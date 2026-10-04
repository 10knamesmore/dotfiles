pragma Singleton
pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io

// 读取 dots-wallpaper 生成的深色配色；文件变化时所有消费者立即更新。
// 中性色仅用于首次生成前，状态色由 Matugen 与壁纸主色协调。
Singleton {
    id: root

    readonly property color base: palette.background
    readonly property color mantle: palette.surfaceLow
    readonly property color crust: palette.surfaceLowest
    readonly property color text: palette.text
    readonly property color subtext1: palette.textSecondary
    readonly property color subtext0: Qt.tint(base, withAlpha(root.subtext1, 0.78))
    readonly property color overlay2: palette.outline
    readonly property color overlay1: Qt.tint(base, withAlpha(root.subtext1, 0.60))
    readonly property color overlay0: palette.outlineVariant
    readonly property color surface2: palette.surfaceHighest
    readonly property color surface1: palette.surfaceHigh
    readonly property color surface0: palette.surface
    readonly property color blue: palette.primary
    readonly property color lavender: palette.secondary
    readonly property color sapphire: palette.secondary
    readonly property color sky: palette.secondary
    readonly property color teal: palette.tertiary
    readonly property color green: palette.success
    readonly property color yellow: palette.warning
    readonly property color peach: palette.warning
    readonly property color maroon: palette.error
    readonly property color red: palette.error
    readonly property color mauve: palette.primary
    readonly property color pink: palette.tertiary
    readonly property color flamingo: palette.secondary
    readonly property color rosewater: palette.secondary

    function withAlpha(color, alpha) {
        return Qt.rgba(color.r, color.g, color.b, alpha);
    }

    function overlay(alpha) {
        return withAlpha(root.text, alpha);
    }

    JsonAdapter {
        id: palette
        property string source: ""
        property string background: "#181818"
        property string surfaceLowest: "#101010"
        property string surfaceLow: "#1c1c1c"
        property string surface: "#242424"
        property string surfaceHigh: "#2b2b2b"
        property string surfaceHighest: "#333333"
        property string text: "#eeeeee"
        property string textSecondary: "#cccccc"
        property string outline: "#999999"
        property string outlineVariant: "#555555"
        property string primary: "#cccccc"
        property string secondary: "#bbbbbb"
        property string tertiary: "#dddddd"
        property string success: "#a5d6a7"
        property string warning: "#ffe082"
        property string error: "#ef9a9a"
    }

    FileView {
        path: Quickshell.env("HOME") + "/.local/state/dots/theme/colors.json"
        adapter: palette
        watchChanges: true
        blockLoading: true
        onFileChanged: reload()
        onLoaded: console.info("[theme] wallpaper palette loaded", palette.source)
        onLoadFailed: error => console.warn("[theme] palette unavailable; run dots-wallpaper", error)
    }
}
