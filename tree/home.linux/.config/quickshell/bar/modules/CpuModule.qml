import "../../theme"
import "../../state"
import "../components"
import QtQuick
import Quickshell

BarModule {
    id: root

    // 数据来自 SystemStats（SystemStatsService 每秒更新）
    // Waybar 彩色柱状图（8级）
    readonly property var barChars: ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]
    readonly property var barColors: ["#69ff94", "#2aa9ff", "#f8f8f2", "#f8f8f2", "#ffffa5", "#ffffa5", "#ff9977", "#dd532e"]
    // 构建彩色柱状图富文本
    property string chartHtml: {
        // 收回时保留图表直到淡出结束，完全隐藏后不再读取各核心数据。
        if (!root.hovered && !root.hoverDetailsVisible)
            return "";
        let cores = SystemStats.cpuCorePcts;
        if (cores.length === 0)
            return "";

        let s = "";
        for (let i = 0; i < cores.length; i++) {
            let bi = barIndex(cores[i]);
            s += "<span style='color:" + barColors[bi] + "'>" + barChars[bi] + "</span>";
        }
        return s;
    }

    function barIndex(pct) {
        return Math.min(Math.floor(pct / 12.5), 7);
    }

    accentColor: Colors.blue
    implicitWidth: compactLabel.implicitWidth + 32 + (hovered && chartHtml !== "" ? coreChart.implicitWidth + label.spacing : 0)
    clickable: false

    // 紧凑模式宽度参考
    Row {
        id: compactLabel
        visible: false
        spacing: 5
        Text {
            text: cpuIcon.text
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }
        Text {
            text: SystemStats.cpuUsage + "%"
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
        }
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            id: cpuIcon
            text: ""
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: SystemStats.cpuUsage + "%"
            color: SystemStats.cpuUsage > 80 ? Colors.red : (SystemStats.cpuUsage > 50 ? Colors.yellow : Colors.text)
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: 300
                }
            }
        }

        // Per-core bar chart — hover 时展开显示
        Text {
            id: coreChart
            visible: root.chartHtml !== "" && root.hoverDetailsVisible
            width: implicitWidth * root.hoverReveal
            clip: true
            textFormat: Text.RichText
            text: root.chartHtml
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
            opacity: root.hoverReveal
        }
    }
}
