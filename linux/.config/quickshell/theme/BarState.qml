import QtQuick
pragma Singleton

// Bar 显隐状态 — 固定显示 / hover 临时唤出（按显示器）
QtObject {
    property bool barPinnedVisible: true
    property string barHoverRevealScreen: ""
    readonly property bool barVisible: barPinnedVisible

    function toggleBar() {
        barPinnedVisible = !barPinnedVisible;
        if (barPinnedVisible)
            barHoverRevealScreen = "";
    }

    function showBarForScreen(screenName) {
        if (!screenName || barPinnedVisible)
            return;

        barHoverRevealScreen = screenName;
    }

    function hideHoverBar() {
        barHoverRevealScreen = "";
    }

    function isBarVisibleForScreen(screenName) {
        return barPinnedVisible || barHoverRevealScreen === screenName;
    }
}
