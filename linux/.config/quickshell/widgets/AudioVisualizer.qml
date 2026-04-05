import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

// 桌面底部音频频谱可视化 — 纯渲染组件，数据由 CavaService 提供
// 通过 Variants 在每个显示器上各生成一个实例
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 外观参数 ──
    property int barCount: PanelState.visualizerBars.length > 0 ? PanelState.visualizerBars.length : 48
    property int maxRange: 100       // 与 CavaService 的 ascii_max_range 一致

    function barColor(index) {
        let t = index / (barCount - 1);
        // 低频 blue → 中频 mauve → 高频 pink
        if (t < 0.5) {
            let f = t * 2;
            return Qt.rgba(Colors.blue.r + (Colors.mauve.r - Colors.blue.r) * f, Colors.blue.g + (Colors.mauve.g - Colors.blue.g) * f, Colors.blue.b + (Colors.mauve.b - Colors.blue.b) * f, 1);
        } else {
            let f = (t - 0.5) * 2;
            return Qt.rgba(Colors.mauve.r + (Colors.pink.r - Colors.mauve.r) * f, Colors.mauve.g + (Colors.pink.g - Colors.mauve.g) * f, Colors.mauve.b + (Colors.pink.b - Colors.mauve.b) * f, 1);
        }
    }

    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    implicitHeight: 80
    margins.bottom: 0
    visible: PanelState.visualizerVisible
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // ── 频谱柱状图 ──
    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 8
        height: 60
        spacing: 2

        Repeater {
            model: root.barCount

            delegate: Rectangle {
                required property int index

                property real barValue: index < PanelState.visualizerBars.length ? PanelState.visualizerBars[index] : 0

                width: 6
                height: Math.max(2, barValue / root.maxRange * 52)
                radius: 3
                color: root.barColor(index)
                opacity: 0.5
                anchors.bottom: parent.bottom

                Behavior on height {
                    NumberAnimation {
                        duration: 80
                        easing.type: Easing.OutQuad
                    }
                }
            }
        }
    }
}
