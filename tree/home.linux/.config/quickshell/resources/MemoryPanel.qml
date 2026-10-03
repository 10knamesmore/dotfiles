import "../components"
import "../state"
import "../state/StatsFormat.js" as StatsFormat
import "../theme"
import QtQuick

// 独立内存面板；容量、等待比例和进程 RSS 都来自同一份快照。
PanelOverlay {
    id: root

    readonly property var memory: ResourceStats.memory
    readonly property var processRows: memory.processes.map(process => ({
                pid: process.pid,
                name: process.name,
                value: StatsFormat.bytes(process.rssBytes)
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

            StatRow {
                label: "可用"
                value: StatsFormat.gib(root.memory.availableBytes) + " GiB"
                valueColor: Colors.mauve
                width: parent.width
            }
            StatRow {
                label: "已用内存"
                value: "0–" + StatsFormat.gib(root.memory.totalBytes) + " GiB"
                valueColor: Colors.subtext0
                width: parent.width
            }
            TimeSeriesChart {
                Accessible.name: "已用内存，最近一分钟"
                lineColor: Colors.mauve
                maxValue: root.memory.totalBytes
                points: ResourceStats.memoryHistory
                width: parent.width
            }
            Divider {
                width: parent.width
            }
            Column {
                spacing: Tokens.spaceS
                width: parent.width

                StatRow {
                    label: "Free"
                    value: StatsFormat.bytes(root.memory.breakdown.freeBytes)
                    width: parent.width
                }
                StatRow {
                    label: "Buffer"
                    value: StatsFormat.bytes(root.memory.breakdown.buffersBytes)
                    width: parent.width
                }
                StatRow {
                    label: "Cache"
                    value: StatsFormat.bytes(root.memory.breakdown.cachedBytes)
                    width: parent.width
                }
                StatRow {
                    label: "共享内存"
                    value: StatsFormat.bytes(root.memory.breakdown.sharedBytes)
                    width: parent.width
                }
            }
            Divider {
                width: parent.width
            }
            StatRow {
                label: "Swap"
                value: root.memory.swapTotalBytes > 0 ? StatsFormat.gib(root.memory.swapUsedBytes) + " / " + StatsFormat.gib(root.memory.swapTotalBytes) + " GiB" : "未启用"
                width: parent.width
            }
            StatRow {
                label: "内存等待 · 10s"
                value: root.memory.pressureSome10 === null ? "" : root.memory.pressureSome10.toFixed(1) + "%"
                valueColor: root.memory.pressureSome10 >= 10 ? Colors.yellow : Colors.text
                visible: root.memory.pressureSome10 !== null
                width: parent.width
            }
            Divider {
                visible: root.processRows.length > 0
                width: parent.width
            }
            ProcessTable {
                Accessible.name: "内存占用前八进程"
                metricLabel: "RSS"
                rows: root.processRows
                width: parent.width
            }
        }
    }
}
