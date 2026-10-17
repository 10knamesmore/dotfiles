import "../components"
import "../state"
import "../state/StatsFormat.js" as StatsFormat
import "../theme"
import QtQuick

// 独立内存面板；容量分段与进程 RSS 都来自同一份快照。
PanelOverlay {
    id: root

    readonly property var memory: ResourceStats.memory
    readonly property var processRows: memory.processes.map(process => ({
                pid: process.pid,
                name: process.name,
                value: StatsFormat.bytes(process.rssBytes),
                fraction: process.rssBytes / memory.totalBytes
            }))

    panelHeight: content.implicitHeight + Tokens.spaceL * 2
    panelTargetY: 54
    panelWidth: 440
    showing: PanelState.memoryOpen

    onCloseRequested: PanelState.memoryOpen = false

    Column {
        id: content

        anchors.left: parent.left
        anchors.margins: Tokens.spaceL
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Tokens.spaceM

        Text {
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            text: "内存数据不可用"
            visible: root.memory.status === "error"
        }
        Column {
            spacing: Tokens.spaceM
            visible: root.memory.status !== "error" && root.memory.totalBytes > 0
            width: parent.width

            Item {
                height: usedMetric.implicitHeight
                width: parent.width

                StatMetric {
                    id: usedMetric

                    label: "已用内存"
                    unit: "/ " + StatsFormat.gib(root.memory.totalBytes) + " GiB"
                    value: StatsFormat.gib(root.memory.usedBytes)
                    valueColor: Colors.teal
                    valueSize: Fonts.display3
                }
                StatMetric {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    label: "可用"
                    unit: "GiB"
                    value: StatsFormat.gib(root.memory.availableBytes)
                    valueColor: Colors.teal
                    valueSize: Fonts.heading
                }
            }
            MemoryBreakdown {
                memory: root.memory
                width: parent.width
            }
            StatRow {
                label: "共享内存"
                value: StatsFormat.bytes(root.memory.breakdown.sharedBytes)
                width: parent.width
            }
            StatRow {
                label: "Swap"
                value: root.memory.swapTotalBytes > 0 ? StatsFormat.gib(root.memory.swapUsedBytes) + " / " + StatsFormat.gib(root.memory.swapTotalBytes) + " GiB" : "未启用"
                width: parent.width
            }
            Rectangle {
                Accessible.name: "Swap，" + StatsFormat.bytes(root.memory.swapUsedBytes) + " / " + StatsFormat.bytes(root.memory.swapTotalBytes)
                Accessible.role: Accessible.Chart
                color: Colors.overlay(0.05)
                height: 6
                radius: 3
                visible: root.memory.swapTotalBytes > 0
                width: parent.width

                Rectangle {
                    color: Colors.lavender
                    height: parent.height
                    radius: parent.radius
                    width: root.memory.swapTotalBytes > 0 ? parent.width * root.memory.swapUsedBytes / root.memory.swapTotalBytes : 0
                }
            }
            StatRow {
                label: "已用趋势"
                value: "0–" + StatsFormat.gib(root.memory.totalBytes) + " GiB"
                valueColor: Colors.subtext0
                width: parent.width
            }
            TimeSeriesChart {
                Accessible.name: "已用内存，最近一分钟"
                lineColor: Colors.teal
                maxValue: root.memory.totalBytes
                plotHeight: 65
                points: ResourceStats.memoryHistory
                width: parent.width
            }
            Divider {
                visible: root.processRows.length > 0
                width: parent.width
            }
            ProcessTable {
                Accessible.name: "内存占用前八进程"
                barColor: Colors.teal
                metricLabel: "RSS"
                rows: root.processRows
                width: parent.width
            }
        }
    }
}
