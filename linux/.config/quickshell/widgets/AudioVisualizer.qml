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
        // 低频 teal → blue → mauve → pink → red 五段渐变
        let stops = [
            {
                pos: 0.0,
                c: Colors.blue
            },
            {
                pos: 0.33,
                c: Colors.mauve
            },
            {
                pos: 0.66,
                c: Colors.pink
            },
            {
                pos: 1.0,
                c: Colors.red
            }
        ];
        let i = 0;
        for (let s = 1; s < stops.length; s++) {
            if (t <= stops[s].pos) {
                i = s - 1;
                break;
            }
        }
        let f = (t - stops[i].pos) / (stops[i + 1].pos - stops[i].pos);
        let a = stops[i].c, b = stops[i + 1].c;
        return Qt.rgba(a.r + (b.r - a.r) * f, a.g + (b.g - a.g) * f, a.b + (b.b - a.b) * f, 1);
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
    mask: Region {
        width: 0
        height: 0
    }

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
                height: Math.max(2, Math.pow(barValue / root.maxRange, 0.8) * 52)
                radius: 3
                color: root.barColor(index)
                opacity: 0.5
                anchors.bottom: parent.bottom
            }
        }
    }
}
