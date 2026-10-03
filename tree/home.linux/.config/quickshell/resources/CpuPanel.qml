import "../components"
import "../state"
import "../theme"
import QtQuick

// 独立 CPU 面板；顶部由原胶囊提供，详情读取 100ms 快照。
PanelOverlay {
    id: root

    readonly property var cpu: ResourceStats.cpu
    readonly property var processRows: cpu.processes.map(process => ({
                pid: process.pid,
                name: process.name,
                value: process.usage.toFixed(1) + "%"
            }))

    panelHeight: content.implicitHeight + Tokens.spaceL * 2
    panelTargetY: 54
    panelWidth: 460
    showing: PanelState.cpuOpen

    onCloseRequested: PanelState.cpuOpen = false

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
            text: "CPU 数据不可用"
            visible: root.cpu.status === "error"
        }
        Column {
            spacing: Tokens.spaceM
            visible: root.cpu.status !== "error"
            width: parent.width

            Row {
                spacing: Tokens.spaceXL
                visible: root.cpu.frequencyGHz !== null || root.cpu.temperatureC !== null

                Text {
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    text: root.cpu.frequencyGHz === null ? "" : "频率  " + root.cpu.frequencyGHz.toFixed(1) + " GHz"
                    visible: root.cpu.frequencyGHz !== null
                }
                Text {
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    text: root.cpu.temperatureC === null ? "" : "温度  " + Math.round(root.cpu.temperatureC) + " °C"
                    visible: root.cpu.temperatureC !== null
                }
            }
            StatRow {
                label: "使用率"
                value: "0–100%"
                valueColor: Colors.subtext0
                width: parent.width
            }
            TimeSeriesChart {
                Accessible.name: "整机 CPU 使用率，最近一分钟"
                lineColor: Colors.blue
                maxValue: 100
                points: ResourceStats.cpuHistory
                width: parent.width
            }
            Divider {
                width: parent.width
            }
            ProcessTable {
                Accessible.name: "CPU 占用前八进程"
                metricLabel: "整机 CPU"
                rows: root.processRows
                width: parent.width
            }
        }
    }
}
