import "../../theme"
import "../components"
import QtQuick
import Quickshell.Io

BarModule {
    id: root

    property int usage: 0
    property var prevStat: null // [total, idle]
    property var prevCores: [] // [[total, idle], ...] per core
    property var corePcts: [] // per-core usage percentages
    // Waybar 彩色柱状图（8级）
    readonly property var barChars: ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]
    readonly property var barColors: ["#69ff94", "#2aa9ff", "#f8f8f2", "#f8f8f2", "#ffffa5", "#ffffa5", "#ff9977", "#dd532e"]
    // 构建彩色柱状图富文本
    property string chartHtml: {
        if (corePcts.length === 0)
            return "";

        let s = "";
        for (let i = 0; i < corePcts.length; i++) {
            let bi = barIndex(corePcts[i]);
            s += "<span style='color:" + barColors[bi] + "'>" + barChars[bi] + "</span>";
        }
        return s;
    }

    function barIndex(pct) {
        return Math.min(Math.floor(pct / 12.5), 7);
    }

    accentColor: Colors.blue
    implicitWidth: label.implicitWidth + 32
    Component.onCompleted: reader.running = true
    onClicked: Quickshell.execDetached(["plasma-systemmonitor"])

    Process {
        id: reader

        command: ["bash", "-c", "grep '^cpu' /proc/stat"]
        onExited: {
            let lines = reader.stdout.lines;
            reader.stdout.lines = [];
            let cores = [];
            for (let i = 0; i < lines.length; i++) {
                let parts = lines[i].trim().split(/\s+/);
                let nums = parts.slice(1).map(Number);
                let idle = nums[3] + nums[4];
                let total = nums.reduce((a, b) => {
                    return a + b;
                }, 0);
                if (parts[0] === "cpu") {
                    if (root.prevStat !== null) {
                        let dt = total - root.prevStat[0];
                        let di = idle - root.prevStat[1];
                        root.usage = dt > 0 ? Math.round((dt - di) / dt * 100) : 0;
                    }
                    root.prevStat = [total, idle];
                } else {
                    cores.push([total, idle]);
                }
            }
            // Per-core percentages
            if (root.prevCores.length === cores.length && cores.length > 0) {
                let pcts = [];
                for (let i = 0; i < cores.length; i++) {
                    let dt = cores[i][0] - root.prevCores[i][0];
                    let di = cores[i][1] - root.prevCores[i][1];
                    pcts.push(dt > 0 ? Math.round((dt - di) / dt * 100) : 0);
                }
                root.corePcts = pcts;
            }
            root.prevCores = cores;
        }

        stdout: SplitParser {
            property var lines: []

            onRead: (data) => {
                lines.push(data);
            }
        }

    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            reader.running = false;
            reader.running = true;
        }
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: ""
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.usage + "%"
            color: root.usage > 80 ? Colors.red : (root.usage > 50 ? Colors.yellow : Colors.text)
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

        // Per-core bar chart — 单个 Text 用富文本着色，字符紧贴无间距
        Text {
            visible: root.chartHtml !== ""
            textFormat: Text.RichText
            text: root.chartHtml
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

    }

}
